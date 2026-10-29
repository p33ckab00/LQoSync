use crate::model::{Plan, Settings, Subscriber, valid_id, validate};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

fn durable_directory(path: &Path) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    let parent = path.parent().context("Directory has no parent")?;
    durable_directory(parent)?;
    fs::create_dir(path)?;
    File::open(path)?.sync_all()?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn atomic_write(path: &Path, text: &str) -> Result<()> {
    let parent = path.parent().context("Missing parent directory")?;
    durable_directory(parent)?;
    let tmp = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn save_settings(root: &Path, settings: &Settings) -> Result<()> {
    validate(settings)?;
    atomic_write(
        &root.join("settings.toml"),
        &toml::to_string_pretty(settings)?,
    )
}
pub fn load_settings(root: &Path) -> Result<Settings> {
    let path = root.join("settings.toml");
    if !path.exists() {
        let s = Settings::default();
        save_settings(root, &s)?;
        return Ok(s);
    }
    let s: Settings = toml::from_str(&fs::read_to_string(path)?)?;
    validate(&s)?;
    Ok(s)
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(default)]
    pub subscribers: Vec<Subscriber>,
}
fn read_current(root: &Path) -> Result<Option<Current>> {
    let path = root.join("state/current.toml");
    if !path.exists() {
        return Ok(None);
    }
    let current: Current = toml::from_str(&fs::read_to_string(path)?)?;
    ensure!(valid_id(&current.generation), "Invalid generation pointer");
    ensure!(
        (!current.assignments || current.managed)
            && (!current.core_plan || (current.managed && current.assignments)),
        "Inconsistent generation metadata flags"
    );
    Ok(Some(current))
}
pub fn is_managed(root: &Path) -> Result<bool> {
    Ok(read_current(root)?.is_some_and(|c| c.managed))
}
pub fn load_snapshot(root: &Path) -> Result<Vec<Subscriber>> {
    let current = read_current(root)?;
    let path = if let Some(c) = current.filter(|c| c.managed) {
        root.join("staging")
            .join(c.generation)
            .join("subscribers.toml")
    } else {
        root.join("state/subscribers.toml")
    };
    if !path.exists() {
        ensure!(
            !is_managed(root)?,
            "Committed subscriber snapshot is missing"
        );
        return Ok(vec![]);
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(path)?)?;
    Ok(snapshot.subscribers)
}

#[derive(Serialize, Deserialize)]
struct Current {
    generation: String,
    status: String,
    #[serde(default)]
    managed: bool,
    #[serde(default)]
    assignments: bool,
    #[serde(default)]
    core_plan: bool,
}
pub fn current_generation(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join("state/current.toml")).ok()?;
    let current: Current = toml::from_str(&text).ok()?;
    valid_id(&current.generation).then_some(current.generation)
}
pub fn publish(root: &Path, plan: &Plan) -> Result<String> {
    ensure!(
        !is_managed(root)?,
        "Managed generations require a complete normalized snapshot"
    );
    if let Some(id) = current_generation(root) {
        let p = root.join("staging").join(&id);
        if fs::read_to_string(p.join("ShapedDevices.csv"))
            .ok()
            .as_deref()
            == Some(&plan.csv)
            && fs::read_to_string(p.join("network.json")).ok().as_deref() == Some(&plan.network)
        {
            return Ok(id);
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let p = root.join("staging").join(&id);
    durable_directory(&p)?;
    atomic_write(&p.join("ShapedDevices.csv"), &plan.csv)?;
    atomic_write(&p.join("network.json"), &plan.network)?;
    atomic_write(
        &root.join("state/current.toml"),
        &toml::to_string(&Current {
            generation: id.clone(),
            status: "staged_local_only".into(),
            managed: false,
            assignments: false,
            core_plan: false,
        })?,
    )?;
    Ok(id)
}

pub struct Publication {
    pub generation: String,
    pub mirror_warning: Option<String>,
}
pub fn publish_normalized(
    root: &Path,
    plan: &Plan,
    subscribers: &[Subscriber],
) -> Result<Publication> {
    publish_bundle(root, plan, subscribers, None, None)
}
fn publish_bundle(
    root: &Path,
    plan: &Plan,
    subscribers: &[Subscriber],
    state: Option<&crate::assignment::AssignmentState>,
    core: Option<&crate::core_plan::Plan>,
) -> Result<Publication> {
    #[derive(Serialize)]
    struct SnapshotRef<'a> {
        subscribers: &'a [Subscriber],
    }
    let snapshot = toml::to_string_pretty(&SnapshotRef { subscribers })?;
    let current = read_current(root)?;
    ensure!(
        state.is_some() || !current.as_ref().is_some_and(|c| c.assignments),
        "Assignment-aware generation requires assignment-aware publication"
    );
    let assignments = state
        .map(|state| {
            validate_assignment_snapshot(subscribers, state)?;
            toml::to_string_pretty(state).map_err(anyhow::Error::from)
        })
        .transpose()?;
    let core_text = core
        .map(|core| {
            ensure!(
                state.is_some(),
                "Core plan requires assignment-aware publication"
            );
            crate::core_plan::validate_snapshot(core, subscribers)?;
            // Compact derived output reduces repeated disk writes for large subscriber sets.
            Ok::<_, anyhow::Error>(serde_json::to_string(core)?)
        })
        .transpose()?;
    let reusable = current
        .filter(|c| c.managed && c.assignments == state.is_some() && c.core_plan == core.is_some())
        .filter(|c| {
            let p = root.join("staging").join(&c.generation);
            fs::read_to_string(p.join("ShapedDevices.csv"))
                .ok()
                .as_deref()
                == Some(&plan.csv)
                && fs::read_to_string(p.join("network.json")).ok().as_deref() == Some(&plan.network)
                && assignments.as_ref().is_none_or(|text| {
                    fs::read_to_string(p.join("assignments.toml"))
                        .ok()
                        .as_deref()
                        == Some(text)
                })
                && core_text.as_ref().is_none_or(|text| {
                    fs::read_to_string(p.join("core-plan.json")).ok().as_deref() == Some(text)
                })
                && fs::read_to_string(p.join("subscribers.toml"))
                    .ok()
                    .as_deref()
                    == Some(&snapshot)
        });
    let generation = if let Some(c) = reusable {
        if let Some(core) = core {
            crate::core_plan::validate_fresh(core, crate::orchestrator::now())?;
        }
        c.generation
    } else {
        let generation = uuid::Uuid::new_v4().to_string();
        let p = root.join("staging").join(&generation);
        durable_directory(&p)?;
        atomic_write(&p.join("subscribers.toml"), &snapshot)?;
        if let Some(text) = &assignments {
            atomic_write(&p.join("assignments.toml"), text)?;
        }
        if let Some(text) = &core_text {
            atomic_write(&p.join("core-plan.json"), text)?;
        }
        atomic_write(&p.join("ShapedDevices.csv"), &plan.csv)?;
        atomic_write(&p.join("network.json"), &plan.network)?;
        // Disk work can outlive source freshness even when the core read is newer.
        if let Some(core) = core {
            crate::core_plan::validate_fresh(core, crate::orchestrator::now())?;
        }
        atomic_write(
            &root.join("state/current.toml"),
            &toml::to_string(&Current {
                generation: generation.clone(),
                status: "staged_local_only".into(),
                managed: true,
                assignments: state.is_some(),
                core_plan: core.is_some(),
            })?,
        )?;
        generation
    };
    // The pointer is authoritative. This compatibility mirror cannot undo a committed generation.
    let mirror_warning = if fs::read_to_string(root.join("state/subscribers.toml"))
        .ok()
        .as_deref()
        == Some(&snapshot)
    {
        None
    } else {
        atomic_write(&root.join("state/subscribers.toml"), &snapshot)
            .err()
            .map(|e| format!("Snapshot mirror could not be updated: {e}"))
    };
    Ok(Publication {
        generation,
        mirror_warning,
    })
}

fn validate_assignment_snapshot(
    subscribers: &[Subscriber],
    state: &crate::assignment::AssignmentState,
) -> Result<()> {
    for (id, pin) in &state.assignments {
        ensure!(
            !id.is_empty() && valid_id(&pin.source_node) && valid_id(&pin.node),
            "Invalid stored assignment identity or node"
        );
    }
    let mut seen = std::collections::HashSet::new();
    for subscriber in subscribers {
        ensure!(
            seen.insert(&subscriber.id),
            "Duplicate subscriber ID in committed snapshot"
        );
        let record = state
            .assignments
            .get(&subscriber.id)
            .context("Committed subscriber assignment is missing")?;
        ensure!(
            record.node == subscriber.node && !record.source_node.is_empty(),
            "Committed assignment does not match subscriber parent"
        );
    }
    Ok(())
}
pub fn load_assignments(root: &Path) -> Result<crate::assignment::AssignmentState> {
    Ok(load_assignment_snapshot(root)?.1)
}
pub fn load_assignment_snapshot(
    root: &Path,
) -> Result<(Vec<Subscriber>, crate::assignment::AssignmentState)> {
    let Some(current) = read_current(root)?.filter(|c| c.managed) else {
        // Use this pointer decision for the whole read, even if a publisher switches
        // to a managed generation concurrently.
        let path = root.join("state/subscribers.toml");
        let subscribers = if path.exists() {
            toml::from_str::<Snapshot>(&fs::read_to_string(path)?)?.subscribers
        } else {
            vec![]
        };
        return Ok((subscribers, Default::default()));
    };
    let (subscribers, state, _) = load_managed(root, &current)?;
    Ok((subscribers, state))
}
fn load_managed(
    root: &Path,
    current: &Current,
) -> Result<(
    Vec<Subscriber>,
    crate::assignment::AssignmentState,
    Option<crate::core_plan::Plan>,
)> {
    let path = root.join("staging").join(&current.generation);
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(path.join("subscribers.toml"))?)?;
    let state = if current.assignments {
        let state = toml::from_str(&fs::read_to_string(path.join("assignments.toml"))?)?;
        validate_assignment_snapshot(&snapshot.subscribers, &state)?;
        state
    } else {
        crate::assignment::AssignmentState {
            assignments: snapshot
                .subscribers
                .iter()
                .map(|s| {
                    (
                        s.id.clone(),
                        crate::assignment::Pin {
                            source_node: s.node.clone(),
                            node: s.node.clone(),
                        },
                    )
                })
                .collect(),
        }
    };
    let core = if current.core_plan {
        ensure!(
            current.managed && current.assignments,
            "Core plan pointer is inconsistent"
        );
        let core: crate::core_plan::Plan =
            serde_json::from_str(&fs::read_to_string(path.join("core-plan.json"))?)?;
        crate::core_plan::validate_snapshot(&core, &snapshot.subscribers)?;
        Some(core)
    } else {
        None
    };
    Ok((snapshot.subscribers, state, core))
}
pub fn publish_assigned(
    root: &Path,
    plan: &Plan,
    subscribers: &[Subscriber],
    state: &crate::assignment::AssignmentState,
) -> Result<Publication> {
    ensure!(
        !read_current(root)?.is_some_and(|c| c.core_plan),
        "Core-aware generation requires explicit orchestrated publication"
    );
    publish_bundle(root, plan, subscribers, Some(state), None)
}
pub fn publish_orchestrated(
    root: &Path,
    plan: &Plan,
    subscribers: &[Subscriber],
    state: &crate::assignment::AssignmentState,
    core: Option<&crate::core_plan::Plan>,
) -> Result<Publication> {
    publish_bundle(root, plan, subscribers, Some(state), core)
}
pub struct CommittedCorePlan {
    pub generation: String,
    pub plan: crate::core_plan::Plan,
}
pub fn load_core_plan(root: &Path) -> Result<Option<CommittedCorePlan>> {
    let Some(current) = read_current(root)?.filter(|c| c.core_plan) else {
        return Ok(None);
    };
    let (_, _, plan) = load_managed(root, &current)?;
    Ok(Some(CommittedCorePlan {
        generation: current.generation,
        plan: plan.context("Committed core plan is missing")?,
    }))
}
