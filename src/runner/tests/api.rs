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
