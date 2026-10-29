use lqosync_v3::{
    core_diagnostics::{self, Origin},
    routeros, store,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (mut saved, mut export_fixture, mut end_to_end) = (false, false, false);
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--saved" => saved = true,
            "--fixture" => export_fixture = true,
            "--end-to-end" => end_to_end = true,
            _ => anyhow::bail!("Unknown core-validate option"),
        }
    }
    let root = std::env::var_os("LQOSYNC3_DATA")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?.join("data"))
        .canonicalize()?;
    anyhow::ensure!(
        !root.starts_with("/opt/libreqos") && !root.starts_with("/etc/libreqos"),
        "Use the LQoSync application data directory"
    );
    let settings = store::load_settings(&root)?;
    let required = lqosync_v3::validation::required_routers(&settings);
    let selected = |r: &&lqosync_v3::model::Router| {
        r.role == "core" || r.role == "both" || (end_to_end && required.contains(r.id.as_str()))
    };
    let (mut inventories, mut errors) = if saved {
        core_diagnostics::saved_routers(settings.routers.iter().filter(selected), &root)
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };
    for router in settings.routers.iter().filter(selected) {
        if saved {
            continue;
        }
        let inventory = match routeros::inspect_from(router, &root).await {
            Ok(i) => i,
            Err(_) => {
                errors.insert(
                    router.id.clone(),
                    "Core read failed; check connection, credentials and TLS".into(),
                );
                continue;
            }
        };
        store::atomic_write(
            &root
                .join("state/router-inventory")
                .join(format!("{}.toml", router.id)),
            &toml::to_string_pretty(&inventory)?,
        )?;
        println!(
            "Router inventory saved locally: {} tables, {} table errors",
            inventory.tables.len(),
            inventory.errors.len()
        );
        for (table, rows) in &inventory.tables {
            println!("{table}: {} rows", rows.len());
        }
        for table in inventory.errors.keys() {
            println!("Incomplete table: {table}");
        }
        inventories.insert(router.id.clone(), Arc::new(inventory));
    }
    let report = core_diagnostics::review(
        &settings,
        &inventories,
        &errors,
        lqosync_v3::orchestrator::now(),
        if saved { Origin::Saved } else { Origin::Live },
    );
    if export_fixture {
        for (n, inventory) in settings
            .routers
            .iter()
            .filter(|r| r.role == "core" || r.role == "both")
            .filter_map(|r| inventories.get(&r.id))
            .enumerate()
        {
            let fixture = lqosync_v3::policy_fixture::fixture(inventory)?;
            store::atomic_write(
                &root
                    .join("state/core-policy-fixtures")
                    .join(format!("fixture-{:03}.json", n + 1)),
                &serde_json::to_string_pretty(&fixture)?,
            )?;
            println!(
                "Sanitized routing projection exported locally; raw identifiers and comments are excluded."
            );
        }
    }
    store::atomic_write(
        &root.join("state/core-validation.json"),
        &serde_json::to_string_pretty(&report)?,
    )?;
    for core in &report.cores {
        println!(
            "Core review: {:?}; {} observed/configured paths; {} findings; {} active PCC rules; {} active FastTrack rules",
            core.health,
            core.wans.len(),
            core.findings.len(),
            core.counts.pcc_rules,
            core.counts.fasttrack_rules
        );
        let mut codes = BTreeMap::new();
        for f in &core.findings {
            *codes.entry(f.code.as_str()).or_insert(0) += 1;
        }
        for (code, count) in codes {
            println!("{code}: {count}");
        }
    }
    let mut end_to_end_passed = true;
    if end_to_end {
        let validation = lqosync_v3::validation::review(
            &settings,
            &inventories,
            &errors,
            &root,
            lqosync_v3::orchestrator::now(),
            if saved { Origin::Saved } else { Origin::Live },
        );
        store::atomic_write(
            &root.join("state/end-to-end-validation.json"),
            &serde_json::to_string_pretty(&validation)?,
        )?;
        println!(
            "End-to-end review: {:?}; {} subscribers; {} checks",
            validation.status(),
            validation.subscribers,
            validation.checks.len()
        );
        end_to_end_passed = validation.status() == core_diagnostics::Outcome::Pass;
        for check in &validation.checks {
            println!("{}: {:?}", check.code, check.status);
        }
    }
    anyhow::ensure!(
        !report.cores.is_empty()
            && report.cores.iter().all(|c| !matches!(
                c.health,
                core_diagnostics::Health::Failed
                    | core_diagnostics::Health::Missing
                    | core_diagnostics::Health::Partial
            )),
        "Core validation input is unavailable or incomplete; local report records the failure"
    );
    anyhow::ensure!(
        end_to_end_passed,
        "End-to-end validation did not pass; inspect the local report and Validation page"
    );
    Ok(())
}
