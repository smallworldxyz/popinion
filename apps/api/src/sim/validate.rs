//! Honesty-test metrics. An LLM-agent opinion sim can produce confident fiction:
//! numbers that come from the model's prior, not the population. These metrics
//! quantify two failure modes so a report can disclose them:
//!
//! - **noise floor** — how much the stance distribution moves between identical
//!   runs on different seeds. Any claimed "effect" smaller than this is noise.
//! - **persona sensitivity** — distance between a run and its persona-permuted
//!   ablation. If it's near the noise floor, the personas are inert and the
//!   output is the model prior wearing masks.

use serde_json::Value;
use std::collections::BTreeMap;

/// Normalize a `Store::stance_distribution()` result (`[{stance,count,..}]`)
/// into shares that sum to 1.
pub fn stance_shares(dist: &Value) -> BTreeMap<String, f64> {
    let mut counts: BTreeMap<String, f64> = BTreeMap::new();
    let mut total = 0.0;
    for row in dist.as_array().into_iter().flatten() {
        let stance = row["stance"].as_str().unwrap_or("unknown").to_string();
        let c = row["count"].as_f64().unwrap_or(0.0);
        *counts.entry(stance).or_insert(0.0) += c;
        total += c;
    }
    if total > 0.0 {
        for v in counts.values_mut() {
            *v /= total;
        }
    }
    counts
}

/// Total-variation distance between two stance distributions: 0 (identical) to
/// 1 (disjoint). Symmetric; defined over the union of stance labels.
pub fn total_variation(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> f64 {
    let mut sum = 0.0;
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        sum += (a.get(k).copied().unwrap_or(0.0) - b.get(k).copied().unwrap_or(0.0)).abs();
    }
    sum / 2.0
}

/// Significance bar for a claimed effect: 1.5× the noise floor, or a 0.05
/// absolute default when no rerun data exists to estimate the floor. Shared by
/// /validate's ablation verdict and /compare's A/B verdict so "significant"
/// means the same thing everywhere.
pub fn noise_threshold(floor: f64) -> f64 {
    if floor > 0.0 {
        floor * 1.5
    } else {
        0.05
    }
}

/// Mean pairwise total-variation distance across runs — the noise floor. 0 when
/// fewer than two runs are supplied.
pub fn noise_floor(runs: &[BTreeMap<String, f64>]) -> f64 {
    if runs.len() < 2 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut pairs = 0usize;
    for i in 0..runs.len() {
        for j in (i + 1)..runs.len() {
            sum += total_variation(&runs[i], &runs[j]);
            pairs += 1;
        }
    }
    sum / pairs as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shares_normalize() {
        let dist = json!([{"stance": "support", "count": 3}, {"stance": "oppose", "count": 1}]);
        let s = stance_shares(&dist);
        assert!((s["support"] - 0.75).abs() < 1e-9);
        assert!((s["oppose"] - 0.25).abs() < 1e-9);
    }

    #[test]
    fn tv_identical_is_zero_disjoint_is_one() {
        let a = stance_shares(&json!([{"stance": "support", "count": 1}]));
        let b = stance_shares(&json!([{"stance": "oppose", "count": 1}]));
        assert!(total_variation(&a, &a).abs() < 1e-9);
        assert!((total_variation(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn tv_is_symmetric_and_partial() {
        let a = stance_shares(&json!([{"stance": "support", "count": 3}, {"stance": "oppose", "count": 1}]));
        let b = stance_shares(&json!([{"stance": "support", "count": 1}, {"stance": "oppose", "count": 1}]));
        let d = total_variation(&a, &b);
        assert!((d - total_variation(&b, &a)).abs() < 1e-9, "symmetric");
        assert!(d > 0.0 && d < 1.0, "partial overlap");
    }

    #[test]
    fn noise_threshold_defaults_without_rerun_data() {
        assert!((noise_threshold(0.0) - 0.05).abs() < 1e-9);
        assert!((noise_threshold(0.2) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn ab_verdict_from_hand_built_distributions() {
        // The /compare pipeline on hand-built stance distributions:
        // A 60/40 vs B 30/70 → TV = 0.3, above the no-rerun threshold (0.05).
        let a = stance_shares(&json!([{"stance": "support", "count": 6}, {"stance": "oppose", "count": 4}]));
        let b = stance_shares(&json!([{"stance": "support", "count": 3}, {"stance": "oppose", "count": 7}]));
        let tv = total_variation(&a, &b);
        assert!((tv - 0.3).abs() < 1e-9);
        assert!(tv > noise_threshold(0.0), "a 30-point swing is a real difference");

        // Identical distributions → TV 0 → within noise, never significant.
        let tv_same = total_variation(&a, &a);
        assert!(tv_same <= noise_threshold(0.0), "no swing is within noise");

        // A noisy sim raises the bar: floor 0.25 → threshold 0.375 swallows the
        // same 0.3 swing.
        assert!(tv <= noise_threshold(0.25), "noise floor can absorb the delta");
    }

    #[test]
    fn noise_floor_needs_two_runs() {
        let a = stance_shares(&json!([{"stance": "support", "count": 1}]));
        assert_eq!(noise_floor(std::slice::from_ref(&a)), 0.0);
        let b = stance_shares(&json!([{"stance": "oppose", "count": 1}]));
        assert!((noise_floor(&[a, b]) - 1.0).abs() < 1e-9);
    }
}
