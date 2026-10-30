//! Validated, atomic edits of the exported hierarchy.
use crate::{
    dynamic::{Replacement, Target},
    model::{self, Node, Settings},
};
use anyhow::{Context, Result, ensure};
#[derive(Clone)]
pub enum Operation {
    Add(Node),
    Reparent {
        id: String,
        parent: String,
    },
    Remove {
        id: String,
        replacement: Option<String>,
        child_parent: Option<String>,
    },
}
pub struct EditPlan {
    pub settings: Settings,
    pub revision: String,
    pub affected_destinations: usize,
    pub affected_pins: usize,
    pub affected_children: usize,
}
pub fn revision(settings: &Settings) -> Result<String> {
    Ok(uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &serde_json::to_vec(settings)?).to_string())
}
pub fn preview(settings: &Settings, operation: Operation) -> Result<EditPlan> {
    ensure!(
        settings.dynamic.is_some(),
        "Enable independent topology before editing independent roots"
    );
    let mut next = settings.clone();
    let mut destinations = 0;
    let mut pins = 0;
    let mut children = 0;
    match operation {
        Operation::Add(n) => {
            ensure!(
                !next.nodes.iter().any(|x| x.id == n.id),
                "Node ID already exists"
            );
            next.nodes.push(n);
        }
        Operation::Reparent { id, parent } => {
            let n = next
                .nodes
                .iter_mut()
                .find(|n| n.id == id)
                .context("Node no longer exists")?;
            n.parent = parent;
        }
        Operation::Remove {
            id,
            replacement,
            child_parent,
        } => {
            ensure!(
                next.nodes.iter().any(|n| n.id == id),
                "Node no longer exists"
            );
            if let Some(to) = &replacement {
                ensure!(
                    to != &id && next.nodes.iter().any(|n| &n.id == to),
                    "Choose a different existing replacement node"
                );
            }
            children = next.nodes.iter().filter(|n| n.parent == id).count();
            if children > 0 {
                let parent = child_parent
                    .context("Choose a parent or independent roots for dependent children")?;
                ensure!(parent != id, "Child destination cannot be the removed node");
                for n in &mut next.nodes {
                    if n.parent == id {
                        n.parent = parent.clone();
                    }
                }
            }
            let c = next.dynamic.as_mut().unwrap();
            for p in &mut c.policies {
                let affected: Vec<_> = p.targets.iter().filter(|t| t.node == id).cloned().collect();
                destinations += affected.len();
                for old in affected {
                    let to = Target {
                        node: replacement
                            .clone()
                            .context("Choose a replacement for referenced destinations")?,
                        wan: old.wan.clone(),
                    };
                    for target in &mut p.targets {
                        if target == &old {
                            *target = to.clone();
                        }
                    }
                    if p.default == old {
                        p.default = to.clone();
                    }
                    for pin in c.pins.values_mut() {
                        if pin.policy == p.id && pin.target == old {
                            pin.target = to.clone();
                            pins += 1;
                        }
                    }
                    c.replacements
                        .retain(|r| !(r.policy == p.id && r.from == old));
                    c.replacements.push(Replacement {
                        policy: p.id.clone(),
                        from: old,
                        to,
                    });
                }
                p.targets.sort();
                p.targets.dedup();
            }
            next.nodes.retain(|n| n.id != id);
        }
    }
    model::validate(&next)?;
    crate::web::nodelimits(&next)?;
    Ok(EditPlan {
        settings: next,
        revision: revision(settings)?,
        affected_destinations: destinations,
        affected_pins: pins,
        affected_children: children,
    })
}
