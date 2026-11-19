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

#[tokio::test(start_paused = true)]
async fn legacy_router_locations_equal_the_manager_node_locations() {
    let cfg = crate::config::Config::from_toml_str(
        "[[routers]]\nname='Core'\naddress='192.0.2.1'\nusername='u'\npassword_file='/x'\n\
         latitude=14.5995\nlongitude=120.9842\n\
         [[routers]]\nname='Distro'\naddress='192.0.2.2'\nusername='u'\npassword_file='/x'\n\
         latitude=-33.8688\nlongitude=-111.97804110201159\n",
    )
    .unwrap();
    let mut observed = input(1);
    observed.router_locations = super::super::publication::router_locations(&cfg);
    let pair = prepare(observed).unwrap();
    // Make the legacy network: the same nodes without a location, then the legacy writer.
    let mut legacy = pair.network().clone();
    let mut stored = std::collections::BTreeMap::new();
    let mut work = vec![&mut legacy];
    while let Some(tree) = work.pop() {
        for (name, node) in tree.as_object_mut().unwrap() {
            let node = node.as_object_mut().unwrap();
            node.remove("latitude");
            node.remove("longitude");
            let id = node["id"].as_str().unwrap().to_string();
            let router = if id.starts_with("wan") {
                "Core"
            } else {
                "Distro"
            };
            stored.insert(
                name.clone(),
                crate::model::StoredNode {
                    id,
                    router: Some(router.into()),
                    parent: None,
                    r#type: "site".into(),
                    down: 0.0,
                    up: 0.0,
                    down_min: None,
                    up_min: None,
                    seeded: false,
                },
            );
            work.push(node.get_mut("children").unwrap());
        }
    }
    crate::topology::add_locations(&mut legacy, &stored, &cfg.routers);
    let text = legacy.to_string();
    assert!(text.contains("-111.97804110201159"), "{text}");
    // Enrollment reads the legacy network from disk: compare after a real text round trip.
    let legacy: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(compare_legacy(&pair, pair.rows(), &legacy).unwrap(), []);
}
