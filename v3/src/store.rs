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
    #[derive(Serialize)]
    struct SnapshotRef<'a> {
        subscribers: &'a [Subscriber],
    }
    let snapshot = toml::to_string_pretty(&SnapshotRef { subscribers })?;
    let current = read_current(root)?;
    let reusable = current.filter(|c| c.managed).filter(|c| {
        let p = root.join("staging").join(&c.generation);
        fs::read_to_string(p.join("ShapedDevices.csv"))
            .ok()
            .as_deref()
            == Some(&plan.csv)
            && fs::read_to_string(p.join("network.json")).ok().as_deref() == Some(&plan.network)
            && fs::read_to_string(p.join("subscribers.toml"))
                .ok()
                .as_deref()
                == Some(&snapshot)
    });
    let generation = if let Some(c) = reusable {
        c.generation
    } else {
        let generation = uuid::Uuid::new_v4().to_string();
        let p = root.join("staging").join(&generation);
        durable_directory(&p)?;
        atomic_write(&p.join("subscribers.toml"), &snapshot)?;
        atomic_write(&p.join("ShapedDevices.csv"), &plan.csv)?;
        atomic_write(&p.join("network.json"), &plan.network)?;
        atomic_write(
            &root.join("state/current.toml"),
            &toml::to_string(&Current {
                generation: generation.clone(),
                status: "staged_local_only".into(),
                managed: true,
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
