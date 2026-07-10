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

/// Stance on a support(+1)/neutral(0)/oppose(−1) axis; unknown labels carry no
/// score (they still appear in the distributions).
fn score(stance: &str) -> Option<f64> {
    match stance {
        "support" => Some(1.0),
        "neutral" => Some(0.0),
        "oppose" => Some(-1.0),
        _ => None,
    }
}

/// Exposed-vs-unexposed stance comparison for one seed/injected post.
///
/// `first_exposure` maps agent id → the first round the post appeared in that
/// agent's served feed. Exposed agents are scored by their latest stance act AT
/// OR AFTER that round (an act made with the post in view); their pre-exposure
/// acts are evidence for neither group. Unexposed agents are scored by their
/// latest act overall. `shift` = exposed mean − unexposed mean on the
/// support(+1)…oppose(−1) axis; null when either group has no scored agent.
/// Observational, not causal — exposure follows network position, not random
/// assignment.
pub fn exposure_shift(acts: &[StanceAct], first_exposure: &HashMap<i64, i64>) -> Value {
    let mut exposed: HashMap<i64, &StanceAct> = HashMap::new();
    let mut unexposed: HashMap<i64, &StanceAct> = HashMap::new();
    for act in acts {
        let bucket = match first_exposure.get(&act.user_id) {
            Some(&r) if act.round >= r => &mut exposed,
            Some(_) => continue,
            None => &mut unexposed,
        };
        bucket
            .entry(act.user_id)
            .and_modify(|cur| {
                if (act.round, act.created_at.as_str()) > (cur.round, cur.created_at.as_str()) {
                    *cur = act;
                }
            })
            .or_insert(act);
    }
    let side = |group: &HashMap<i64, &StanceAct>| {
        let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
        let mut scores: Vec<f64> = Vec::new();
        for a in group.values() {
            *counts.entry(a.stance.as_str()).or_insert(0) += 1;
            scores.extend(score(&a.stance));
        }
        let mean =
            (!scores.is_empty()).then(|| scores.iter().sum::<f64>() / scores.len() as f64);
        (json!({ "agents": group.len(), "mean": mean, "distribution": stance_rows(counts) }), mean)
    };
    let (exposed_v, exposed_mean) = side(&exposed);
    let (unexposed_v, unexposed_mean) = side(&unexposed);
    let shift = match (exposed_mean, unexposed_mean) {
        (Some(e), Some(u)) => Some(e - u),
        _ => None,
    };
    json!({ "exposed_stance": exposed_v, "unexposed_stance": unexposed_v, "shift": shift })
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
    fn exposure_shift_compares_groups_with_known_shift() {
        // Agents 1 and 2 were served the post at round 2. Agent 1's pre-exposure
        // oppose is evidence for neither group; post-exposure it says support.
        // Agent 2 stays neutral. Unexposed agents 3 and 4 both oppose.
        let acts = vec![
            act(1, "oppose", 0, "t0"),
            act(1, "support", 3, "t3"),
            act(2, "neutral", 2, "t2"),
            act(3, "oppose", 3, "t1"),
            act(4, "oppose", 0, "t0"),
        ];
        let first: HashMap<i64, i64> = HashMap::from([(1, 2), (2, 2)]);
        let v = exposure_shift(&acts, &first);
        // exposed mean = (1 + 0) / 2 = 0.5; unexposed mean = -1; shift = +1.5
        assert_eq!(v["exposed_stance"]["agents"], 2);
        assert!((v["exposed_stance"]["mean"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(v["unexposed_stance"]["agents"], 2);
        assert!((v["unexposed_stance"]["mean"].as_f64().unwrap() + 1.0).abs() < 1e-9);
        assert!((v["shift"].as_f64().unwrap() - 1.5).abs() < 1e-9);
        assert_eq!(counts_of(&v["exposed_stance"]["distribution"]).get("support"), Some(&1));
        assert_eq!(counts_of(&v["unexposed_stance"]["distribution"]).get("oppose"), Some(&2));
    }

    #[test]
    fn exposure_shift_empty_groups_yield_null_not_nan() {
        // The only exposed agent spoke before exposure only: neither group scores.
        let acts = vec![act(1, "support", 1, "t1")];
        let first: HashMap<i64, i64> = HashMap::from([(1, 5)]);
        let v = exposure_shift(&acts, &first);
        assert_eq!(v["exposed_stance"]["agents"], 0);
        assert_eq!(v["exposed_stance"]["mean"], Value::Null);
        assert_eq!(v["shift"], Value::Null);
        // No exposures at all: everyone is unexposed, shift still null.
        let v2 = exposure_shift(&acts, &HashMap::new());
        assert_eq!(v2["unexposed_stance"]["agents"], 1);
        assert_eq!(v2["shift"], Value::Null);
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
