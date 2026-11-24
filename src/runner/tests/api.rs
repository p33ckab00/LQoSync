use super::*;

#[tokio::test]
async fn api_bind_needs_the_instance_lock() {
    let e = env();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    assert!(r.api_bind().is_err());
    r.lock().unwrap();
    let (_listener, _guard) = r.api_bind().unwrap();
    assert!(e.state.join("api.sock").exists());
}

mod operations {
    use super::super::enrollment::cli_flow::{AGE, setup};
    use super::*;
    use crate::api::dispatch::{LostLock, handle};
    use crate::api::protocol::Request;
    use serde_json::{Value, json};

    async fn call(r: &mut Runner<FakeConnector>, op: &str, args: Value) -> Value {
        let req = Request {
            v: 1,
            id: "t".into(),
            op: op.into(),
            args,
        };
        let line = handle(r, &req)
            .await
            .unwrap_or_else(|LostLock(l)| panic!("lost lock: {l}"));
        serde_json::from_str(line.trim_end()).unwrap()
    }

    async fn draft(r: &mut Runner<FakeConnector>) -> Value {
        serde_json::to_value(r.propose_policy(AGE).await.unwrap()).unwrap()
    }

    fn owner_file(e: &Env, bytes: &[u8]) {
        drop(crate::topology_manager::store::RegistryStore::open(&e.state).unwrap());
        crate::files::write_secret(&e.state.join("topology-manager/runtime-owner.json"), bytes)
            .unwrap();
    }

    #[tokio::test]
    async fn an_automatic_edit_saves_in_one_call_and_a_replay_commits_nothing() {
        use web_contract::topology::{BuilderEdit, Capacity, WebCommand};
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let init = WebCommand::Edit {
            expected_revision: 0,
            edit: BuilderEdit::Initialize,
        };
        let saved = r
            .api_web_mutate("automatic-edit-0001", "operator", init)
            .await
            .unwrap();
        assert_eq!(
            (saved["state"].as_str(), saved["revision"].as_str()),
            (Some("saved"), Some("1"))
        );
        let create = WebCommand::Edit {
            expected_revision: 1,
            edit: BuilderEdit::NodeCreate {
                name: "Distribution".into(),
                parent: None,
                capacity: Capacity::Fixed {
                    down_mbps: 100,
                    up_mbps: 50,
                    factor_percent: None,
                },
                node_kind: None,
            },
        };
        let first = r
            .api_web_mutate("automatic-edit-0002", "operator", create.clone())
            .await
            .unwrap();
        let replay = r
            .api_web_mutate("automatic-edit-0002", "operator", create)
            .await
            .unwrap();
        assert_eq!(first, replay);
        let doc = r.api_registry().unwrap().unwrap();
        assert_eq!(
            (doc.revision, doc.catalog.as_ref().unwrap().nodes.len()),
            (2, 1)
        );
        let node_id = doc.catalog.unwrap().nodes[0].id.clone();
        // The same key with another payload is refused; nothing runs.
        let other = WebCommand::Edit {
            expected_revision: 2,
            edit: BuilderEdit::NodeRemove {
                node_id: node_id.clone(),
            },
        };
        assert!(
            r.api_web_mutate("automatic-edit-0002", "operator", other)
                .await
                .is_err()
        );
        // A stale revision fails before any commit.
        let stale = WebCommand::Edit {
            expected_revision: 1,
            edit: BuilderEdit::NodeRemove { node_id },
        };
        let failed = r
            .api_web_mutate("automatic-edit-0003", "operator", stale)
            .await
            .unwrap();
        assert_eq!(failed["state"], "failed");
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 2);
        assert_eq!(e.fake.reads.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_automatic_location_edit_keeps_the_full_decimal_text() {
        use web_contract::topology::{BuilderEdit, Capacity, GeoType, Location, WebCommand};
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let edit = |rev, edit| WebCommand::Edit {
            expected_revision: rev,
            edit,
        };
        r.api_web_mutate(
            "automatic-loc-00001",
            "operator",
            edit(0, BuilderEdit::Initialize),
        )
        .await
        .unwrap();
        r.api_web_mutate(
            "automatic-loc-00002",
            "operator",
            edit(
                1,
                BuilderEdit::NodeCreate {
                    name: "Tower".into(),
                    parent: None,
                    capacity: Capacity::Fixed {
                        down_mbps: 100,
                        up_mbps: 50,
                        factor_percent: None,
                    },
                    node_kind: None,
                },
            ),
        )
        .await
        .unwrap();
        let id = r.api_registry().unwrap().unwrap().catalog.unwrap().nodes[0]
            .id
            .clone();
        let location = Location {
            latitude: "12.236199459585354".into(),
            longitude: "122.02419129509887".into(),
        };
        let saved = r
            .api_web_mutate(
                "automatic-loc-00003",
                "operator",
                edit(
                    2,
                    BuilderEdit::NodeLocation {
                        node_id: id,
                        geo_type: Some(GeoType::Site),
                        location: Some(location.clone()),
                    },
                ),
            )
            .await
            .unwrap();
        assert_eq!(saved["state"], "saved");
        let doc = r.api_registry().unwrap().unwrap();
        let geo = doc.catalog.unwrap().nodes[0].geo.clone().unwrap();
        assert_eq!(geo.location, Some(location));
    }

    #[tokio::test]
    async fn a_layout_save_keeps_the_registry_revision_and_refuses_unknown_nodes() {
        use web_contract::topology::{BuilderEdit, Capacity, WebCommand};
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let edit = |rev, edit| WebCommand::Edit {
            expected_revision: rev,
            edit,
        };
        r.api_web_mutate(
            "layout-test-00001",
            "operator",
            edit(0, BuilderEdit::Initialize),
        )
        .await
        .unwrap();
        r.api_web_mutate(
            "layout-test-00002",
            "operator",
            edit(
                1,
                BuilderEdit::NodeCreate {
                    name: "Tower".into(),
                    parent: None,
                    capacity: Capacity::Fixed {
                        down_mbps: 100,
                        up_mbps: 50,
                        factor_percent: None,
                    },
                    node_kind: None,
                },
            ),
        )
        .await
        .unwrap();
        r.reconcile_requested = false;
        let id = r.api_registry().unwrap().unwrap().catalog.unwrap().nodes[0]
            .id
            .clone();
        let saved = call(
            &mut r,
            "topology.layout.save",
            json!({"expected_revision": 0, "positions": {id.clone(): [120.5, -40.0]}}),
        )
        .await;
        assert_eq!(saved["result"]["revision"], 1, "{saved}");
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 2);
        assert!(
            !r.reconcile_requested,
            "a layout save never schedules shaping"
        );
        let stale = call(
            &mut r,
            "topology.layout.save",
            json!({"expected_revision": 0, "positions": {}}),
        )
        .await;
        assert_eq!(stale["error"]["code"], "conflict", "{stale}");
        let unknown = call(
            &mut r,
            "topology.layout.save",
            json!({"expected_revision": 1, "positions": {"ghost": [1.0, 1.0]}}),
        )
        .await;
        assert_eq!(unknown["ok"], false, "{unknown}");
        let view: web_contract::topology::TopologyView = serde_json::from_slice(
            &std::fs::read(e.state.join(web_contract::topology::CACHE)).unwrap(),
        )
        .unwrap();
        assert_eq!(view.layout_revision.as_deref(), Some("1"));
        assert_eq!(view.nodes[0].position, Some([120.5, -40.0]));
    }

    #[tokio::test]
    async fn frontend_initialize_requires_confirmation_and_builds_without_live_io() {
        use crate::topology_manager::edits::{CapacityInput, Edit};
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let shaped_before = read(&e.lqos.join("ShapedDevices.csv"));
        let network_before = read(&e.lqos.join("network.json"));
        assert!(r.api_registry().unwrap().is_none());
        let proposal = r.api_edit_propose(0, Edit::Initialize).await.unwrap();
        assert_eq!(proposal["nodes"], json!([]));
        assert!(r.api_registry().unwrap().is_none());
        assert!(!e.state.join("topology-manager/registry.json").exists());
        r.api_edit_cancel(proposal["proposal_id"].as_str().unwrap())
            .unwrap();
        assert!(r.api_registry().unwrap().is_none());
        let proposal = r.api_edit_propose(0, Edit::Initialize).await.unwrap();
        let command = web_contract::topology::WebCommand::EditConfirm {
            proposal_id: proposal["proposal_id"].as_str().unwrap().into(),
        };
        let receipt = r
            .api_web_mutate("frontend-init-0001", "operator", command.clone())
            .await
            .unwrap();
        let repeated = r
            .api_web_mutate("frontend-init-0001", "operator", command)
            .await
            .unwrap();
        assert_eq!(receipt, repeated);
        let doc = r.api_registry().unwrap().unwrap();
        assert_eq!(doc.revision, 1);
        assert!(doc.frontend_initialized);
        assert!(doc.catalog.as_ref().unwrap().nodes.is_empty());
        assert!(doc.catalog.as_ref().unwrap().sources.is_empty());
        assert!(doc.subscribers.is_empty() && doc.export_ownership.is_none());
        assert!(r.api_edit_propose(1, Edit::Initialize).await.is_err());
        let proposal = r
            .api_edit_propose(
                1,
                Edit::NodeCreate {
                    name: "Distribution".into(),
                    parent: None,
                    node_kind: None,
                    capacity: CapacityInput::Fixed {
                        down_mbps: 100,
                        up_mbps: 50,
                        factor_percent: None,
                    },
                },
            )
            .await
            .unwrap();
        r.api_edit_confirm(proposal["proposal_id"].as_str().unwrap())
            .unwrap();
        let doc = r.api_registry().unwrap().unwrap();
        assert_eq!(doc.catalog.unwrap().nodes.len(), 1);
        assert!(doc.subscribers.is_empty() && doc.export_ownership.is_none());
        assert_eq!(e.fake.reads.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(read(&e.lqos.join("ShapedDevices.csv")), shaped_before);
        assert_eq!(read(&e.lqos.join("network.json")), network_before);
    }

    #[tokio::test]
    async fn frontend_initialized_discovery_is_inventory_and_attach_keeps_subscribers_unassigned() {
        use crate::topology_manager::edits::Edit;
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let p = r.api_edit_propose(0, Edit::Initialize).await.unwrap();
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        let available = r.api_source_candidates().await.unwrap();
        let key = available["candidates"][0]["key"]
            .as_str()
            .unwrap()
            .to_string();
        let unchanged = r.api_registry().unwrap().unwrap();
        assert!(unchanged.catalog.as_ref().unwrap().nodes.is_empty());
        assert!(unchanged.catalog.as_ref().unwrap().sources.is_empty());
        let p = r
            .api_edit_propose(
                1,
                Edit::SourceAdd {
                    key: key.clone(),
                    parent: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(p["new_records"], 0);
        assert!(
            r.api_registry()
                .unwrap()
                .unwrap()
                .catalog
                .unwrap()
                .nodes
                .is_empty()
        );
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        let doc = r.api_registry().unwrap().unwrap();
        let c = doc.catalog.unwrap();
        assert_eq!(c.sources.len(), 1);
        assert_eq!(c.sources[0].id, key);
        assert_eq!(c.nodes.len(), 1);
        assert!(c.nodes[0].parent.is_none());
        assert!(doc.subscribers.is_empty() && doc.export_ownership.is_none());
    }

    #[tokio::test]
    async fn frontend_initialize_refuses_an_active_writer_at_preview_and_confirm() {
        use crate::topology_manager::edits::Edit;
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let proposal = r.api_edit_propose(0, Edit::Initialize).await.unwrap();
        r.open_topology_store().unwrap();
        crate::files::write_secret(
            &e.state.join("topology-manager/runtime-owner.json"),
            br#"{"schema_version":1,"writer":"topology"}"#,
        )
        .unwrap();
        assert!(
            r.api_edit_confirm(proposal["proposal_id"].as_str().unwrap())
                .is_err()
        );
        assert!(r.api_edit_propose(0, Edit::Initialize).await.is_err());
        let doc = r.api_registry().unwrap().unwrap();
        assert!(doc.catalog.is_none() && !doc.frontend_initialized);
    }

    #[tokio::test]
    async fn status_reports_the_legacy_writer_without_an_owner_file() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "status", json!({})).await["result"]["writer"],
            json!("legacy")
        );
    }

    #[tokio::test]
    async fn status_reports_the_topology_writer_with_the_owner_file() {
        let e = env();
        owner_file(&e, br#"{"schema_version":1,"writer":"topology"}"#);
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "status", json!({})).await["result"]["writer"],
            json!("topology")
        );
    }

    #[tokio::test]
    async fn status_reports_an_unknown_writer_for_a_bad_owner_file() {
        let e = env();
        owner_file(&e, b"not json");
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "status", json!({})).await["result"]["writer"],
            json!("unknown")
        );
    }

    #[tokio::test]
    async fn status_without_a_registry_reports_null_and_creates_nothing() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let v = call(&mut r, "status", json!({})).await;
        assert_eq!(v["ok"], json!(true), "{v}");
        assert_eq!(v["result"]["registry_revision"], Value::Null);
        assert_eq!(v["result"]["enrolled"], json!(false));
        assert!(v["result"]["version"].is_string());
        assert!(!e.state.join("topology-manager").exists());
    }

    #[tokio::test]
    async fn registry_get_without_a_registry_is_null() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "registry.get", json!({})).await["result"],
            Value::Null
        );
        assert!(!e.state.join("topology-manager").exists());
    }

    #[tokio::test]
    async fn status_after_install_reports_the_revision() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        let d = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&d).unwrap();
        assert_eq!(
            call(&mut r, "status", json!({})).await["result"]["registry_revision"],
            json!(1)
        );
        let doc = call(&mut r, "registry.get", json!({})).await;
        assert_eq!(doc["result"]["revision"], json!(1));
    }

    #[tokio::test]
    async fn policy_commit_succeeds_and_returns_the_new_revision() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = draft(&mut r).await;
        let args = json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]});
        let v = call(&mut r, "policy.commit", args).await;
        assert_eq!(v["result"]["revision"], json!(1), "{v}");
    }

    #[tokio::test]
    async fn two_commits_with_one_revision_give_conflict_then_the_store_works() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = draft(&mut r).await;
        let args = json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]});
        assert_eq!(
            call(&mut r, "policy.commit", args.clone()).await["ok"],
            json!(true)
        );
        let v = call(&mut r, "policy.commit", args).await;
        assert_eq!(v["error"]["code"], json!("conflict"), "{v}");
        assert_eq!(
            call(&mut r, "status", json!({})).await["result"]["registry_revision"],
            json!(1)
        );
    }

    #[tokio::test]
    async fn an_invalid_catalog_gives_invalid() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let mut d = draft(&mut r).await;
        d["catalog"]["nodes"][0]["parent"] = json!("missing-node");
        let args = json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]});
        let v = call(&mut r, "policy.commit", args).await;
        assert_eq!(v["error"]["code"], json!("invalid"), "{v}");
    }

    #[tokio::test]
    async fn unknown_op_and_bad_args_are_reported() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "nope", json!({})).await["error"]["code"],
            json!("unknown_op")
        );
        let v = call(&mut r, "policy.commit", json!({"expected_revision": "x"})).await;
        assert_eq!(v["error"]["code"], json!("bad_request"), "{v}");
    }

    #[tokio::test]
    async fn source_studio_ops_report_capabilities_and_never_echo_a_draft() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let caps = call(&mut r, "source_studio.capabilities", json!({})).await;
        assert_eq!(caps["result"]["version"], json!(1), "{caps}");
        assert_eq!(
            caps["result"]["features"],
            json!([
                "draft_test",
                "access_profiles",
                "source_collect",
                "wan_evidence",
                "policy_preview",
                "policy_save"
            ])
        );
        let secret = "s3cret-in-args";
        let connection = |address: Value| {
            json!({"address": address, "port": 8729, "tls": true, "tls_verify": true,
                   "username": "studio", "password": secret, "ca_pem": null})
        };
        let bad = call(
            &mut r,
            "source_studio.test",
            json!({"connection": connection(json!(5)), "required_sources": []}),
        )
        .await;
        assert_eq!(bad["error"]["code"], json!("bad_request"), "{bad}");
        let invalid = call(
            &mut r,
            "source_studio.test",
            json!({"connection": connection(json!("bad address")), "required_sources": []}),
        )
        .await;
        assert_eq!(invalid["error"]["code"], json!("invalid"), "{invalid}");
        let ok = call(
            &mut r,
            "source_studio.test",
            json!({"connection": connection(json!("198.51.100.7")), "required_sources": ["ppp"]}),
        )
        .await;
        assert_eq!(
            ok["result"]["checks"].as_array().map(Vec::len),
            Some(5),
            "{ok}"
        );
        for v in [bad, invalid, ok] {
            assert!(!v.to_string().contains(secret), "{v}");
        }
    }

    #[tokio::test]
    async fn source_collection_answers_a_snapshot_and_writes_the_studio_cache() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let id = crate::topology_manager::policy_draft::router_id(&r.cfg.routers[0].name);
        let digest = r.config_digest.clone();
        let stale = call(
            &mut r,
            "source_studio.collect",
            json!({"router_id": id, "expected_config_digest": "old", "sources": ["ppp"]}),
        )
        .await;
        assert_eq!(stale["error"]["code"], json!("invalid"), "{stale}");
        let ok = call(
            &mut r,
            "source_studio.collect",
            json!({"router_id": id, "expected_config_digest": digest, "sources": ["ppp"]}),
        )
        .await;
        let snapshot = &ok["result"];
        assert_eq!(snapshot["router_id"], json!(id), "{ok}");
        assert_eq!(snapshot["tables"].as_array().map(Vec::len), Some(11));
        let cache: web_contract::source_studio::StudioCache = serde_json::from_slice(
            &std::fs::read(e.state.join(web_contract::source_studio::CACHE)).unwrap(),
        )
        .unwrap();
        let view = cache.routers.iter().find(|v| v.router_id == id).unwrap();
        assert_eq!(view.snapshots.len(), 1);
        assert_eq!(view.snapshots[0].id, snapshot["id"].as_str().unwrap());
    }

    /// A PPP policy in the configured PPP rate order, grouped by profile.
    fn ppp_policy() -> serde_json::Value {
        let legacy = json!({"format": "legacy_configured"});
        let mut rates = vec![json!({"kind": "subscriber_override"})];
        for kind in ["account_comment", "active_comment", "profile_comment"] {
            let mut r = legacy.clone();
            r["kind"] = json!(kind);
            rates.push(r);
        }
        rates.extend([
            json!({"kind": "profile_rate_limit"}),
            json!({"kind": "profile_name"}),
            json!({"kind": "settings"}),
        ]);
        json!({"id": null, "name": "PPPoE", "enabled": true, "priority": 10, "source": "ppp",
               "read_access_ref": "default", "members": "joined",
               "filter": {"kind": "all", "rules": []},
               "group": {"kind": "by_field", "field": "account.profile"},
               "identity": "registry", "rates": rates, "parent": {"kind": "router"},
               "capacity": {"kind": "from_plans", "down_factor": "1", "up_factor": "1",
                            "minimum_down_bps": "0", "minimum_up_bps": "0"},
               "writes": []})
    }

    fn sources_set(
        rev: u64,
        id: &str,
        digest: &str,
        policies: serde_json::Value,
    ) -> web_contract::topology::WebCommand {
        serde_json::from_value(
            json!({"command": "edit", "expected_revision": rev, "edit": {
            "kind": "router.sources_set", "router_id": id, "expected_config_digest": digest,
            "policies": policies}}),
        )
        .unwrap()
    }

    fn revision(r: &Runner<FakeConnector>) -> u64 {
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .revision
    }

    fn router_entry(
        r: &Runner<FakeConnector>,
        id: &str,
    ) -> crate::topology_manager::catalog::CatalogRouter {
        r.topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone()
            .catalog
            .unwrap()
            .routers
            .into_iter()
            .find(|x| x.id == id)
            .unwrap()
    }

    fn router_sources(r: &Runner<FakeConnector>, id: &str) -> usize {
        use crate::topology_manager::catalog::SourceOrigin;
        let doc = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        doc.catalog.unwrap().sources.iter()
            .filter(|s| matches!(&s.origin, SourceOrigin::Subscribers { scope, .. } if scope.router_id == id))
            .count()
    }

    #[tokio::test]
    async fn uninitialized_registry_refuses_policy_without_side_effects() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let id = crate::topology_manager::policy_draft::router_id(&r.cfg.routers[0].name);
        let digest = r.config_digest.clone();
        let out = r
            .api_web_mutate(
                "studio-save-0000000001",
                "alice",
                sources_set(0, &id, &digest, json!([ppp_policy()])),
            )
            .await
            .unwrap();
        assert_eq!(out["state"], json!("failed"), "{out}");
        assert!(
            out["error"]
                .as_str()
                .unwrap_or_default()
                .contains("not_initialized"),
            "{out}"
        );
        let doc = r.topology_store.as_ref().unwrap().document().unwrap();
        assert!(doc.catalog.is_none() && doc.revision == 0);
    }

    #[tokio::test]
    async fn policy_save_goes_through_receipts_revisions_and_digests() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = draft(&mut r).await;
        let v = call(&mut r, "policy.commit",
            json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]})).await;
        assert_eq!(v["result"]["revision"], json!(1), "{v}");
        let id = crate::topology_manager::policy_draft::router_id(&r.cfg.routers[0].name);
        let digest = r.config_digest.clone();
        // none_keeps_current_sources: before a save the router has its legacy sources.
        assert!(router_entry(&r, &id).source_policies.is_none());
        let legacy_sources = router_sources(&r, &id);
        assert!(legacy_sources > 0);
        // A save needs the newest complete snapshot of the router.
        let out = r
            .api_web_mutate(
                "studio-save-0000000001",
                "alice",
                sources_set(1, &id, &digest, json!([ppp_policy()])),
            )
            .await
            .unwrap();
        assert_eq!(out["state"], json!("failed"), "{out}");
        assert!(
            out["error"]
                .as_str()
                .unwrap()
                .contains("collect_router_first"),
            "{out}"
        );
        // partial_read_never_removes_circuits: a failed read cannot save or remove anything.
        e.fake
            .failing
            .lock()
            .unwrap()
            .insert(r.cfg.routers[0].name.clone());
        let v = call(
            &mut r,
            "source_studio.collect",
            json!({"router_id": id, "expected_config_digest": digest, "sources": ["ppp"]}),
        )
        .await;
        assert_eq!(v["result"]["tables"][2]["state"], json!("failed"), "{v}");
        let out = r
            .api_web_mutate(
                "studio-save-0000000002",
                "alice",
                sources_set(1, &id, &digest, json!([ppp_policy()])),
            )
            .await
            .unwrap();
        assert_eq!(out["state"], json!("failed"), "{out}");
        assert_eq!((revision(&r), router_sources(&r, &id)), (1, legacy_sources));
        e.fake.failing.lock().unwrap().clear();
        let v = call(
            &mut r,
            "source_studio.collect",
            json!({"router_id": id, "expected_config_digest": digest, "sources": ["ppp"]}),
        )
        .await;
        assert!(v["result"]["id"].is_string(), "{v}");
        // stale_config_digest_refuses_policy, and config_save_is_separate_from_policy_save: a saved
        // router edit that the service has not loaded yet gives another digest. The policy waits.
        let out = r
            .api_web_mutate(
                "studio-save-0000000003",
                "alice",
                sources_set(1, &id, "saved-not-loaded", json!([ppp_policy()])),
            )
            .await
            .unwrap();
        assert!(
            out["error"]
                .as_str()
                .unwrap()
                .contains("stale_config_digest"),
            "{out}"
        );
        assert_eq!(revision(&r), 1);
        // A stale registry revision is refused before commit.
        let out = r
            .api_web_mutate(
                "studio-save-0000000004",
                "alice",
                sources_set(0, &id, &digest, json!([ppp_policy()])),
            )
            .await
            .unwrap();
        assert_ne!(out["state"], json!("saved"), "{out}");
        assert_eq!(revision(&r), 1);
        // same_key_same_payload_saves_once.
        let save = sources_set(1, &id, &digest, json!([ppp_policy()]));
        let out = r
            .api_web_mutate("studio-save-0000000005", "alice", save.clone())
            .await
            .unwrap();
        assert_eq!(out["state"], json!("saved"), "{out}");
        assert_eq!(revision(&r), 2);
        let again = r
            .api_web_mutate("studio-save-0000000005", "alice", save)
            .await
            .unwrap();
        assert_eq!(again, out);
        assert_eq!(revision(&r), 2, "a retry commits nothing");
        let saved = router_entry(&r, &id).source_policies.unwrap();
        assert_eq!(saved.len(), 1);
        // The current engine prepares the saved policy, as the next cycle does.
        let preview = call(&mut r, "topology.preview.refresh", json!({})).await;
        assert_eq!(preview["result"]["state"], json!("ok"), "{preview}");
        assert_eq!(preview["result"]["registry_revision"], json!(2));
        assert!(
            saved[0]
                .id
                .as_deref()
                .is_some_and(|x| x.starts_with("policy-"))
        );
        // changed_payload_same_key_is_refused.
        let changed = sources_set(1, &id, &digest, json!([]));
        assert!(
            r.api_web_mutate("studio-save-0000000005", "alice", changed)
                .await
                .is_err()
        );
        assert_eq!(revision(&r), 2);
        // some_empty_policies_disables_sources: an explicit empty set removes the policy sources.
        let out = r
            .api_web_mutate(
                "studio-save-0000000006",
                "alice",
                sources_set(2, &id, &digest, json!([])),
            )
            .await
            .unwrap();
        assert_eq!(out["state"], json!("saved"), "{out}");
        assert_eq!(router_entry(&r, &id).source_policies, Some(vec![]));
        assert_eq!(router_sources(&r, &id), 0);
        let doc = r
            .topology_store
            .as_ref()
            .unwrap()
            .document()
            .unwrap()
            .clone();
        assert!(
            doc.source_settings
                .unwrap()
                .selections
                .iter()
                .all(|s| s.scope.router_id != id)
        );
    }

    #[tokio::test]
    async fn wan_decision_survives_read_failure() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = draft(&mut r).await;
        call(&mut r, "policy.commit",
            json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]})).await;
        let id = crate::topology_manager::policy_draft::router_id(&r.cfg.routers[0].name);
        let digest = r.config_digest.clone();
        let decide = |rev: u64, decision: &str| -> web_contract::topology::WebCommand {
            serde_json::from_value(
                json!({"command": "edit", "expected_revision": rev, "edit": {
                "kind": "wan.candidate_set", "router_id": id, "expected_config_digest": digest,
                "interface": "pppoe-out1", "decision": decision}}),
            )
            .unwrap()
        };
        let reads = e.fake.reads.load(std::sync::atomic::Ordering::SeqCst);
        let out = r
            .api_web_mutate("studio-wan-000000000001", "alice", decide(1, "pin"))
            .await
            .unwrap();
        assert_eq!(out["state"], json!("saved"), "{out}");
        assert_eq!(
            reads,
            e.fake.reads.load(std::sync::atomic::Ordering::SeqCst),
            "a decision reads no router"
        );
        e.fake
            .failing
            .lock()
            .unwrap()
            .insert(r.cfg.routers[0].name.clone());
        let v = call(
            &mut r,
            "source_studio.collect",
            json!({"router_id": id, "expected_config_digest": digest, "sources": ["wan"]}),
        )
        .await;
        assert_eq!(v["result"]["tables"][1]["state"], json!("failed"), "{v}");
        let kept = router_entry(&r, &id).wan_decisions;
        assert_eq!(kept.len(), 1);
        assert_eq!(
            kept[0].decision,
            web_contract::source_studio::WanCandidateDecision::Pin
        );
        let out = r
            .api_web_mutate("studio-wan-000000000002", "alice", decide(2, "clear"))
            .await
            .unwrap();
        assert_eq!(out["state"], json!("saved"), "{out}");
        assert!(router_entry(&r, &id).wan_decisions.is_empty());
    }

    /// Every file under the state directory, with its bytes.
    fn files(dir: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    out.insert(path, bytes);
                }
            }
        }
        out
    }

    #[tokio::test]
    async fn source_evaluation_refuses_stale_input_and_changes_no_state_file() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let id = crate::topology_manager::policy_draft::router_id(&r.cfg.routers[0].name);
        let digest = r.config_digest.clone();
        let before = files(&e.state);
        let args = |revision: &str| {
            json!({"actor": "alice", "router_id": id, "expected_revision": revision,
                   "expected_config_digest": digest, "snapshot_id": "snap-x", "policies": []})
        };
        let v = call(&mut r, "source_studio.evaluate", args("0")).await;
        let code = v["error"]["code"].as_str().unwrap_or_default().to_owned();
        assert!(code == "invalid" || code == "conflict", "{v}");
        let v = call(&mut r, "source_studio.evaluate", args("x")).await;
        assert!(v["error"]["code"].is_string(), "{v}");
        for part in ["summary", "records", "issues"] {
            let v = call(
                &mut r,
                "source_studio.candidate",
                json!({"actor": "alice", "id": "cand-old-1", "part": part}),
            )
            .await;
            assert_eq!(v["result"]["status"], json!("expired"), "{v}");
        }
        assert_eq!(files(&e.state), before, "no state file changed");
    }

    #[tokio::test]
    async fn a_lost_lock_answers_internal_and_returns_lost_lock() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let moved = e.state.with_extension("moved");
        std::fs::rename(&e.state, &moved).unwrap();
        std::fs::create_dir_all(&e.state).unwrap();
        let req = Request {
            v: 1,
            id: "t".into(),
            op: "status".into(),
            args: json!({}),
        };
        let Err(LostLock(line)) = handle(&mut r, &req).await else {
            panic!("a replaced state directory must lose the lock");
        };
        let v: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["error"]["code"], json!("internal"));
    }
}

#[tokio::test]
async fn client_against_a_served_runner_gets_status() {
    let e = env();
    let mut r = runner(&e, cfg(&e, "shadow", ""));
    r.lock().unwrap();
    let (listener, _guard) = r.api_bind().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::api::server::Job>(16);
    crate::api::server::spawn(listener, tx);
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            let line = crate::api::dispatch::handle(&mut r, &job.request)
                .await
                .unwrap_or_else(|l| l.0);
            let _ = job.reply.send(line);
        }
    });
    let v = crate::api::client::request(
        &e.state,
        "status",
        serde_json::json!({}),
        std::time::Duration::from_secs(5),
    )
    .await
    .unwrap();
    assert_eq!(v["mode"], serde_json::json!("shadow"));
    assert_eq!(v["registry_revision"], serde_json::Value::Null);
}

mod preview {
    use super::super::enrollment::cli_flow::{AGE, setup};
    use super::*;
    use serde_json::{Value, json};
    use std::sync::atomic::Ordering;

    async fn shadow_with_policy(e: &Env) -> Runner<FakeConnector> {
        let mut c = setup(e, None);
        c.runtime.mode = RuntimeMode::Shadow;
        let mut r = runner(e, c);
        r.lock().unwrap();
        let d = r.propose_policy(AGE).await.unwrap();
        r.api_policy_commit(0, d.catalog, d.source_settings)
            .unwrap();
        r
    }

    #[tokio::test]
    async fn preview_refresh_works_in_shadow_mode_before_enrollment() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        let v = r.api_preview_refresh().await.unwrap();
        assert_eq!(v["state"], json!("ok"), "{v}");
        assert_eq!(v["registry_revision"], json!(1));
        assert_eq!(v["nodes"].as_array().unwrap().len(), 4);
        assert_eq!(v["subscribers"].as_array().unwrap().len(), 7);
        assert!(v["collected_at"].as_u64().is_some());
        assert!(!v["rounded"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn preview_get_reuses_the_cache_without_router_reads() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        assert_eq!(r.api_preview_get().unwrap(), Value::Null);
        let fresh = r.api_preview_refresh().await.unwrap();
        let reads = e.fake.reads.load(Ordering::SeqCst);
        assert_eq!(r.api_preview_get().unwrap(), fresh);
        assert_eq!(e.fake.reads.load(Ordering::SeqCst), reads);
    }

    #[tokio::test]
    async fn preview_with_a_duplicate_mac_is_a_conflict_state() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        {
            let mut rows = e.fake.rows.lock().unwrap();
            let leases = rows
                .get_mut("Hub-1")
                .unwrap()
                .get_mut("/ip/dhcp-server/lease")
                .unwrap();
            let mac = leases[0]["active-mac-address"].clone();
            leases[1].insert("active-mac-address".into(), mac);
        }
        let v = r.api_preview_refresh().await.unwrap();
        assert_eq!(v["state"], json!("conflicts"), "{v}");
        let conflicts = v["conflicts"].as_array().unwrap();
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.iter().all(|c| c["kind"] == json!("identity")));
        assert!(v["nodes"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_failed_router_read_keeps_the_old_preview() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        let old = r.api_preview_refresh().await.unwrap();
        e.fake.failing.lock().unwrap().insert("Hub-1".into());
        assert!(r.api_preview_refresh().await.is_err());
        assert_eq!(r.api_preview_get().unwrap(), old);
    }

    #[tokio::test]
    async fn preview_without_policy_is_invalid() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let err = r.api_preview_refresh().await.unwrap_err();
        assert!(
            err.chain()
                .any(|c| c.is::<crate::topology_manager::store::InvalidPolicy>()),
            "{err:#}"
        );
    }

    async fn call(r: &mut Runner<FakeConnector>, op: &str) -> Value {
        let req = crate::api::protocol::Request {
            v: 1,
            id: "p".into(),
            op: op.into(),
            args: json!({}),
        };
        let line = crate::api::dispatch::handle(r, &req).await.unwrap();
        serde_json::from_str::<Value>(line.trim_end()).unwrap()
    }

    #[tokio::test]
    async fn preview_operations_are_dispatched() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        assert_eq!(
            call(&mut r, "topology.preview.get").await["result"],
            Value::Null
        );
        let fresh = call(&mut r, "topology.preview.refresh").await;
        assert_eq!(fresh["result"]["state"], json!("ok"), "{fresh}");
        assert_eq!(
            call(&mut r, "topology.preview.get").await["result"],
            fresh["result"]
        );
        assert!(call(&mut r, "topology.policies").await["result"]["profiles"].is_array());
    }

    #[tokio::test]
    async fn policies_show_defaults_and_each_profile() {
        let e = env();
        let mut r = shadow_with_policy(&e).await;
        let v = r.api_policies().unwrap();
        assert!(v["defaults"].is_object(), "{v}");
        let profiles = v["profiles"].as_array().unwrap();
        // Gold, Silver, LAN, IOT, and EMPTY. The empty node is left out of the network only.
        assert_eq!(profiles.len(), 5);
        assert!(
            profiles
                .iter()
                .all(|p| p["base"] == json!("source_read") && p["min_mbps"].is_null())
        );
    }
}

mod plans {
    use super::super::enrollment::cli_flow::{AGE, confirm, setup};
    use super::*;
    use crate::topology_manager::model::IdentityRef;
    use crate::topology_manager::store::{InvalidPolicy, RevisionConflict};
    use serde_json::{Value, json};

    async fn enrolled(e: &Env) -> Runner<FakeConnector> {
        let mut r = runner(e, setup(e, None));
        let d = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&d).unwrap();
        let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
        confirm(&mut file);
        assert_eq!(
            r.enroll_with_mapping(&file, AGE, true, false)
                .await
                .unwrap()
                .revision,
            Some(2)
        );
        r.lock().unwrap();
        r
    }

    fn rule(e: &anyhow::Error) -> String {
        e.chain()
            .find_map(|c| c.downcast_ref::<InvalidPolicy>())
            .map_or_else(|| format!("not a rule error: {e:#}"), |i| i.0.clone())
    }

    fn registry_identity(r: &mut Runner<FakeConnector>) -> IdentityRef {
        r.api_registry().unwrap().unwrap().subscribers[0].identities[0].clone()
    }

    /// A new bound lease with a MAC and an address that no record has.
    fn add_lease(e: &Env) -> String {
        let mac = "02:00:00:00:09:99";
        let mut rows = e.fake.rows.lock().unwrap();
        let leases = rows
            .get_mut("Hub-1")
            .unwrap()
            .get_mut("/ip/dhcp-server/lease")
            .unwrap();
        let mut lease = leases[0].clone();
        lease.insert(".id".into(), "*l99".into());
        for (k, v) in lease.iter_mut() {
            if k.ends_with("mac-address") {
                *v = mac.into();
            } else if k.ends_with("address") {
                let (net, _) = v.rsplit_once('.').unwrap();
                *v = format!("{net}.199");
            }
        }
        leases.push(lease);
        format!("dhcp-mac/v1/{mac}")
    }

    fn preview_identity(preview: &Value, account_ref: &str) -> Value {
        preview["subscribers"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["identities"].as_array().unwrap().iter())
            .find(|i| i["account_ref"] == json!(account_ref))
            .unwrap_or_else(|| panic!("{account_ref} not in the preview: {preview}"))
            .clone()
    }

    async fn web(r: &mut Runner<FakeConnector>, op: &str, args: Value) -> Value {
        let req = crate::api::protocol::Request {
            v: 1,
            id: "w".into(),
            op: op.into(),
            args,
        };
        let line = crate::api::dispatch::handle(r, &req).await.unwrap();
        serde_json::from_str::<Value>(line.trim_end()).unwrap()
    }

    const KEY1: &str = "0f8b5a3c-1d2e-4f60-9a7b-000000000001";
    const KEY2: &str = "0f8b5a3c-1d2e-4f60-9a7b-000000000002";

    fn create(revision: u64, name: &str) -> Value {
        json!({"command": "plan_create", "expected_revision": revision, "name": name,
               "down_mbps": 100, "up_mbps": 50})
    }

    #[tokio::test]
    async fn a_web_mutation_commits_once_per_request_key() {
        let e = env();
        let mut r = enrolled(&e).await;
        let args = json!({"key": KEY1, "actor": "alice", "command": create(2, "Web 100")});
        let first = web(&mut r, "topology.web.mutate", args.clone()).await;
        assert_eq!(first["ok"], true, "{first}");
        let receipt = &first["result"];
        assert_eq!(receipt["state"], "saved");
        assert_eq!(receipt["base_revision"], "2");
        assert_eq!(receipt["revision"], "3");
        assert_eq!(receipt["subject_id"], "plan-1");
        // A retry after a lost reply gets the same receipt and makes no second commit.
        let again = web(&mut r, "topology.web.mutate", args).await;
        assert_eq!(again["result"], *receipt);
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 3);
        // The same key with another payload is a conflict.
        let reused = web(
            &mut r,
            "topology.web.mutate",
            json!({"key": KEY1, "actor": "alice", "command": create(3, "Other")}),
        )
        .await;
        assert_eq!(reused["error"]["code"], "conflict", "{reused}");
        // The receipt lookup is per actor.
        let found = web(
            &mut r,
            "topology.web.operation",
            json!({"key": KEY1, "actor": "alice"}),
        )
        .await;
        assert_eq!(found["result"], *receipt);
        // The view lists the saved change at once, as saved.
        let history = web_view(&e).history;
        assert_eq!(
            (history[0].state.as_str(), history[0].change.as_str()),
            ("saved", "Create plan Web 100")
        );
        let foreign = web(
            &mut r,
            "topology.web.operation",
            json!({"key": KEY1, "actor": "bob"}),
        )
        .await;
        assert_eq!(foreign["result"], Value::Null);
        // A stale revision gives a durable failed receipt and no commit.
        let stale = web(
            &mut r,
            "topology.web.mutate",
            json!({"key": KEY2, "actor": "alice", "command": create(2, "Late")}),
        )
        .await;
        assert_eq!(stale["result"]["state"], "failed", "{stale}");
        assert_eq!(stale["result"]["error"], "conflict");
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 3);
        // An unknown command is not an operation.
        let unknown = web(
            &mut r,
            "topology.web.mutate",
            json!({"key": KEY2, "actor": "alice", "command": {"command": "policy_commit"}}),
        )
        .await;
        assert_eq!(unknown["error"]["code"], "bad_request", "{unknown}");
    }

    fn web_view(e: &Env) -> web_contract::topology::TopologyView {
        let path = e.state.join(web_contract::topology::CACHE);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn a_commit_and_a_refresh_publish_the_web_view_at_once() {
        let e = env();
        let mut r = enrolled(&e).await;
        r.api_plan_create(2, "Web 100", 100, 50).unwrap();
        let view = web_view(&e);
        assert_eq!(view.registry_revision.as_deref(), Some("3"));
        assert!(view.plans.iter().any(|p| p.name == "Web 100"));
        r.api_preview_refresh().await.unwrap();
        let observed = web_view(&e).observed.unwrap();
        assert_eq!((observed.revision.as_str(), observed.stale), ("3", false));
    }

    #[tokio::test]
    async fn an_open_intent_resolves_when_the_service_starts_again() {
        let e = env();
        let r = enrolled(&e).await;
        // A crash after the intent and before the commit leaves this record.
        let receipts = crate::topology_manager::web_receipts::Receipts::open(&e.state).unwrap();
        let command: web_contract::topology::WebCommand =
            serde_json::from_value(create(2, "Lost")).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        receipts.begin(KEY1, "alice", &command, 2, now).unwrap();
        drop(r);
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let found = web(
            &mut r,
            "topology.web.operation",
            json!({"key": KEY1, "actor": "alice"}),
        )
        .await;
        assert_eq!(found["result"]["state"], "not_committed", "{found}");
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 2);
    }

    #[tokio::test]
    async fn plan_operations_commit_and_return_the_new_revision() {
        let e = env();
        let mut r = enrolled(&e).await;
        let v = r.api_plan_create(2, "100M", 100, 100).unwrap();
        assert_eq!(v, json!({"revision": 3, "plan_id": "plan-1"}));
        let v = r
            .api_plan_update(3, "plan-1", Some("120M"), Some(120), None)
            .unwrap();
        assert_eq!(v, json!({"revision": 4}));
        let identity = registry_identity(&mut r);
        let v = r
            .api_subscriber_plan(4, identity.clone(), Some("plan-1".into()))
            .unwrap();
        assert_eq!(v["revision"], json!(5));
        let doc = r.api_registry().unwrap().unwrap();
        assert_eq!(v["subscriber_id"], json!(doc.subscribers[0].id));
        assert_eq!(
            doc.subscribers[0].policy.profile_id.as_deref(),
            Some("plan-1")
        );
        let e2 = r.api_plan_delete(5, "plan-1").unwrap_err();
        assert_eq!(rule(&e2), "the plan is in use");
        r.api_subscriber_plan(5, identity, None).unwrap();
        assert_eq!(
            r.api_plan_delete(6, "plan-1").unwrap(),
            json!({"revision": 7})
        );
    }

    #[tokio::test]
    async fn a_stale_revision_gives_conflict() {
        let e = env();
        let mut r = enrolled(&e).await;
        r.api_plan_create(2, "100M", 100, 100).unwrap();
        r.api_plan_create(3, "50M", 50, 50).unwrap();
        let identity = registry_identity(&mut r);
        // Two operators read revision 4 and attach different plans.
        r.api_subscriber_plan(4, identity.clone(), Some("plan-1".into()))
            .unwrap();
        let e2 = r
            .api_subscriber_plan(4, identity, Some("plan-2".into()))
            .unwrap_err();
        assert!(e2.chain().any(|c| c.is::<RevisionConflict>()), "{e2:#}");
        let doc = r.api_registry().unwrap().unwrap();
        assert_eq!(doc.revision, 5);
        assert_eq!(
            doc.subscribers[0].policy.profile_id.as_deref(),
            Some("plan-1")
        );
        let e3 = r.api_plan_create(4, "10M", 10, 10).unwrap_err();
        assert!(e3.chain().any(|c| c.is::<RevisionConflict>()), "{e3:#}");
    }

    #[tokio::test]
    async fn subscriber_plan_creates_a_record_for_a_new_account_seen_in_the_preview() {
        let e = env();
        let mut r = enrolled(&e).await;
        r.api_plan_create(2, "100M", 100, 100).unwrap();
        let account = add_lease(&e);
        let preview = r.api_preview_refresh().await.unwrap();
        assert_eq!(preview["state"], json!("ok"), "{preview}");
        let identity: IdentityRef = {
            let i = preview_identity(&preview, &account);
            serde_json::from_value(json!({
                "scope": {"router_id": i["router_id"], "source_id": i["source_id"], "kind": i["kind"]},
                "account_ref": i["account_ref"],
            }))
            .unwrap()
        };
        let before = r.api_registry().unwrap().unwrap();
        assert!(
            !before
                .subscribers
                .iter()
                .any(|s| s.identities.contains(&identity))
        );
        let v = r
            .api_subscriber_plan(3, identity.clone(), Some("plan-1".into()))
            .unwrap();
        assert_eq!(
            v,
            json!({"revision": 4, "subscriber_id": format!("tm-sub-{:016x}", before.next_id)})
        );
        let after = r.api_registry().unwrap().unwrap();
        assert_eq!(after.next_id, before.next_id + 1);
        let record = after
            .subscribers
            .iter()
            .find(|s| s.identities == vec![identity.clone()])
            .unwrap();
        assert_eq!(record.policy.profile_id.as_deref(), Some("plan-1"));
        assert_eq!(
            (
                record.policy.pin.as_ref(),
                record.policy.assignment.as_ref()
            ),
            (None, None)
        );
    }

    #[tokio::test]
    async fn a_stale_preview_gives_conflict_for_a_new_account() {
        let e = env();
        let mut r = enrolled(&e).await;
        let account = add_lease(&e);
        let preview = r.api_preview_refresh().await.unwrap();
        let i = preview_identity(&preview, &account);
        let identity: IdentityRef = serde_json::from_value(json!({
            "scope": {"router_id": i["router_id"], "source_id": i["source_id"], "kind": i["kind"]},
            "account_ref": i["account_ref"],
        }))
        .unwrap();
        r.api_plan_create(2, "100M", 100, 100).unwrap();
        let e2 = r
            .api_subscriber_plan(3, identity.clone(), Some("plan-1".into()))
            .unwrap_err();
        assert!(e2.chain().any(|c| c.is::<RevisionConflict>()), "{e2:#}");
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 3);
        // A fresh preview at revision 3 makes the same request work.
        r.api_preview_refresh().await.unwrap();
        r.api_subscriber_plan(3, identity, Some("plan-1".into()))
            .unwrap();
    }

    #[tokio::test]
    async fn edits_before_enrollment_give_invalid() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = r.propose_policy(AGE).await.unwrap();
        r.api_policy_commit(0, d.catalog, d.source_settings)
            .unwrap();
        let e2 = r.api_plan_create(1, "100M", 100, 100).unwrap_err();
        assert_eq!(
            rule(&e2),
            "enroll first: the registry has no export ownership"
        );
        assert_eq!(r.api_registry().unwrap().unwrap().revision, 1);
    }

    #[tokio::test]
    async fn policies_list_plans_with_their_use_count() {
        let e = env();
        let mut r = enrolled(&e).await;
        assert_eq!(r.api_policies().unwrap()["plans"], json!([]));
        r.api_plan_create(2, "100M", 100, 50).unwrap();
        r.api_plan_create(3, "7M", 7, 7).unwrap();
        let identity = registry_identity(&mut r);
        r.api_subscriber_plan(4, identity, Some("plan-2".into()))
            .unwrap();
        let v = r.api_policies().unwrap();
        assert_eq!(
            v["plans"],
            json!([
                {"id": "plan-1", "name": "100M", "down_mbps": 100, "up_mbps": 50, "subscribers": 0},
                {"id": "plan-2", "name": "7M", "down_mbps": 7, "up_mbps": 7, "subscribers": 1},
            ])
        );
        // A plan is a profile too, so the profile list shows its resolved rates.
        assert!(
            v["profiles"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == json!("plan-1"))
        );
    }

    async fn call(r: &mut Runner<FakeConnector>, op: &str, args: Value) -> Value {
        let req = crate::api::protocol::Request {
            v: 1,
            id: "p".into(),
            op: op.into(),
            args,
        };
        let line = crate::api::dispatch::handle(r, &req).await.unwrap();
        serde_json::from_str::<Value>(line.trim_end()).unwrap()
    }

    #[tokio::test]
    async fn plan_operations_are_dispatched() {
        let e = env();
        let mut r = enrolled(&e).await;
        let v = call(
            &mut r,
            "plan.create",
            json!({"expected_revision": 2, "name": "100M", "down_mbps": 100, "up_mbps": 100}),
        )
        .await;
        assert_eq!(
            v["result"],
            json!({"revision": 3, "plan_id": "plan-1"}),
            "{v}"
        );
        let v = call(
            &mut r,
            "plan.update",
            json!({"expected_revision": 3, "plan_id": "plan-1", "up_mbps": 50}),
        )
        .await;
        assert_eq!(v["result"], json!({"revision": 4}), "{v}");
        let preview = r.api_preview_refresh().await.unwrap();
        let identity = preview["subscribers"][0]["identities"][0].clone();
        let v = call(
            &mut r,
            "subscriber.plan",
            json!({"expected_revision": 4, "identity": identity, "plan_id": "plan-1"}),
        )
        .await;
        assert_eq!(v["result"]["revision"], json!(5), "{v}");
        let v = call(
            &mut r,
            "plan.delete",
            json!({"expected_revision": 5, "plan_id": "plan-1"}),
        )
        .await;
        assert_eq!(v["error"]["code"], json!("invalid"), "{v}");
        let v = call(
            &mut r,
            "subscriber.plan",
            json!({"expected_revision": 5, "identity": identity, "plan_id": null}),
        )
        .await;
        assert_eq!(v["result"]["revision"], json!(6), "{v}");
        let v = call(
            &mut r,
            "plan.delete",
            json!({"expected_revision": 6, "plan_id": "plan-1"}),
        )
        .await;
        assert_eq!(v["result"], json!({"revision": 7}), "{v}");
        let v = call(
            &mut r,
            "plan.create",
            json!({"expected_revision": 7, "name": "x", "down_mbps": 1, "up_mbps": 1, "extra": 1}),
        )
        .await;
        assert_eq!(v["error"]["code"], json!("bad_request"), "{v}");
        let v = call(
            &mut r,
            "plan.create",
            json!({"expected_revision": 7, "name": "x", "down_mbps": 1.5, "up_mbps": 1}),
        )
        .await;
        assert_eq!(v["error"]["code"], json!("bad_request"), "{v}");
    }
}
mod wan {
    use super::*;
    use crate::api::dispatch::{LostLock, handle};
    use crate::api::protocol::Request;
    use serde_json::{Value, json};

    fn with_core(e: &Env) -> Runner<FakeConnector> {
        let mut c = cfg(e, "shadow", "");
        c.routers.extend(
            Config::from_toml_str(
                "[[routers]]\nname=\"Core\"\naddress=\"192.0.2.9\"\nusername=\"u\"\npassword_file=\"/x\"\n",
            )
            .unwrap()
            .routers,
        );
        e.fake.rows.lock().unwrap().insert(
            "R1".into(),
            BTreeMap::from([(
                "/interface".to_string(),
                vec![
                    row(&[("name", "ether1"), ("type", "ether"), ("running", "true")]),
                    row(&[
                        ("name", "pppoe-out1"),
                        ("type", "pppoe-out"),
                        ("running", "true"),
                    ]),
                ],
            )]),
        );
        let mut r = runner(e, c);
        r.lock().unwrap();
        r
    }

    #[tokio::test]
    async fn wan_discover_lists_candidates_and_marks_failed_routers() {
        let e = env();
        let mut r = with_core(&e);
        e.fake.failing.lock().unwrap().insert("Core".into());
        let v = r.api_wan_discover(&[]).await.unwrap();
        let (r1, core) = (&v["routers"][0], &v["routers"][1]);
        assert_eq!((&r1["name"], &r1["state"]), (&json!("R1"), &json!("ok")));
        assert_eq!(r1["candidates"][0]["name"], json!("pppoe-out1"));
        assert_eq!(r1["candidates"][0]["suggested"], json!(true));
        assert_eq!(
            (&core["name"], &core["state"]),
            (&json!("Core"), &json!("failed"))
        );
        assert!(core["error"].as_str().unwrap().contains("down"), "{core}");
    }

    #[tokio::test]
    async fn wan_discover_changes_no_registry() {
        let e = env();
        let mut r = with_core(&e);
        r.api_wan_discover(&[]).await.unwrap();
        assert!(!e.state.join("topology-manager").exists());
    }

    #[tokio::test(start_paused = true)]
    async fn a_stalled_router_fails_alone_within_the_limit() {
        let e = env();
        let mut r = with_core(&e);
        e.fake.stalling.lock().unwrap().insert("Core".into());
        let started = tokio::time::Instant::now();
        let v = r.api_wan_discover(&[]).await.unwrap();
        assert!(started.elapsed() <= std::time::Duration::from_secs(31));
        assert_eq!(v["routers"][0]["state"], json!("ok"));
        assert_eq!(v["routers"][1]["state"], json!("failed"));
        assert_eq!(v["routers"][1]["error"], json!("read expired"));
    }

    #[tokio::test]
    async fn wan_discover_is_dispatched() {
        let e = env();
        let mut r = with_core(&e);
        let req = Request {
            v: 1,
            id: "t".into(),
            op: "wan.discover".into(),
            args: json!({}),
        };
        let line = handle(&mut r, &req)
            .await
            .unwrap_or_else(|LostLock(l)| panic!("lost lock: {l}"));
        let v: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["ok"], json!(true), "{v}");
        assert_eq!(v["result"]["routers"].as_array().unwrap().len(), 2);
    }
}
mod edits {
    use super::super::enrollment::cli_flow::{AGE, confirm, setup};
    use super::*;
    use crate::api::dispatch::{LostLock, handle};
    use crate::api::protocol::Request;
    use crate::topology_manager::edits::Edit;
    use crate::topology_manager::store::InvalidPolicy;
    use serde_json::{Value, json};

    async fn enrolled(e: &Env) -> Runner<FakeConnector> {
        let mut r = runner(e, setup(e, None));
        let d = r.propose_policy(AGE).await.unwrap();
        r.install_topology_policy(&d).unwrap();
        let mut file = r.propose_enrollment(None, AGE, false).await.unwrap();
        confirm(&mut file);
        r.enroll_with_mapping(&file, AGE, true, false)
            .await
            .unwrap();
        r.lock().unwrap();
        r
    }

    fn rule(e: &anyhow::Error) -> String {
        e.chain()
            .find_map(|c| c.downcast_ref::<InvalidPolicy>())
            .map_or_else(|| format!("not a rule error: {e:#}"), |i| i.0.clone())
    }

    fn edit(v: Value) -> Edit {
        serde_json::from_value(v).unwrap()
    }

    fn revision(r: &mut Runner<FakeConnector>) -> u64 {
        r.api_registry().unwrap().unwrap().revision
    }

    /// Discover two `pppoe-out` WANs on Hub-1 and commit them as WAN1 and WAN2.
    async fn two_wans(e: &Env, r: &mut Runner<FakeConnector>) -> (String, String) {
        e.fake
            .rows
            .lock()
            .unwrap()
            .get_mut("Hub-1")
            .unwrap()
            .insert(
                "/interface".into(),
                vec![
                    row(&[
                        ("name", "pppoe-out1"),
                        ("type", "pppoe-out"),
                        ("running", "true"),
                    ]),
                    row(&[
                        ("name", "pppoe-out2"),
                        ("type", "pppoe-out"),
                        ("running", "true"),
                    ]),
                ],
            );
        let discovery = r.api_wan_discover(&[]).await.unwrap();
        let router_id = discovery["routers"][0]["router_id"]
            .as_str()
            .unwrap()
            .to_string();
        let mut ids = Vec::new();
        for (i, endpoint) in ["pppoe-out1", "pppoe-out2"].into_iter().enumerate() {
            let rev = revision(r);
            let before: Vec<String> = tree_ids(r);
            let p = r
                .api_edit_propose(
                    rev,
                    edit(json!({"kind": "wan.create", "router_id": router_id, "endpoint_ref": endpoint,
                                "name": format!("WAN{}", i + 1), "down_mbps": 1000, "up_mbps": 1000})),
                )
                .await
                .unwrap();
            r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
                .unwrap();
            ids.push(
                tree_ids(r)
                    .into_iter()
                    .find(|id| !before.contains(id))
                    .unwrap(),
            );
        }
        (ids[0].clone(), ids[1].clone())
    }

    fn tree_ids(r: &mut Runner<FakeConnector>) -> Vec<String> {
        r.api_tree().unwrap()["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect()
    }

    /// The group node and source with the most registry subscribers.
    fn biggest_group(r: &mut Runner<FakeConnector>) -> (String, String) {
        let doc = r.api_registry().unwrap().unwrap();
        let tree = r.api_tree().unwrap();
        let count = |id: &str| {
            doc.subscribers
                .iter()
                .filter(|s| s.policy.assignment.as_deref() == Some(id))
                .count()
        };
        let node = tree["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == "group")
            .max_by_key(|n| count(n["id"].as_str().unwrap()))
            .unwrap();
        (
            node["id"].as_str().unwrap().to_string(),
            node["source_id"].as_str().unwrap().to_string(),
        )
    }

    async fn propose(r: &mut Runner<FakeConnector>, v: Value) -> anyhow::Result<Value> {
        let rev = revision(r);
        r.api_edit_propose(rev, edit(v)).await
    }

    async fn commit(r: &mut Runner<FakeConnector>, v: Value) -> Value {
        let p = propose(r, v).await.unwrap();
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        p
    }

    /// WAN1 and WAN2 with the biggest group below WAN1 and a second instance below WAN2.
    async fn split(e: &Env, r: &mut Runner<FakeConnector>) -> (String, String, Value) {
        let (wan1, wan2) = two_wans(e, r).await;
        let (group, source) = biggest_group(r);
        commit(
            r,
            json!({"kind": "node.move", "node_id": group, "parent": wan1}),
        )
        .await;
        let before = tree_ids(r);
        let p = commit(
            r,
            json!({"kind": "instance.add", "source_id": source, "parent": wan2}),
        )
        .await;
        let second = tree_ids(r)
            .into_iter()
            .find(|id| !before.contains(id))
            .unwrap();
        (group, second, p)
    }

    #[tokio::test]
    async fn propose_then_confirm_commits_the_kept_candidate() {
        let e = env();
        let mut r = enrolled(&e).await;
        let rev = revision(&mut r);
        let p = propose(
            &mut r,
            json!({"kind": "node.create", "name": "Tower", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}),
        )
        .await
        .unwrap();
        assert_eq!(revision(&mut r), rev, "propose commits nothing");
        assert!(
            p["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["name"] == "Tower")
        );
        assert_eq!(p["moves"], json!([]));
        let v = r
            .api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        assert_eq!(v, json!({"revision": rev + 1}));
        let doc = r.api_registry().unwrap().unwrap();
        let catalog = doc.catalog.unwrap();
        assert!(catalog.nodes.iter().any(|n| n.name == "Tower"));
        let again = r
            .api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap_err();
        assert!(format!("{again:#}").contains("the proposal is stale, propose again"));
    }

    #[tokio::test]
    async fn an_instance_below_a_second_wan_moves_half_of_the_group() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, p) = split(&e, &mut r).await;
        assert!(!p["moves"].as_array().unwrap().is_empty(), "{p}");
        let doc = r.api_registry().unwrap().unwrap();
        let on = |id: &str| {
            doc.subscribers
                .iter()
                .filter(|s| s.policy.assignment.as_deref() == Some(id))
                .count()
        };
        let (a, b) = (on(&first), on(&second));
        assert!(a > 0 && b > 0 && a.abs_diff(b) <= 1, "{a} and {b}");
    }

    #[tokio::test]
    async fn a_remove_moves_online_members_and_clears_offline_ones() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        // One more record assigned to the second instance that is offline now.
        let mut doc = r.api_registry().unwrap().unwrap();
        let mut offline = doc
            .subscribers
            .iter()
            .find(|s| s.policy.assignment.as_deref() == Some(first.as_str()))
            .unwrap()
            .clone();
        offline.id = "tm-sub-00000000000fffff".into();
        offline.identities[0].account_ref = "offline-account".into();
        offline.policy.assignment = Some(second.clone());
        offline.policy.pin = None;
        doc.subscribers.push(offline);
        doc.next_id = doc.next_id.max(0x10_0000);
        r.test_commit(doc).unwrap();
        let p = commit(&mut r, json!({"kind": "node.remove", "node_id": second})).await;
        assert_eq!(p["cleared"], json!(["tm-sub-00000000000fffff"]));
        let doc = r.api_registry().unwrap().unwrap();
        assert!(
            doc.subscribers
                .iter()
                .all(|s| s.policy.assignment.as_deref() != Some(second.as_str()))
        );
        assert!(
            doc.subscribers
                .iter()
                .any(|s| s.policy.assignment.as_deref() == Some(first.as_str()))
        );
        let offline = doc
            .subscribers
            .iter()
            .find(|s| s.id == "tm-sub-00000000000fffff")
            .unwrap();
        assert_eq!(offline.policy.assignment, None);
    }

    #[tokio::test]
    async fn confirm_after_another_commit_gives_conflict_and_commits_nothing() {
        let e = env();
        let mut r = enrolled(&e).await;
        let a = propose(
            &mut r,
            json!({"kind": "node.create", "name": "A", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}),
        )
        .await
        .unwrap();
        // Operator B commits through another operation.
        let rev = revision(&mut r);
        r.api_plan_create(rev, "100M", 100, 100).unwrap();
        let err = r
            .api_edit_confirm(a["proposal_id"].as_str().unwrap())
            .unwrap_err();
        assert!(
            err.chain()
                .any(|c| c.is::<crate::topology_manager::store::RevisionConflict>())
        );
        let doc = r.api_registry().unwrap().unwrap();
        assert!(doc.catalog.unwrap().nodes.iter().all(|n| n.name != "A"));
    }

    #[tokio::test(start_paused = true)]
    async fn confirm_after_expiry_gives_conflict() {
        let e = env();
        let mut r = enrolled(&e).await;
        let p = propose(
            &mut r,
            json!({"kind": "node.create", "name": "A", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}),
        )
        .await
        .unwrap();
        tokio::time::advance(std::time::Duration::from_secs(121)).await;
        let err = r
            .api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap_err();
        assert!(
            err.chain()
                .any(|c| c.is::<crate::topology_manager::store::RevisionConflict>())
        );
    }

    #[tokio::test]
    async fn a_new_propose_replaces_the_old_one_and_cancel_drops_it() {
        let e = env();
        let mut r = enrolled(&e).await;
        let node = |name: &str| json!({"kind": "node.create", "name": name, "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}});
        let a = propose(&mut r, node("A")).await.unwrap();
        let b = propose(&mut r, node("B")).await.unwrap();
        assert!(
            r.api_edit_confirm(a["proposal_id"].as_str().unwrap())
                .is_err()
        );
        r.api_edit_confirm(b["proposal_id"].as_str().unwrap())
            .unwrap();
        let b = propose(&mut r, node("C")).await.unwrap();
        assert_eq!(
            r.api_edit_cancel(b["proposal_id"].as_str().unwrap())
                .unwrap(),
            json!({})
        );
        assert!(
            r.api_edit_confirm(b["proposal_id"].as_str().unwrap())
                .is_err()
        );
    }

    #[tokio::test]
    async fn minimums_above_a_wan_ceiling_give_invalid_and_keep_no_proposal() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (wan1, _) = two_wans(&e, &mut r).await;
        let (group, _) = biggest_group(&mut r);
        // A WAN of 1 Mbps cannot hold the group minimum and plans.
        commit(
            &mut r,
            json!({"kind": "node.update", "node_id": wan1, "capacity": {"mode": "fixed", "down_mbps": 1, "up_mbps": 1}}),
        )
        .await;
        let mut doc = r.api_registry().unwrap().unwrap();
        let node = doc
            .catalog
            .as_mut()
            .unwrap()
            .nodes
            .iter_mut()
            .find(|n| n.id == group)
            .unwrap();
        node.minimum.down = Some(50_000_000);
        r.test_commit(doc).unwrap();
        let err = propose(
            &mut r,
            json!({"kind": "node.move", "node_id": group, "parent": wan1}),
        )
        .await
        .unwrap_err();
        assert!(rule(&err).contains("Minimum"), "{}", rule(&err));
        assert!(r.api_edit_confirm("any").is_err());
    }

    #[tokio::test]
    async fn a_down_distro_blocks_only_edits_with_affected_sources() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (wan1, _) = two_wans(&e, &mut r).await;
        let (group, _) = biggest_group(&mut r);
        e.fake.failing.lock().unwrap().insert("Hub-1".into());
        assert!(
            propose(
                &mut r,
                json!({"kind": "node.create", "name": "T", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}),
            )
            .await
            .is_ok()
        );
        assert!(
            propose(
                &mut r,
                json!({"kind": "node.move", "node_id": group, "parent": wan1})
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn tree_and_edit_operations_are_dispatched() {
        let e = env();
        let mut r = enrolled(&e).await;
        let mut call = async |op: &str, args: Value| {
            let req = Request {
                v: 1,
                id: "t".into(),
                op: op.into(),
                args,
            };
            let line = handle(&mut r, &req)
                .await
                .unwrap_or_else(|LostLock(l)| panic!("lost lock: {l}"));
            serde_json::from_str::<Value>(line.trim_end()).unwrap()
        };
        let tree = call("topology.tree", json!({})).await;
        assert_eq!(tree["ok"], json!(true), "{tree}");
        let rev = tree["result"]["revision"].clone();
        let p = call(
            "topology.edit.propose",
            json!({"expected_revision": rev, "edit": {"kind": "node.create", "name": "T", "parent": null, "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}}),
        )
        .await;
        assert_eq!(p["ok"], json!(true), "{p}");
        let id = p["result"]["proposal_id"].clone();
        let c = call("topology.edit.cancel", json!({"proposal_id": id})).await;
        assert_eq!(c["ok"], json!(true), "{c}");
        let c = call("topology.edit.confirm", json!({"proposal_id": id})).await;
        assert_eq!(c["error"]["code"], json!("conflict"), "{c}");
        let bad = call(
            "topology.edit.propose",
            json!({"expected_revision": rev, "edit": {"kind": "nope"}}),
        )
        .await;
        assert_eq!(bad["error"]["code"], json!("bad_request"), "{bad}");
    }

    fn on(r: &mut Runner<FakeConnector>, id: &str) -> usize {
        r.api_registry()
            .unwrap()
            .unwrap()
            .subscribers
            .iter()
            .filter(|s| s.policy.assignment.as_deref() == Some(id))
            .count()
    }

    fn identity_of(s: &crate::topology_manager::model::SubscriberRecord) -> Value {
        let i = &s.identities[0];
        json!({"router_id": i.scope.router_id, "source_id": i.scope.source_id,
               "kind": i.scope.kind, "account_ref": i.account_ref})
    }

    /// An offline record of the group, assigned to `node`, optionally pinned there.
    fn add_offline(r: &mut Runner<FakeConnector>, like: &str, node: &str, pinned: bool) -> String {
        let mut doc = r.api_registry().unwrap().unwrap();
        let mut offline = doc
            .subscribers
            .iter()
            .find(|s| s.policy.assignment.as_deref() == Some(like))
            .unwrap()
            .clone();
        offline.id = "tm-sub-00000000000fffff".into();
        offline.identities[0].account_ref = "offline-account".into();
        offline.policy.assignment = Some(node.into());
        offline.policy.pin = pinned.then(|| node.to_string());
        doc.subscribers.push(offline);
        doc.next_id = doc.next_id.max(0x10_0000);
        r.test_commit(doc).unwrap();
        "tm-sub-00000000000fffff".into()
    }

    #[tokio::test]
    async fn rebalance_returns_a_drifted_group_to_the_capacity_split() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        let mut doc = r.api_registry().unwrap().unwrap();
        for s in &mut doc.subscribers {
            if s.policy.assignment.as_deref() == Some(second.as_str()) {
                s.policy.assignment = Some(first.clone());
            }
        }
        r.test_commit(doc).unwrap();
        assert_eq!(on(&mut r, &second), 0, "the group drifted to one WAN");
        let (_, source) = biggest_group(&mut r);
        let p = commit(&mut r, json!({"kind": "rebalance", "source_id": source})).await;
        assert!(!p["moves"].as_array().unwrap().is_empty(), "{p}");
        let (a, b) = (on(&mut r, &first), on(&mut r, &second));
        assert!(a > 0 && b > 0 && a.abs_diff(b) <= 1, "{a} and {b}");
    }

    #[tokio::test]
    async fn rebalance_clears_offline_unpinned_assignments_of_the_group() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        let offline = add_offline(&mut r, &first, &second, false);
        let p = commit(&mut r, json!({"kind": "rebalance", "source_id": null})).await;
        assert_eq!(p["cleared"], json!([offline]));
        let doc = r.api_registry().unwrap().unwrap();
        let s = doc.subscribers.iter().find(|s| s.id == offline).unwrap();
        assert_eq!(s.policy.assignment, None);
    }

    #[tokio::test]
    async fn a_pinned_subscriber_does_not_move_and_its_demand_counts() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        let (wan1, wan2) = {
            let tree = r.api_tree().unwrap();
            let parent = |id: &str| {
                tree["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["id"] == id)
                    .unwrap()["parent"]
                    .as_str()
                    .unwrap()
                    .to_string()
            };
            (parent(&first), parent(&second))
        };
        let doc = r.api_registry().unwrap().unwrap();
        let target = doc
            .subscribers
            .iter()
            .find(|s| s.policy.assignment.as_deref() == Some(first.as_str()))
            .unwrap()
            .clone();
        commit(&mut r, json!({"kind": "subscriber.pin", "identity": identity_of(&target), "wan_node_id": wan2})).await;
        let pinned = |r: &mut Runner<FakeConnector>| {
            r.api_registry()
                .unwrap()
                .unwrap()
                .subscribers
                .into_iter()
                .find(|s| s.id == target.id)
                .unwrap()
        };
        assert_eq!(pinned(&mut r).policy.pin.as_deref(), Some(second.as_str()));
        commit(&mut r, json!({"kind": "rebalance", "source_id": null})).await;
        let after = pinned(&mut r);
        assert_eq!(
            (
                after.policy.pin.as_deref(),
                after.policy.assignment.as_deref()
            ),
            (Some(second.as_str()), Some(second.as_str()))
        );
        assert!(
            on(&mut r, &first) > 0,
            "the pin demand counts, so WAN1 keeps members"
        );
        let _ = wan1;
    }

    #[tokio::test]
    async fn a_pin_of_an_offline_subscriber_is_refused() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        let offline = add_offline(&mut r, &first, &first, false);
        let doc = r.api_registry().unwrap().unwrap();
        let record = doc
            .subscribers
            .iter()
            .find(|s| s.id == offline)
            .unwrap()
            .clone();
        let wan2 = r.api_tree().unwrap()["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == second.as_str())
            .unwrap()["parent"]
            .clone();
        let err = propose(&mut r, json!({"kind": "subscriber.pin", "identity": identity_of(&record), "wan_node_id": wan2}))
            .await
            .unwrap_err();
        assert_eq!(rule(&err), "the subscriber must be online to pin");
    }

    #[tokio::test]
    async fn unpin_reads_no_router_and_moves_nothing() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, _, _) = split(&e, &mut r).await;
        let offline = add_offline(&mut r, &first, &first, true);
        e.fake.failing.lock().unwrap().insert("Hub-1".into());
        let p = propose(
            &mut r,
            json!({"kind": "subscriber.pin", "subscriber_id": offline, "wan_node_id": null}),
        )
        .await
        .unwrap();
        assert_eq!(p["moves"], json!([]));
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        let doc = r.api_registry().unwrap().unwrap();
        let s = doc.subscribers.iter().find(|s| s.id == offline).unwrap();
        assert_eq!(
            (s.policy.pin.as_deref(), s.policy.assignment.as_deref()),
            (None, Some(first.as_str()))
        );
    }

    #[tokio::test]
    async fn an_offline_pin_that_blocks_remove_is_cleared_by_subscriber_id() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, second, _) = split(&e, &mut r).await;
        let offline = add_offline(&mut r, &first, &second, true);
        let err = propose(&mut r, json!({"kind": "node.remove", "node_id": second}))
            .await
            .unwrap_err();
        assert!(
            rule(&err).starts_with("unpin the subscribers first"),
            "{}",
            rule(&err)
        );
        commit(
            &mut r,
            json!({"kind": "subscriber.pin", "subscriber_id": offline, "wan_node_id": null}),
        )
        .await;
        commit(&mut r, json!({"kind": "node.remove", "node_id": second})).await;
        assert!(!tree_ids(&mut r).contains(&second));
    }

    #[tokio::test]
    async fn tree_pins_carry_the_online_state() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (first, _, _) = split(&e, &mut r).await;
        let offline = add_offline(&mut r, &first, &first, true);
        let doc = r.api_registry().unwrap().unwrap();
        let mut online_pin = doc.clone();
        let target = online_pin
            .subscribers
            .iter_mut()
            .find(|s| s.policy.assignment.as_deref() == Some(first.as_str()) && s.id != offline)
            .unwrap();
        target.policy.pin = Some(first.clone());
        let online_id = target.id.clone();
        r.test_commit(online_pin).unwrap();
        r.api_preview_refresh().await.unwrap();
        let pins = r.api_tree().unwrap()["pins"].clone();
        let state = |id: &str| {
            pins.as_array()
                .unwrap()
                .iter()
                .find(|p| p["subscriber_id"] == id)
                .unwrap()["online"]
                .clone()
        };
        assert_eq!(
            (state(&online_id), state(&offline)),
            (json!(true), json!(false))
        );
    }

    fn speed(min: Value, max: Value) -> Value {
        json!({"kind": "speed.defaults", "down_min": min, "up_min": min, "down_max": max, "up_max": max})
    }

    #[tokio::test]
    async fn a_minimum_change_lists_changed_rates_and_the_legacy_warning() {
        let e = env();
        let mut r = enrolled(&e).await;
        let p = propose(
            &mut r,
            speed(json!({"percent": 20}), json!({"percent": 100})),
        )
        .await
        .unwrap();
        assert!(p["rates_changed"].as_u64().unwrap() > 0, "{p}");
        let sample = &p["rates"][0];
        assert_ne!(
            sample["before"]["min_mbps"], sample["after"]["min_mbps"],
            "{p}"
        );
        assert!(
            p["warnings"][0]
                .as_str()
                .unwrap()
                .contains("legacy rounded rules"),
            "{p}"
        );
        assert_eq!(p["new_records"], json!(0));
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
    }

    #[tokio::test]
    async fn a_factor_change_lists_changed_group_capacities() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (group, source) = biggest_group(&mut r);
        let p = propose(
            &mut r,
            json!({"kind": "group.factor", "source_id": source, "down": "0.1", "up": "0.1"}),
        )
        .await
        .unwrap();
        let caps = p["capacities"].as_array().unwrap();
        assert!(caps.iter().any(|c| c["node_id"] == group.as_str()), "{p}");
        assert_eq!(p["rates_changed"], json!(0));
    }

    #[tokio::test]
    async fn a_plan_change_is_reviewed_before_it_commits() {
        let e = env();
        let mut r = enrolled(&e).await;
        let p = propose(
            &mut r,
            json!({"kind": "plan.create", "name": "Web 7", "down_mbps": 7, "up_mbps": 3}),
        )
        .await
        .unwrap();
        assert_eq!(revision(&mut r), 2, "a review commits nothing");
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        let doc = r.api_registry().unwrap().unwrap();
        let plan = doc
            .catalog
            .as_ref()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.name == "Web 7")
            .unwrap()
            .id
            .clone();
        // A subscriber plan addresses the stable ID and lists that subscriber's new rates.
        let sub = doc.subscribers[0].id.clone();
        let p = propose(
            &mut r,
            json!({"kind": "subscriber.plan", "subscriber_id": sub, "plan_id": plan}),
        )
        .await
        .unwrap();
        let rates = p["rates"].as_array().unwrap();
        assert!(
            rates.iter().any(|x| x["subscriber_id"] == sub.as_str()),
            "{p}"
        );
        r.api_edit_confirm(p["proposal_id"].as_str().unwrap())
            .unwrap();
        let doc = r.api_registry().unwrap().unwrap();
        let record = doc.subscribers.iter().find(|s| s.id == sub).unwrap();
        assert_eq!(record.policy.profile_id.as_deref(), Some(plan.as_str()));
        // A plan in use stays; an unknown subscriber is refused.
        let used = propose(&mut r, json!({"kind": "plan.delete", "plan_id": plan}))
            .await
            .unwrap_err();
        assert_eq!(rule(&used), "the plan is in use");
        let unknown = propose(
            &mut r,
            json!({"kind": "subscriber.plan", "subscriber_id": "tm-sub-ffff", "plan_id": null}),
        )
        .await
        .unwrap_err();
        assert_eq!(rule(&unknown), "unknown subscriber");
    }

    #[tokio::test]
    async fn a_fixed_minimum_above_a_plan_is_refused_with_the_subscribers() {
        let e = env();
        let mut r = enrolled(&e).await;
        let err = propose(&mut r, speed(json!({"mbps": 999}), json!({"percent": 100})))
            .await
            .unwrap_err();
        assert!(
            rule(&err).starts_with("the minimum is above the maximum for: tm-sub-"),
            "{}",
            rule(&err)
        );
    }

    #[tokio::test]
    async fn a_minimum_above_a_wan_ceiling_is_refused_and_keeps_no_proposal() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (wan1, _) = two_wans(&e, &mut r).await;
        let (group, _) = biggest_group(&mut r);
        commit(
            &mut r,
            json!({"kind": "node.move", "node_id": group, "parent": wan1}),
        )
        .await;
        let err = propose(&mut r, speed(json!({"mbps": 500}), json!({"mbps": 1000})))
            .await
            .unwrap_err();
        assert!(
            rule(&err).contains("MinimumExceedsCeiling"),
            "{}",
            rule(&err)
        );
        assert!(r.api_edit_confirm("any").is_err());
    }

    #[tokio::test]
    async fn a_lower_minimum_can_fix_an_existing_minimum_conflict() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (wan1, _) = two_wans(&e, &mut r).await;
        let (group, _) = biggest_group(&mut r);
        commit(
            &mut r,
            json!({"kind": "node.move", "node_id": group, "parent": wan1}),
        )
        .await;
        // Seed a registry that already breaks the WAN ceiling: high minimums, a small WAN.
        let mut doc = r.api_registry().unwrap().unwrap();
        let c = doc.catalog.as_mut().unwrap();
        c.defaults.down_min = crate::topology_manager::rates::RateRule::FixedBps(400_000_000);
        c.defaults.up_min = crate::topology_manager::rates::RateRule::FixedBps(400_000_000);
        c.defaults.down_max = crate::topology_manager::rates::RateRule::FixedBps(900_000_000);
        c.defaults.up_max = crate::topology_manager::rates::RateRule::FixedBps(900_000_000);
        for p in &mut c.profiles {
            p.speed = Default::default();
        }
        r.test_commit(doc).unwrap();
        let pv = r.api_preview_refresh().await.unwrap();
        assert_eq!(
            pv["state"],
            json!("conflicts"),
            "the seeded registry must already conflict: {pv}"
        );
        let fix = propose(
            &mut r,
            speed(json!({"percent": 1}), json!({"percent": 100})),
        )
        .await;
        assert!(fix.is_ok(), "{:#}", fix.unwrap_err());
    }

    #[tokio::test]
    async fn a_down_distro_refuses_a_speed_edit() {
        let e = env();
        let mut r = enrolled(&e).await;
        e.fake.failing.lock().unwrap().insert("Hub-1".into());
        assert!(
            propose(
                &mut r,
                speed(json!({"percent": 20}), json!({"percent": 100}))
            )
            .await
            .is_err()
        );
        assert!(r.api_edit_confirm("any").is_err());
    }

    #[tokio::test]
    async fn policies_list_groups_with_factor_backstop_and_floor() {
        let e = env();
        let mut r = enrolled(&e).await;
        let v = r.api_policies().unwrap();
        let groups = v["groups"].as_array().unwrap();
        assert!(!groups.is_empty(), "{v}");
        let g = &groups[0];
        for key in [
            "source_id",
            "name",
            "factor",
            "formula",
            "backstop_mbps",
            "floor_mbps",
        ] {
            assert!(g.get(key).is_some(), "{key}: {g}");
        }
        assert!(g["factor"][0].is_string(), "{g}");
    }

    #[tokio::test]
    async fn an_old_placement_conflict_does_not_hide_a_new_aggregate_conflict() {
        let e = env();
        let mut r = enrolled(&e).await;
        let (wan1, _) = two_wans(&e, &mut r).await;
        let (group, _) = biggest_group(&mut r);
        commit(
            &mut r,
            json!({"kind": "node.move", "node_id": group, "parent": wan1}),
        )
        .await;
        // One subscriber pinned to a node of another group: an old placement conflict.
        let mut doc = r.api_registry().unwrap().unwrap();
        let other = doc
            .catalog
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|n| {
                n.source_id.is_some()
                    && n.id != group
                    && n.kind == crate::topology_manager::layout::NodeKind::SubscriberGroup
            })
            .unwrap()
            .id
            .clone();
        let stray = doc
            .subscribers
            .iter_mut()
            .find(|s| s.policy.assignment.as_deref() == Some(group.as_str()))
            .unwrap();
        stray.policy.pin = Some(other.clone());
        stray.policy.assignment = Some(other);
        r.test_commit(doc).unwrap();
        let pv = r.api_preview_refresh().await.unwrap();
        assert_eq!(
            pv["state"],
            json!("conflicts"),
            "the stray pin must conflict: {pv}"
        );
        let err = propose(&mut r, speed(json!({"mbps": 500}), json!({"mbps": 1000})))
            .await
            .unwrap_err();
        assert!(
            rule(&err).contains("MinimumExceedsCeiling"),
            "{}",
            rule(&err)
        );
    }

    /// A new router plan "Bronze" with one connected customer.
    fn add_bronze(e: &Env) {
        let mut rows = e.fake.rows.lock().unwrap();
        let hub = rows.get_mut("Hub-1").unwrap();
        let set = |row: &mut RosRow, k: &str, v: &str| {
            if row.contains_key(k) {
                row.insert(k.into(), v.into());
            }
        };
        let mut profile = hub["/ppp/profile"][0].clone();
        set(&mut profile, ".id", "*pB");
        set(&mut profile, "name", "Bronze");
        set(&mut profile, "rate-limit", "8M/8M");
        set(&mut profile, "comment", "");
        hub.get_mut("/ppp/profile").unwrap().push(profile);
        let mut secret = hub["/ppp/secret"][0].clone();
        set(&mut secret, ".id", "*sB");
        set(&mut secret, "name", "bronze-user");
        set(&mut secret, "profile", "Bronze");
        set(&mut secret, "comment", "");
        hub.get_mut("/ppp/secret").unwrap().push(secret);
        let mut active = hub["/ppp/active"][0].clone();
        set(&mut active, ".id", "*aB");
        set(&mut active, "name", "bronze-user");
        set(&mut active, "address", "100.64.9.99");
        set(&mut active, "caller-id", "02:00:00:00:0b:0b");
        set(&mut active, "comment", "");
        hub.get_mut("/ppp/active").unwrap().push(active);
    }

    async fn fallback_root(r: &mut Runner<FakeConnector>) -> String {
        let before = tree_ids(r);
        commit(
            r,
            json!({"kind": "node.create", "name": "Fallback", "parent": null, "node_kind": "group",
                   "capacity": {"mode": "fixed", "down_mbps": 100, "up_mbps": 100}}),
        )
        .await;
        let fb = tree_ids(r)
            .into_iter()
            .find(|id| !before.contains(id))
            .unwrap();
        commit(
            r,
            json!({"kind": "fallback.set", "node_ids": [fb], "default_down_mbps": 10, "default_up_mbps": 10}),
        )
        .await;
        fb
    }

    #[tokio::test]
    async fn candidates_list_a_new_ppp_profile_and_leave_out_known_sources() {
        let e = env();
        let mut r = enrolled(&e).await;
        add_bronze(&e);
        let v = r.api_source_candidates().await.unwrap();
        let names: Vec<&str> = v["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(names.iter().any(|n| n.contains("Bronze")), "{v}");
        assert!(!names.iter().any(|n| n.contains("Gold")), "{v}");
    }

    #[tokio::test]
    async fn source_add_moves_the_fallback_subscribers_to_the_new_group() {
        let e = env();
        let mut r = enrolled(&e).await;
        let fb = fallback_root(&mut r).await;
        add_bronze(&e);
        let pv = r.api_preview_refresh().await.unwrap();
        let bronze = pv["subscribers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["fallback"] == true)
            .unwrap_or_else(|| panic!("no fallback subscriber: {pv}"))
            .clone();
        assert_eq!(bronze["node"], json!(fb));
        let key = r.api_source_candidates().await.unwrap()["candidates"][0]["key"].clone();
        let p = commit(
            &mut r,
            json!({"kind": "source.add", "key": key, "parent": null}),
        )
        .await;
        assert_eq!(p["sources"].as_array().unwrap().len(), 1, "{p}");
        let doc = r.api_registry().unwrap().unwrap();
        let c = doc.catalog.as_ref().unwrap();
        let source = c
            .sources
            .iter()
            .find(|s| s.id == key.as_str().unwrap())
            .unwrap();
        let node = c
            .nodes
            .iter()
            .find(|n| n.source_id.as_deref() == Some(source.id.as_str()))
            .unwrap();
        let pv = r.api_preview_refresh().await.unwrap();
        let moved = pv["subscribers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == bronze["id"])
            .unwrap()
            .clone();
        assert_eq!(
            (moved["node"].as_str(), &moved["fallback"]),
            (Some(node.id.as_str()), &json!(false)),
            "{pv}"
        );
        let settings = doc.source_settings.as_ref().unwrap();
        assert!(
            settings.selections.iter().any(|s| s
                .policy
                .profiles
                .iter()
                .any(|b| b.current_name == "Bronze"))
        );
    }

    #[tokio::test]
    async fn a_vanished_candidate_is_refused() {
        let e = env();
        let mut r = enrolled(&e).await;
        let err = propose(
            &mut r,
            json!({"kind": "source.add", "key": "nope", "parent": null}),
        )
        .await
        .unwrap_err();
        assert_eq!(rule(&err), "the source is no longer on the router");
    }

    #[tokio::test]
    async fn fallback_clear_is_refused_with_online_fallback_subscribers() {
        let e = env();
        let mut r = enrolled(&e).await;
        fallback_root(&mut r).await;
        add_bronze(&e);
        let err = propose(&mut r, json!({"kind": "fallback.clear"}))
            .await
            .unwrap_err();
        assert_eq!(rule(&err), "move the fallback subscribers first");
    }

    #[tokio::test]
    async fn candidates_and_source_add_are_dispatched() {
        let e = env();
        let mut r = enrolled(&e).await;
        let req = Request {
            v: 1,
            id: "t".into(),
            op: "topology.sources.candidates".into(),
            args: json!({}),
        };
        let line = handle(&mut r, &req)
            .await
            .unwrap_or_else(|LostLock(l)| panic!("lost lock: {l}"));
        let v: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["ok"], json!(true), "{v}");
        assert!(v["result"]["candidates"].is_array());
    }
}
