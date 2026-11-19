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
        let mut file = r.propose_enrollment(None, AGE).await.unwrap();
        confirm(&mut file);
        assert_eq!(
            r.enroll_with_mapping(&file, AGE, true)
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
        let v = r.api_wan_discover().await.unwrap();
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
        r.api_wan_discover().await.unwrap();
        assert!(!e.state.join("topology-manager").exists());
    }

    #[tokio::test(start_paused = true)]
    async fn a_stalled_router_fails_alone_within_the_limit() {
        let e = env();
        let mut r = with_core(&e);
        e.fake.stalling.lock().unwrap().insert("Core".into());
        let started = tokio::time::Instant::now();
        let v = r.api_wan_discover().await.unwrap();
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
        let mut file = r.propose_enrollment(None, AGE).await.unwrap();
        confirm(&mut file);
        r.enroll_with_mapping(&file, AGE, true).await.unwrap();
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
        let discovery = r.api_wan_discover().await.unwrap();
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
}
