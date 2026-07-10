use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

// Pure stance-measurement math, split out of `Store` so the product's core
// opinion metrics are unit-testable without a database (like sim/validate.rs).
// `Store` does the raw SELECTs and hands plain rows here.

/// One stance-bearing action (post or comment) by an agent. The reader excludes
/// NULL and 'seed' stances (the injected event is not an opinion).
pub struct StanceAct {
    pub user_id: i64,
    pub stance: String,
    pub round: i64,
    pub created_at: String,
}

/// One row of the independent-classifier pass (monoculture check).
pub struct IndependentLabel {
    pub self_stance: Option<String>,
    pub ind_stance: String,
}

/// Agent-weighted stance distribution: ONE vote per agent — its most recent
/// stance-bearing action. This is the honest population metric; post-weighted
/// counts let one hyperactive agent dominate.
///
/// Replicates the former SQL window function exactly: `ROW_NUMBER() OVER
/// (PARTITION BY user_id ORDER BY round DESC, created_at DESC)`, rn = 1.
/// Recency is (round, created_at) — created_at, NOT the row id, because
/// post_id and comment_id are independent sequences and comparing them would
/// mix two id-spaces. On an exact (round, created_at) tie the first row seen
/// wins (the SQL's pick among exact ties was likewise unspecified).
pub fn agent_distribution(acts: &[StanceAct]) -> Value {
    let mut latest: HashMap<i64, &StanceAct> = HashMap::new();
    for act in acts {
        latest
            .entry(act.user_id)
            .and_modify(|cur| {
                if (act.round, act.created_at.as_str()) > (cur.round, cur.created_at.as_str()) {
                    *cur = act;
                }
            })
            .or_insert(act);
    }
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for a in latest.values() {
        *counts.entry(a.stance.as_str()).or_insert(0) += 1;
    }
    stance_rows(counts)
}

/// How often the agents' self-reported stance matched the independent label.
/// Low agreement means the self-reports (and headline numbers) are unreliable.
///
/// Same numbers as the former SQL: `labelled`/`agree` count only rows with a
/// self-report; the confusion matrix covers ALL rows with missing self-reports
/// coalesced to "none"; both marginals are over the SAME labelled set (posts +
/// comments), so self-reported vs independent is apples-to-apples.
pub fn agreement(labels: &[IndependentLabel]) -> Value {
    let n = labels.iter().filter(|l| l.self_stance.is_some()).count() as i64;
    let agree = labels
        .iter()
        .filter(|l| l.self_stance.as_deref() == Some(l.ind_stance.as_str()))
        .count() as i64;

    let mut cells: BTreeMap<(&str, &str), i64> = BTreeMap::new();
    let mut self_counts: BTreeMap<&str, i64> = BTreeMap::new();
    let mut ind_counts: BTreeMap<&str, i64> = BTreeMap::new();
    for l in labels {
        *cells
            .entry((l.self_stance.as_deref().unwrap_or("none"), l.ind_stance.as_str()))
            .or_insert(0) += 1;
        if let Some(s) = l.self_stance.as_deref() {
            *self_counts.entry(s).or_insert(0) += 1;
        }
        *ind_counts.entry(l.ind_stance.as_str()).or_insert(0) += 1;
    }
    let mut cells: Vec<((&str, &str), i64)> = cells.into_iter().collect();
    cells.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let confusion: Vec<Value> = cells
        .into_iter()
        .map(|((s, i), count)| json!({ "self": s, "independent": i, "count": count }))
        .collect();

    json!({
        "labelled": n,
        "agree": agree,
        "agreement_rate": if n > 0 { agree as f64 / n as f64 } else { 0.0 },
        "self_distribution": stance_rows(self_counts),
        "independent_distribution": stance_rows(ind_counts),
        "confusion": confusion,
    })
}

/// `[{stance, count}]` sorted by count DESC. The SQL's `ORDER BY COUNT(*) DESC`
/// left equal-count ordering unspecified; we tiebreak by stance name so the
/// output is deterministic.
fn stance_rows(counts: BTreeMap<&str, i64>) -> Value {
    let mut rows: Vec<(&str, i64)> = counts.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    Value::Array(
        rows.into_iter()
            .map(|(stance, count)| json!({ "stance": stance, "count": count }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(user_id: i64, stance: &str, round: i64, created_at: &str) -> StanceAct {
        StanceAct { user_id, stance: stance.into(), round, created_at: created_at.into() }
    }

    fn counts_of(v: &Value) -> std::collections::HashMap<String, i64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| (r["stance"].as_str().unwrap().to_string(), r["count"].as_i64().unwrap()))
            .collect()
    }

    #[test]
    fn one_vote_per_agent_latest_action_wins() {
        // Agent 1 is hyperactive (3 support acts) but still counts once; agent 2
        // changed its mind — only the round-5 oppose counts.
        let acts = vec![
            act(1, "support", 0, "t1"),
            act(1, "support", 1, "t2"),
            act(1, "support", 2, "t3"),
            act(2, "support", 0, "t1"),
            act(2, "oppose", 5, "t9"),
        ];
        let counts = counts_of(&agent_distribution(&acts));
        assert_eq!(counts.get("support"), Some(&1));
        assert_eq!(counts.get("oppose"), Some(&1));
    }

    #[test]
    fn same_round_ties_break_on_created_at() {
        // Post-then-comment within one round: the later created_at wins, exactly
        // like the SQL's ORDER BY round DESC, created_at DESC.
        let acts = vec![
            act(1, "neutral", 2, "2026-01-01T10:00:01Z"),
            act(1, "support", 2, "2026-01-01T10:00:02Z"),
        ];
        let counts = counts_of(&agent_distribution(&acts));
        assert_eq!(counts.get("support"), Some(&1));
        assert_eq!(counts.get("neutral"), None);
    }

    #[test]
    fn distribution_sorted_by_count_desc() {
        let acts = vec![
            act(1, "oppose", 0, "t"),
            act(2, "oppose", 0, "t"),
            act(3, "support", 0, "t"),
        ];
        let dist = agent_distribution(&acts);
        assert_eq!(dist[0]["stance"], "oppose");
        assert_eq!(dist[0]["count"], 2);
        assert_eq!(dist[1]["stance"], "support");
        assert_eq!(agent_distribution(&[]), json!([]));
    }

    #[test]
    fn agreement_matrix_and_marginals() {
        let labels = vec![
            IndependentLabel { self_stance: Some("support".into()), ind_stance: "support".into() },
            IndependentLabel { self_stance: Some("support".into()), ind_stance: "oppose".into() },
            IndependentLabel { self_stance: None, ind_stance: "neutral".into() },
        ];
        let ag = agreement(&labels);
        // Only self-reported rows count toward the rate; the None row does not.
        assert_eq!(ag["labelled"], 2);
        assert_eq!(ag["agree"], 1);
        assert!((ag["agreement_rate"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        // Self marginal skips missing self-reports; independent covers all rows.
        assert_eq!(counts_of(&ag["self_distribution"]).get("support"), Some(&2));
        let ind = counts_of(&ag["independent_distribution"]);
        assert_eq!(ind.get("support"), Some(&1));
        assert_eq!(ind.get("oppose"), Some(&1));
        assert_eq!(ind.get("neutral"), Some(&1));
        // Confusion coalesces a missing self-report to "none".
        let confusion = ag["confusion"].as_array().unwrap();
        assert_eq!(confusion.len(), 3);
        assert!(confusion
            .iter()
            .any(|c| c["self"] == "none" && c["independent"] == "neutral" && c["count"] == 1));
    }

    #[test]
    fn agreement_empty_is_zero_not_nan() {
        let ag = agreement(&[]);
        assert_eq!(ag["labelled"], 0);
        assert_eq!(ag["agreement_rate"], 0.0);
        assert_eq!(ag["confusion"], json!([]));
    }
}
