//! Graph-boosted search scoring.
//!
//! Computes proximity scores for search results based on their distance
//! to anchor nodes in the graph, then applies a weighted boost to the
//! original relevance score.

use std::collections::HashMap;

use strata_core::types::BranchId;
use strata_core::StrataResult;

use super::types::{BfsOptions, Direction};
use super::GraphStore;

/// Parameters for graph-boosted search.
#[derive(Debug, Clone)]
pub struct GraphBoost {
    /// Graph to use for proximity computation.
    pub graph: String,
    /// Anchor node IDs (e.g. the node representing the current patient).
    pub anchors: Vec<String>,
    /// Maximum BFS depth for proximity (default 2).
    pub max_depth: usize,
    /// Boost weight: `boosted = score * (1.0 + weight * proximity)`.
    /// 0.0 means no boost, 0.3 means up to 30% boost for direct neighbors.
    pub weight: f64,
}

impl Default for GraphBoost {
    fn default() -> Self {
        Self {
            graph: String::new(),
            anchors: Vec::new(),
            max_depth: 2,
            weight: 0.3,
        }
    }
}

/// Compute proximity scores for entity_refs based on graph distance to anchors.
///
/// Returns a map from entity_ref URI → proximity score in [0.0, 1.0].
/// - Anchor itself: 1.0
/// - 1-hop neighbor: 0.5
/// - 2-hop neighbor: 0.25
/// - Beyond max_depth: 0.0
pub fn compute_proximity_map(
    gs: &GraphStore,
    branch_id: BranchId,
    boost: &GraphBoost,
) -> StrataResult<HashMap<String, f64>> {
    let mut proximity: HashMap<String, f64> = HashMap::new();

    for anchor in &boost.anchors {
        let result = gs.bfs(
            branch_id,
            &boost.graph,
            anchor,
            BfsOptions {
                max_depth: boost.max_depth,
                direction: Direction::Both,
                ..Default::default()
            },
        )?;

        for (node_id, depth) in &result.depths {
            let score = if *depth == 0 {
                1.0
            } else if *depth <= boost.max_depth {
                1.0 / (2.0_f64.powi(*depth as i32))
            } else {
                0.0
            };

            // Look up the node's entity_ref to map it
            if let Some(data) = gs.get_node(branch_id, &boost.graph, node_id)? {
                if let Some(uri) = data.entity_ref {
                    let existing = proximity.get(&uri).copied().unwrap_or(0.0);
                    // Use the closest anchor (maximum proximity)
                    if score > existing {
                        proximity.insert(uri, score);
                    }
                }
            }
        }
    }

    Ok(proximity)
}

/// Apply graph boost to a search result score.
///
/// `boosted = score * (1.0 + weight * proximity)`
pub fn apply_boost(score: f32, weight: f64, proximity: f64) -> f32 {
    (score as f64 * (1.0 + weight * proximity)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::graph::types::*;
    use std::sync::Arc;

    fn setup() -> (Arc<Database>, GraphStore) {
        let db = Database::cache().unwrap();
        let gs = GraphStore::new(db.clone());
        (db, gs)
    }

    fn branch() -> BranchId {
        BranchId::from_bytes([0u8; 16])
    }

    fn build_chain(gs: &GraphStore, b: BranchId) {
        // A → B → C → D, each with entity_ref
        gs.create_graph(b, "g", None).unwrap();
        for (id, uri) in &[
            ("A", "kv://main/A"),
            ("B", "kv://main/B"),
            ("C", "kv://main/C"),
            ("D", "kv://main/D"),
        ] {
            gs.add_node(
                b,
                "g",
                id,
                NodeData {
                    entity_ref: Some(uri.to_string()),
                    properties: None,
                },
            )
            .unwrap();
        }
        gs.add_edge(b, "g", "A", "B", "E", EdgeData::default())
            .unwrap();
        gs.add_edge(b, "g", "B", "C", "E", EdgeData::default())
            .unwrap();
        gs.add_edge(b, "g", "C", "D", "E", EdgeData::default())
            .unwrap();
    }

    #[test]
    fn anchor_has_proximity_1() {
        let (_db, gs) = setup();
        let b = branch();
        build_chain(&gs, b);

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "g".into(),
                anchors: vec!["A".into()],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        assert_eq!(prox["kv://main/A"], 1.0);
    }

    #[test]
    fn one_hop_neighbor_proximity_0_5() {
        let (_db, gs) = setup();
        let b = branch();
        build_chain(&gs, b);

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "g".into(),
                anchors: vec!["A".into()],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        assert!((prox["kv://main/B"] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn two_hop_neighbor_proximity_0_25() {
        let (_db, gs) = setup();
        let b = branch();
        build_chain(&gs, b);

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "g".into(),
                anchors: vec!["A".into()],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        assert!((prox["kv://main/C"] - 0.25).abs() < 1e-10);
    }

    #[test]
    fn beyond_max_depth_not_in_map() {
        let (_db, gs) = setup();
        let b = branch();
        build_chain(&gs, b);

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "g".into(),
                anchors: vec!["A".into()],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        // D is 3 hops from A
        assert!(!prox.contains_key("kv://main/D"));
    }

    #[test]
    fn weight_0_no_change() {
        let boosted = apply_boost(1.0, 0.0, 1.0);
        assert!((boosted - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weight_0_3_proximity_1() {
        let boosted = apply_boost(1.0, 0.3, 1.0);
        assert!((boosted - 1.3).abs() < 1e-6);
    }

    #[test]
    fn weight_0_3_proximity_0_5() {
        let boosted = apply_boost(1.0, 0.3, 0.5);
        assert!((boosted - 1.15).abs() < 1e-6);
    }

    #[test]
    fn empty_anchors_empty_map() {
        let (_db, gs) = setup();
        let b = branch();
        gs.create_graph(b, "g", None).unwrap();

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "g".into(),
                anchors: vec![],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        assert!(prox.is_empty());
    }

    #[test]
    fn nonexistent_graph_returns_empty_map() {
        let (_db, gs) = setup();
        let b = branch();

        let prox = compute_proximity_map(
            &gs,
            b,
            &GraphBoost {
                graph: "nonexistent".into(),
                anchors: vec!["X".into()],
                max_depth: 2,
                weight: 0.3,
            },
        )
        .unwrap();

        // Start node gets visited but has no entity_ref
        assert!(prox.is_empty());
    }
}
