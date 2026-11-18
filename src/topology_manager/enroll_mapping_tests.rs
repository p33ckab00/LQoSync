use super::*;
use crate::topology_manager::policy_draft::tests::{candidate, legacy, scenario};

fn confirm_all(file: &mut MappingFile) {
    for row in &mut file.rows {
        if let RowDecision::Map { confirmed, .. } = &mut row.decision {
            *confirmed = true;
        }
    }
    for node in &mut file.nodes {
        node.confirmed = true;
    }
}

#[tokio::test(start_paused = true)]
async fn ppp_row_is_suggested_by_circuit_name_and_dhcp_row_by_mac() {
    let (cfg, snap) = scenario();
    let old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let file = propose(&pair, &old.rows, &old.network, None).unwrap();
    let g1 = file
        .rows
        .iter()
        .find(|r| r.legacy_circuit_name == "g1")
        .unwrap();
    assert_eq!(
        g1.decision,
        RowDecision::Map {
            account_ref: "ppp-name/v1/g1".into(),
            rule: Some(SuggestRule::CircuitNameEqualsPppName),
            confirmed: false,
        }
    );
    let pc1 = file
        .rows
        .iter()
        .find(|r| r.legacy_circuit_name == "DHCP-pc-1")
        .unwrap();
    assert!(matches!(
        &pc1.decision,
        RowDecision::Map { account_ref, rule: Some(SuggestRule::MacEqualsLeaseMac), confirmed: false }
            if account_ref == "dhcp-mac/v1/02:00:00:00:02:01"
    ));
    assert!(file.candidate_only.is_empty(), "{:?}", file.candidate_only);
}

#[tokio::test(start_paused = true)]
async fn ip_match_alone_is_never_suggested() {
    let (cfg, snap) = scenario();
    let mut old = legacy(&cfg, &snap);
    let row = old
        .rows
        .iter_mut()
        .find(|r| r.circuit_name == "g1")
        .unwrap();
    row.circuit_name = "renamed".into();
    row.mac = "02:00:00:00:99:99".into();
    let pair = candidate(&cfg, &snap).await;
    let file = propose(&pair, &old.rows, &old.network, None).unwrap();
    let entry = file
        .rows
        .iter()
        .find(|r| r.legacy_circuit_name == "renamed")
        .unwrap();
    assert_eq!(entry.decision, RowDecision::Undecided);
    assert_eq!(file.candidate_only, vec!["ppp-name/v1/g1".to_string()]);
}

#[tokio::test(start_paused = true)]
async fn equal_node_ids_are_suggested_unconfirmed() {
    let (cfg, snap) = scenario();
    let old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let file = propose(&pair, &old.rows, &old.network, None).unwrap();
    let gold = file
        .nodes
        .iter()
        .find(|n| n.legacy_name == "Gold-Hub-1")
        .unwrap();
    assert_eq!(gold.current_id.as_deref(), Some(gold.legacy_id.as_str()));
    assert!(!gold.confirmed);
}

#[tokio::test(start_paused = true)]
async fn previous_confirmed_decision_carries_over_for_the_same_legacy_row() {
    let (cfg, snap) = scenario();
    let old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let mut first = propose(&pair, &old.rows, &old.network, None).unwrap();
    let i = first
        .rows
        .iter()
        .position(|r| r.legacy_circuit_name == "s1")
        .unwrap();
    first.rows[i].decision = RowDecision::Drop;
    let again = propose(&pair, &old.rows, &old.network, Some(&first)).unwrap();
    assert_eq!(
        again
            .rows
            .iter()
            .find(|r| r.legacy_circuit_name == "s1")
            .unwrap()
            .decision,
        RowDecision::Drop
    );
}

#[tokio::test(start_paused = true)]
async fn unconfirmed_or_undecided_entry_is_drift() {
    let (cfg, snap) = scenario();
    let old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let file = propose(&pair, &old.rows, &old.network, None).unwrap();
    let drift = build_manifest(&pair, &old.rows, &old.network, &file).unwrap_err();
    assert!(
        drift.issues.iter().any(|i| i.contains("unconfirmed")),
        "{drift}"
    );
}

#[tokio::test(start_paused = true)]
async fn candidate_row_without_legacy_row_is_drift() {
    let (cfg, snap) = scenario();
    let mut old = legacy(&cfg, &snap);
    old.rows.retain(|r| r.circuit_name != "g1");
    let pair = candidate(&cfg, &snap).await;
    let mut file = propose(&pair, &old.rows, &old.network, None).unwrap();
    confirm_all(&mut file);
    let drift = build_manifest(&pair, &old.rows, &old.network, &file).unwrap_err();
    assert!(
        drift.issues.iter().any(|i| i.contains("ppp-name/v1/g1")),
        "{drift}"
    );
}

#[tokio::test(start_paused = true)]
async fn reconnect_with_new_ip_is_drift_and_names_the_row() {
    let (cfg, snap) = scenario();
    let mut old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let mut file = propose(&pair, &old.rows, &old.network, None).unwrap();
    confirm_all(&mut file);
    old.rows
        .iter_mut()
        .find(|r| r.circuit_name == "g2")
        .unwrap()
        .ipv4 = "100.64.0.99".into();
    let drift = build_manifest(&pair, &old.rows, &old.network, &file).unwrap_err();
    assert!(!drift.issues.is_empty());
}

#[tokio::test(start_paused = true)]
async fn confirmed_file_builds_a_manifest_that_adoption_accepts() {
    let (cfg, snap) = scenario();
    let old = legacy(&cfg, &snap);
    let pair = candidate(&cfg, &snap).await;
    let mut file = propose(&pair, &old.rows, &old.network, None).unwrap();
    confirm_all(&mut file);
    let (manifest, roots, attachments) =
        build_manifest(&pair, &old.rows, &old.network, &file).unwrap_or_else(|d| panic!("{d}"));
    assert_eq!(manifest.rows.len(), old.rows.len());
    crate::topology_manager::legacy_static::adopt(
        &pair,
        &old.rows,
        &old.network,
        &manifest,
        &roots,
        &attachments,
    )
    .unwrap();
}
