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

    fn call(r: &mut Runner<FakeConnector>, op: &str, args: Value) -> Value {
        let req = Request {
            v: 1,
            id: "t".into(),
            op: op.into(),
            args,
        };
        let line = handle(r, &req).unwrap_or_else(|LostLock(l)| panic!("lost lock: {l}"));
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
        let v = call(&mut r, "status", json!({}));
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
            call(&mut r, "registry.get", json!({}))["result"],
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
            call(&mut r, "status", json!({}))["result"]["registry_revision"],
            json!(1)
        );
        let doc = call(&mut r, "registry.get", json!({}));
        assert_eq!(doc["result"]["revision"], json!(1));
    }

    #[tokio::test]
    async fn policy_commit_succeeds_and_returns_the_new_revision() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        let d = draft(&mut r).await;
        let args = json!({"expected_revision": 0, "catalog": d["catalog"], "source_settings": d["source_settings"]});
        let v = call(&mut r, "policy.commit", args);
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
            call(&mut r, "policy.commit", args.clone())["ok"],
            json!(true)
        );
        let v = call(&mut r, "policy.commit", args);
        assert_eq!(v["error"]["code"], json!("conflict"), "{v}");
        assert_eq!(
            call(&mut r, "status", json!({}))["result"]["registry_revision"],
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
        let v = call(&mut r, "policy.commit", args);
        assert_eq!(v["error"]["code"], json!("invalid"), "{v}");
    }

    #[tokio::test]
    async fn unknown_op_and_bad_args_are_reported() {
        let e = env();
        let mut r = runner(&e, setup(&e, None));
        r.lock().unwrap();
        assert_eq!(
            call(&mut r, "nope", json!({}))["error"]["code"],
            json!("unknown_op")
        );
        let v = call(&mut r, "policy.commit", json!({"expected_revision": "x"}));
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
        let Err(LostLock(line)) = handle(&mut r, &req) else {
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
            let line = crate::api::dispatch::handle(&mut r, &job.request).unwrap_or_else(|l| l.0);
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
