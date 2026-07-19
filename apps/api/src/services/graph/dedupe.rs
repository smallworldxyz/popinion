//! Alias merging over a freshly built graph.
//!
//! Extraction runs per chunk and matches entities by exact name, so one real
//! actor arrives under every name the source used for it: "Electricite du
//! Cambodge" and "EDC", "Meridian Studios" and "Meridian". Each becomes its own
//! persona, so a single constituency votes two or three times and the stance
//! counts measure the source's wording rather than the population.
//!
//! Candidates are generated deterministically and only ever CONFIRMED by the
//! model — it never proposes a merge of its own. That keeps the pass bounded,
//! auditable, and unable to collapse the graph on one bad reply. There is no
//! embedding capability in this codebase (the model picker excludes embedding
//! models), so string structure plus a judgement call is what is available.

use anyhow::Result;
use std::collections::{BTreeSet, HashMap};

use super::db::{self, GraphDb, GraphData, GraphNode};
use crate::llm::{Llm, Msg};

/// Upper bound on pairs sent for confirmation. A graph with hundreds of
/// near-identical names is a different problem; truncating silently would hide
/// it, so the caller logs what was dropped.
const MAX_CANDIDATES: usize = 60;

const STOPWORDS: [&str; 12] =
    ["the", "of", "a", "an", "and", "for", "in", "on", "to", "at", "by", "s"];

fn content_words(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .filter(|w| !STOPWORDS.contains(&w.as_str()))
        .collect()
}

fn normalized(name: &str) -> String {
    content_words(name).join(" ")
}

/// Initials of a name's content words: "Electricite du Cambodge" -> "ec".
/// Deliberately not applied to single-word names, which have no initialism.
fn initials(name: &str) -> Option<String> {
    let words = content_words(name);
    if words.len() < 2 {
        return None;
    }
    Some(words.iter().filter_map(|w| w.chars().next()).collect())
}

fn types_overlap(a: &GraphNode, b: &GraphNode) -> bool {
    a.entity_types.iter().any(|t| b.entity_types.contains(t))
}

/// Deterministic candidate pairs, by name. Tight rules may cross entity type
/// because chunks often type the same actor differently; the loose one may not.
pub fn candidates(nodes: &[GraphNode]) -> Vec<(String, String)> {
    let mut out = BTreeSet::new();
    for (i, a) in nodes.iter().enumerate() {
        for b in nodes.iter().skip(i + 1) {
            let (na, nb) = (normalized(&a.name), normalized(&b.name));
            if na.is_empty() || nb.is_empty() {
                continue;
            }

            // 1. Same name once punctuation, case and stopwords are removed.
            let same_normalized = na == nb;

            // 2. One side is the other's initialism. Requires the short side to
            //    actually look like an acronym, or "Kaya Lindqvist" would match
            //    any two-word name starting K, L.
            let acronym = [(a, b), (b, a)].iter().any(|(long, short)| {
                let s = normalized(&short.name);
                !s.contains(' ')
                    && s.len() >= 2
                    && s.len() <= 6
                    && initials(&long.name).is_some_and(|ini| ini == s)
            });

            // 3. One name's words contain the other's, within the same type.
            //    "Rooftop solar owners" vs "Medium and large rooftop solar owners".
            let (wa, wb): (BTreeSet<_>, BTreeSet<_>) =
                (content_words(&a.name).into_iter().collect(), content_words(&b.name).into_iter().collect());
            let contained = types_overlap(a, b)
                && !wa.is_empty()
                && !wb.is_empty()
                && (wa.is_subset(&wb) || wb.is_subset(&wa));

            if same_normalized || acronym || contained {
                out.insert((a.name.clone(), b.name.clone()));
            }
        }
    }
    out.into_iter().collect()
}

/// Ask the model which candidate pairs name the same real-world entity.
///
/// Unavoidably a model call: the hard cases are judgements about referents, not
/// strings. "Cambodia" and "Cambodian government" are containment-adjacent and
/// arguably one; "Council for the Development of Cambodia" and "Cambodian
/// government" are two institutions and must stay apart. No string metric
/// separates those; names plus summaries and a reader do.
async fn confirm(
    llm: &Llm,
    nodes: &[GraphNode],
    pairs: &[(String, String)],
) -> Vec<(String, String)> {
    let by_name: HashMap<&str, &GraphNode> = nodes.iter().map(|n| (n.name.as_str(), n)).collect();
    let describe = |name: &str| {
        by_name
            .get(name)
            .map(|n| {
                let s = n.summary.chars().take(200).collect::<String>();
                format!("{} [{}]: {}", n.name, n.entity_types.join("/"), if s.is_empty() { "(no summary)" } else { &s })
            })
            .unwrap_or_else(|| name.to_string())
    };
    let listing = pairs
        .iter()
        .enumerate()
        .map(|(i, (a, b))| format!("{i}.\n  A = {}\n  B = {}", describe(a), describe(b)))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Each numbered pair below is two entities from one knowledge graph that MIGHT be the same \
         real-world entity under different names.\n\n{listing}\n\n\
         For each pair answer whether A and B are the SAME entity.\n\
         Answer yes only when one name is plainly another way of writing the other: an \
         abbreviation, an acronym, a shortened form, or the same group described at different \
         length.\n\
         Answer no when they are genuinely different things, even if closely related — a parent \
         organisation and its subsidiary, a regulator and the body it regulates, a person and \
         their employer, a whole and one of its parts, two sub-groups of a larger population.\n\
         When uncertain, answer no: leaving two entities apart is a smaller error than fusing two \
         real ones.\n\n\
         Reply with JSON only: {{\"0\": true, \"1\": false, ...}} keyed by the pair number."
    );

    let Ok(v) = llm.chat_json(&[Msg::user(prompt)], 0.0, 800).await else {
        tracing::warn!("dedupe: confirmation call failed, merging nothing");
        return vec![];
    };
    pairs
        .iter()
        .enumerate()
        .filter(|(i, _)| v.get(i.to_string()).and_then(|x| x.as_bool()).unwrap_or(false))
        .map(|(_, p)| p.clone())
        .collect()
}

/// Merge confirmed aliases in `graph_id`. Returns the merges applied as
/// (kept, absorbed) so the caller can show its work.
pub async fn run(db: &GraphDb, graph_id: &str, llm: &Llm) -> Result<Vec<(String, String)>> {
    let data: GraphData = db::get_graph_data(db, graph_id).await?;
    let mut pairs = candidates(&data.nodes);
    if pairs.is_empty() {
        return Ok(vec![]);
    }
    if pairs.len() > MAX_CANDIDATES {
        tracing::warn!(
            "dedupe: {} candidate pairs, confirming only the first {MAX_CANDIDATES}",
            pairs.len()
        );
        pairs.truncate(MAX_CANDIDATES);
    }

    // Weight by how much the graph knows about each name, so the better-attested
    // spelling survives and the sparse one folds into it.
    let mut degree: HashMap<&str, usize> = HashMap::new();
    for e in &data.edges {
        *degree.entry(e.source_node_name.as_str()).or_default() += 1;
        *degree.entry(e.target_node_name.as_str()).or_default() += 1;
    }

    let confirmed = confirm(llm, &data.nodes, &pairs).await;
    let mut applied = Vec::new();
    // A name already absorbed cannot also be a merge target, or a chain of
    // aliases would rewrite edges onto a node that no longer exists.
    let mut gone: BTreeSet<String> = BTreeSet::new();
    for (a, b) in confirmed {
        if gone.contains(&a) || gone.contains(&b) {
            continue;
        }
        let (keep, drop) = if (degree.get(a.as_str()), a.len()) >= (degree.get(b.as_str()), b.len()) {
            (a, b)
        } else {
            (b, a)
        };
        db::merge_nodes(db, graph_id, &keep, &drop).await?;
        gone.insert(drop.clone());
        applied.push((keep, drop));
    }
    if !applied.is_empty() {
        tracing::info!("dedupe: merged {} alias entities in {graph_id}", applied.len());
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn n(name: &str, etype: &str) -> GraphNode {
        GraphNode {
            name: name.into(),
            entity_types: vec![etype.into()],
            ..Default::default()
        }
    }

    #[test]
    fn acronyms_and_restatements_become_candidates() {
        let nodes = vec![
            n("Electricite du Cambodge", "Utility"),
            n("EDC", "Organization"),
            n("Rooftop solar owners", "Group"),
            n("Medium and large rooftop solar owners", "Group"),
            n("Meridian Studios", "Company"),
            n("Meridian", "Company"),
        ];
        let c = candidates(&nodes);
        assert!(c.contains(&("Electricite du Cambodge".into(), "EDC".into())), "acronym across types");
        assert!(
            c.contains(&("Rooftop solar owners".into(), "Medium and large rooftop solar owners".into())),
            "one name's words contain the other's"
        );
        assert!(c.contains(&("Meridian Studios".into(), "Meridian".into())), "shortened form");
    }

    #[test]
    fn unrelated_entities_are_not_candidates() {
        let nodes = vec![
            n("Council for the Development of Cambodia", "GovernmentAgency"),
            n("Ministry of Mines and Energy", "GovernmentAgency"),
            n("Kaya Lindqvist", "Person"),
            n("Kevin Lam", "Person"),
            n("Northwind Games", "Company"),
            n("Meridian Studios", "Company"),
        ];
        assert!(candidates(&nodes).is_empty(), "no shared words, no initialism, no containment");
    }

    /// Two people whose initials collide must not merge on shape alone, and a
    /// single-word name has no initialism to match.
    #[test]
    fn initials_do_not_match_arbitrary_two_word_names() {
        let nodes = vec![n("Kaya Lindqvist", "Person"), n("Karl Lindberg", "Person")];
        assert!(candidates(&nodes).is_empty());
    }

    /// Containment is the loose rule, so it stays inside one entity type — a
    /// person and the company they founded share words but are not one thing.
    #[test]
    fn containment_does_not_cross_entity_types() {
        let nodes = vec![n("Meridian Studios", "Company"), n("Meridian Studios founder", "Person")];
        assert!(candidates(&nodes).is_empty());
    }

    /// The merge itself, against a real store: edges must follow the survivor,
    /// the alias must vanish, and neither a duplicate edge nor a self-loop may
    /// be left behind by the rewiring.
    #[tokio::test]
    async fn merging_rewires_edges_and_leaves_no_self_loop() {
        let dir =
            std::env::temp_dir().join(format!("popinion-dedupe-{}", uuid::Uuid::new_v4().simple()));
        let graph = GraphDb::open(&dir.join("g.db")).unwrap();
        let gid = db::create_graph(&graph, "merge test").await.unwrap();
        let attrs = serde_json::Map::new();

        for (name, etype) in
            [("Electricite du Cambodge", "Utility"), ("EDC", "Organization"), ("Households", "Group")]
        {
            db::upsert_entity(&graph, &gid, name, &[etype.to_string()], &attrs).await.unwrap();
        }
        // Both spellings support the same thing: one edge must survive, not two.
        db::upsert_relationship(&graph, &gid, "Electricite du Cambodge", "Households", "SUPPLIES", &attrs)
            .await
            .unwrap();
        db::upsert_relationship(&graph, &gid, "EDC", "Households", "SUPPLIES", &attrs).await.unwrap();
        // An edge between the two names would become a self-loop.
        db::upsert_relationship(&graph, &gid, "EDC", "Electricite du Cambodge", "SUPPORTS", &attrs)
            .await
            .unwrap();
        // And one only the alias had, which must be carried over.
        db::upsert_relationship(&graph, &gid, "EDC", "Households", "BILLS", &attrs).await.unwrap();

        db::merge_nodes(&graph, &gid, "Electricite du Cambodge", "EDC").await.unwrap();
        let data = db::get_graph_data(&graph, &gid).await.unwrap();

        let names: Vec<&str> = data.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(!names.contains(&"EDC"), "the alias is gone");
        assert!(names.contains(&"Electricite du Cambodge"), "the survivor remains");
        assert_eq!(data.nodes.len(), 2);

        let survivor = data.nodes.iter().find(|n| n.name == "Electricite du Cambodge").unwrap();
        assert_eq!(survivor.attributes["aliases"], json!(["EDC"]), "the merge is recorded");
        assert!(survivor.entity_types.contains(&"Organization".to_string()), "types union");

        assert!(
            data.edges.iter().all(|e| e.source_node_name != "EDC" && e.target_node_name != "EDC"),
            "no edge still points at the alias"
        );
        assert!(
            data.edges.iter().all(|e| e.source_node_name != e.target_node_name),
            "the alias-to-survivor edge did not become a self-loop"
        );
        assert_eq!(
            data.edges.iter().filter(|e| e.relation_type == "SUPPLIES").count(),
            1,
            "the two SUPPLIES edges collapsed into one"
        );
        assert_eq!(
            data.edges.iter().filter(|e| e.relation_type == "BILLS").count(),
            1,
            "the alias's own edge was carried over"
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
