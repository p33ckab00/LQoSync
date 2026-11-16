use super::*;
use crate::topology_manager::publication::{prepare, tests::input};

#[tokio::test(start_paused = true)]
async fn formatting_only_changes_are_equivalent_without_identity_inference() {
    let pair = prepare(input(1)).unwrap();
    let mut baseline = pair.rows().to_vec();
    baseline[0].download_max = "50.000000".into();
    assert!(
        compare_legacy(&pair, &baseline, pair.network())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test(start_paused = true)]
async fn endpoint_rate_and_unresolved_identity_changes_are_reported() {
    let pair = prepare(input(1)).unwrap();
    for field in 0..3 {
        let mut baseline = pair.rows().to_vec();
        match field {
            0 => baseline[0].ipv4 = "192.0.2.3".into(),
            1 => baseline[0].download_max = "49".into(),
            _ => baseline[0].circuit_id = "unmapped-old-account".into(),
        }
        assert!(
            !compare_legacy(&pair, &baseline, pair.network())
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn unmanaged_rows_and_network_changes_cannot_disappear_from_a_parity_report() {
    let pair = prepare(input(1)).unwrap();
    let mut baseline = pair.rows().to_vec();
    let mut unmanaged = baseline[0].clone();
    unmanaged.circuit_id = "static-circuit".into();
    unmanaged.device_id = "static-device".into();
    baseline.push(unmanaged);
    let report = compare_legacy(&pair, &baseline, pair.network()).unwrap();
    assert!(report.iter().any(|i| matches!(i, ParityIssue::MissingRow { circuit_id, .. } if circuit_id == "static-circuit")));
    assert!(
        !compare_legacy(&pair, pair.rows(), &serde_json::json!({}))
            .unwrap()
            .is_empty()
    );
    baseline.push(baseline[0].clone());
    assert!(
        compare_legacy(&pair, &baseline, pair.network())
            .unwrap()
            .contains(&ParityIssue::DuplicateRow)
    );
}
