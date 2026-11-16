//! Validate policy structure before any export can use it.

use super::rates::AggregateRule;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Wan,
    Intermediate,
    SubscriberGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyNode {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub kind: NodeKind,
    pub capacity: AggregateRule,
    /// The current circuit count, independent of endpoint count.
    pub direct_subscribers: usize,
}

/// Check each node and edge once. IDs remain borrowed, and traversal uses no recursion.
pub fn validate_layout(nodes: &[PolicyNode]) -> anyhow::Result<()> {
    let mut by_id = HashMap::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        ensure!(!node.id.trim().is_empty(), "node ID must not be empty");
        ensure!(
            by_id.insert(node.id.as_str(), index).is_none(),
            "duplicate node ID: {}",
            node.id
        );
        node.capacity
            .validate()
            .with_context(|| format!("node {} capacity", node.id))?;
    }

    // Linked child indexes avoid a heap allocation for each branch of a deep tree.
    let mut first_child = vec![None; nodes.len()];
    let mut next_sibling = vec![None; nodes.len()];
    let mut ready = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if let Some(parent_id) = &node.parent {
            let parent = *by_id
                .get(parent_id.as_str())
                .with_context(|| format!("node {} parent does not exist", node.id))?;
            ensure!(parent != index, "node {} cannot be its own parent", node.id);
            ensure!(
                node.kind != NodeKind::Wan,
                "WAN node {} must be a root",
                node.id
            );
            next_sibling[index] = first_child[parent];
            first_child[parent] = Some(index);
        } else {
            ensure!(
                !matches!(node.capacity, AggregateRule::Inherit),
                "node {} cannot inherit without a parent",
                node.id
            );
            ready.push((index, false));
        }
        match node.kind {
            NodeKind::Wan => ensure!(
                matches!(node.capacity, AggregateRule::Fixed(_)),
                "WAN node {} requires explicit capacity",
                node.id
            ),
            NodeKind::Intermediate => {
                ensure!(
                    node.direct_subscribers == 0,
                    "intermediate node {} requires a subscriber group",
                    node.id
                );
                ensure!(
                    !matches!(node.capacity, AggregateRule::Automatic { .. }),
                    "intermediate node {} cannot use subscriber demand",
                    node.id
                );
            }
            NodeKind::SubscriberGroup => {}
        }
    }

    for (index, node) in nodes.iter().enumerate() {
        if first_child[index].is_some() {
            ensure!(
                node.direct_subscribers == 0,
                "node {} cannot mix subscribers and group children",
                node.id
            );
            ensure!(
                node.kind != NodeKind::SubscriberGroup,
                "subscriber group {} cannot contain group children",
                node.id
            );
        }
    }

    let mut cursor = 0;
    while cursor < ready.len() {
        let (index, ancestor_wan) = ready[cursor];
        let node = &nodes[index];
        ensure!(
            !matches!(node.capacity, AggregateRule::WanFraction { .. }) || ancestor_wan,
            "node {} WAN percentage requires a WAN ancestor",
            node.id
        );
        let has_wan = ancestor_wan || node.kind == NodeKind::Wan;
        let mut child = first_child[index];
        while let Some(index) = child {
            ready.push((index, has_wan));
            child = next_sibling[index];
        }
        cursor += 1;
    }
    ensure!(
        ready.len() == nodes.len(),
        "topology contains a parent cycle"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology_manager::rates::{Fraction, RateBps};

    fn node(id: &str, parent: Option<&str>, kind: NodeKind) -> PolicyNode {
        PolicyNode {
            id: id.into(),
            name: id.into(),
            parent: parent.map(str::to_owned),
            kind,
            capacity: AggregateRule::Fixed(RateBps {
                down: 310_000_000,
                up: 100_000_000,
            }),
            direct_subscribers: 0,
        }
    }

    fn automatic() -> AggregateRule {
        AggregateRule::Automatic {
            down: Fraction {
                numerator: 4,
                denominator: 5,
            },
            up: Fraction {
                numerator: 1,
                denominator: 1,
            },
        }
    }

    fn wan_percentage() -> AggregateRule {
        AggregateRule::WanFraction {
            down: Fraction {
                numerator: 1,
                denominator: 2,
            },
            up: Fraction {
                numerator: 1,
                denominator: 2,
            },
        }
    }

    #[test]
    fn multiple_wans_and_repeated_profile_instances_are_valid() {
        let nodes = vec![
            node("wan1", None, NodeKind::Wan),
            node("profile1", Some("wan1"), NodeKind::SubscriberGroup),
            node("wan2", None, NodeKind::Wan),
            node("profile2", Some("wan2"), NodeKind::SubscriberGroup),
        ];
        validate_layout(&nodes).unwrap();
    }

    #[test]
    fn profile_only_and_empty_forests_need_no_synthetic_wan() {
        let mut group = node("profile", None, NodeKind::SubscriberGroup);
        group.capacity = automatic();
        group.direct_subscribers = 100;
        validate_layout(&[group]).unwrap();
        validate_layout(&[]).unwrap();
    }

    #[test]
    fn labels_are_not_identity_keys() {
        let first = node("id1", None, NodeKind::SubscriberGroup);
        let mut second = node("id2", None, NodeKind::SubscriberGroup);
        second.name = first.name.clone();
        validate_layout(&[first, second]).unwrap();
    }

    #[test]
    fn invalid_ids_and_missing_parents_are_rejected() {
        let first = node("id1", None, NodeKind::SubscriberGroup);
        assert!(validate_layout(&[first.clone(), first]).is_err());
        for id in ["", "   "] {
            assert!(validate_layout(&[node(id, None, NodeKind::SubscriberGroup)]).is_err());
        }
        assert!(
            validate_layout(&[node("child", Some("missing"), NodeKind::SubscriberGroup)]).is_err()
        );
    }

    #[test]
    fn self_parent_and_disconnected_cycles_are_rejected() {
        assert!(validate_layout(&[node("self", Some("self"), NodeKind::Intermediate)]).is_err());
        let nodes = vec![
            node("root", None, NodeKind::Wan),
            node("a", Some("b"), NodeKind::Intermediate),
            node("b", Some("c"), NodeKind::Intermediate),
            node("c", Some("a"), NodeKind::Intermediate),
        ];
        assert!(validate_layout(&nodes).is_err());
    }

    #[test]
    fn direct_wan_subscribers_keep_explicit_capacity() {
        let mut wan = node("wan", None, NodeKind::Wan);
        wan.direct_subscribers = 10;
        validate_layout(&[wan.clone()]).unwrap();
        for invalid in [automatic(), AggregateRule::Inherit, wan_percentage()] {
            wan.capacity = invalid;
            assert!(validate_layout(&[wan.clone()]).is_err());
        }
    }

    #[test]
    fn mixed_direct_subscribers_and_child_groups_are_rejected() {
        let mut wan = node("wan", None, NodeKind::Wan);
        wan.direct_subscribers = 1;
        assert!(
            validate_layout(&[wan, node("child", Some("wan"), NodeKind::SubscriberGroup)]).is_err()
        );
        let mut intermediate = node("parent", None, NodeKind::Intermediate);
        intermediate.direct_subscribers = 1;
        assert!(validate_layout(&[intermediate]).is_err());
    }

    #[test]
    fn subscriber_group_cannot_hide_another_group() {
        let group = node("group", None, NodeKind::SubscriberGroup);
        assert!(
            validate_layout(&[
                group,
                node("child", Some("group"), NodeKind::SubscriberGroup)
            ])
            .is_err()
        );
    }

    #[test]
    fn intermediate_modes_do_not_recompute_subscriber_demand() {
        let wan = node("wan", None, NodeKind::Wan);
        let mut intermediate = node("intermediate", Some("wan"), NodeKind::Intermediate);
        for allowed in [
            AggregateRule::Inherit,
            intermediate.capacity,
            wan_percentage(),
        ] {
            intermediate.capacity = allowed;
            validate_layout(&[wan.clone(), intermediate.clone()]).unwrap();
        }
        intermediate.capacity = automatic();
        assert!(validate_layout(&[wan, intermediate]).is_err());
    }

    #[test]
    fn wan_percentages_require_a_wan_in_the_actual_ancestry() {
        let unrelated = node("wan", None, NodeKind::Wan);
        let mut group = node("group", None, NodeKind::SubscriberGroup);
        group.capacity = wan_percentage();
        assert!(validate_layout(&[unrelated.clone(), group.clone()]).is_err());
        group.parent = Some("intermediate".into());
        let intermediate = node("intermediate", Some("wan"), NodeKind::Intermediate);
        validate_layout(&[group, intermediate, unrelated]).unwrap();
    }

    #[test]
    fn nested_wans_and_root_inheritance_are_rejected() {
        assert!(
            validate_layout(&[
                node("wan1", None, NodeKind::Wan),
                node("wan2", Some("wan1"), NodeKind::Wan),
            ])
            .is_err()
        );
        let mut root = node("root", None, NodeKind::Intermediate);
        root.capacity = AggregateRule::Inherit;
        assert!(validate_layout(&[root]).is_err());
    }

    #[test]
    fn empty_manual_nodes_still_require_valid_capacity() {
        let mut empty = node("wan", None, NodeKind::Wan);
        empty.capacity = AggregateRule::Fixed(RateBps { down: 0, up: 1 });
        assert!(validate_layout(&[empty]).is_err());
        let mut group = node("profile", None, NodeKind::SubscriberGroup);
        group.capacity = AggregateRule::Automatic {
            down: Fraction {
                numerator: 1,
                denominator: 0,
            },
            up: Fraction {
                numerator: 1,
                denominator: 1,
            },
        };
        assert!(validate_layout(&[group]).is_err());
    }

    #[test]
    fn deep_tree_uses_bounded_iterative_validation() {
        let mut nodes = Vec::with_capacity(50_001);
        nodes.push(node("0", None, NodeKind::Wan));
        for i in 1..=50_000 {
            let mut child = node(
                &i.to_string(),
                Some(&(i - 1).to_string()),
                NodeKind::Intermediate,
            );
            child.capacity = wan_percentage();
            nodes.push(child);
        }
        nodes.reverse();
        validate_layout(&nodes).unwrap();
    }
}
