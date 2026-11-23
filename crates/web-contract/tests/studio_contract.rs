//! Topology Studio contract version 1: wire names, strict requests, and the shared fixture.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fmt::Debug;
use web_contract::routers::RouterEdit;
use web_contract::source_studio::*;
use web_contract::topology::{BuilderEdit, EditRequest, Operation};

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(wire: Value) -> T {
    let typed: T = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), wire);
    typed
}

fn policy() -> Value {
    json!({
        "id": null, "name": "PPPoE", "enabled": true, "priority": 10, "source": "ppp",
        "read_access_ref": "default", "members": "joined",
        "filter": {"kind": "match", "field": "account.profile", "op": "eq", "value": {"kind": "text", "value": "a"}},
        "group": {"kind": "by_field", "field": "account.profile"},
        "identity": "registry",
        "rates": [{"kind": "subscriber_override"}],
        "parent": {"kind": "router"},
        "capacity": {"kind": "fixed", "down_bps": "1000", "up_bps": "1000"},
        "writes": []
    })
}

fn connection() -> Value {
    json!({"address": "192.0.2.1", "port": 8729, "tls": true, "tls_verify": true,
           "username": "lqosync", "password": "s3cret", "ca_pem": null})
}

#[test]
fn old_operation_deserializes_without_source_result() {
    let op: Operation = serde_json::from_value(json!({
        "key": "k-0000000000000001", "state": "done", "receipt": null, "discovery": null
    }))
    .unwrap();
    assert_eq!(op.source_result, None);
    // A legacy operation keeps its old wire form.
    assert!(
        serde_json::to_value(&op)
            .unwrap()
            .get("source_result")
            .is_none()
    );
}

#[test]
fn studio_requests_reject_unknown_fields() {
    let job = json!({"kind": "collect", "router_id": "r", "expected_config_digest": "d", "sources": ["ppp"]});
    let cases: Vec<(&str, Value)> = vec![
        ("request", json!({"key": "k", "job": job, "extra": 1})),
        (
            "job",
            json!({"key": "k", "job": {"kind": "collect", "router_id": "r",
            "expected_config_digest": "d", "sources": [], "extra": 1}}),
        ),
        ("connection", {
            let mut c = connection();
            c["extra"] = json!(1);
            json!({"key": "k", "job": {"kind": "draft_test", "connection": c, "required_sources": []}})
        }),
        ("policy", {
            let mut p = policy();
            p["script"] = json!("/system script run x");
            json!({"key": "k", "job": {"kind": "evaluate", "router_id": "r", "expected_revision": "1",
                "expected_config_digest": "d", "snapshot_id": "s", "policies": [p]}})
        }),
        ("filter", {
            let mut p = policy();
            p["filter"]["path"] = json!("/ip/route");
            json!({"key": "k", "job": {"kind": "evaluate", "router_id": "r", "expected_revision": "1",
                "expected_config_digest": "d", "snapshot_id": "s", "policies": [p]}})
        }),
        (
            "unknown job",
            json!({"key": "k", "job": {"kind": "command", "path": "/system/reboot"}}),
        ),
    ];
    for (name, wire) in cases {
        assert!(
            serde_json::from_value::<StudioJobRequest>(wire).is_err(),
            "{name} must be refused"
        );
    }
    let profile = json!({"id": "a", "label": "A", "username": "u", "password": null,
                         "supported_writes": [], "extra": 1});
    assert!(serde_json::from_value::<AccessProfileEdit>(profile).is_err());
    assert!(
        serde_json::from_value::<CandidatePageQuery>(json!({"limit": 5, "filter": "x"})).is_err()
    );
    assert_eq!(
        serde_json::from_value::<CandidatePageQuery>(json!({})).unwrap(),
        CandidatePageQuery {
            cursor: None,
            limit: 50
        }
    );
}

#[test]
fn every_job_and_result_variant_round_trips() {
    let jobs = [
        json!({"kind": "draft_test", "connection": connection(), "required_sources": ["ppp", "dhcp", "hotspot", "wan"]}),
        json!({"kind": "collect", "router_id": "r", "expected_config_digest": "d", "sources": ["ppp"]}),
        json!({"kind": "evaluate", "router_id": "r", "expected_revision": "18446744073709551615",
               "expected_config_digest": "d", "snapshot_id": "s", "policies": [policy()]}),
    ];
    for job in jobs {
        round_trip::<StudioJobRequest>(json!({"key": "studio-0000000001", "job": job}));
    }
    let snapshot = json!({"kind": "snapshot", "id": "s", "router_id": "r", "config_digest": "d",
        "collected_from": 1, "collected_to": 2, "expires_at": 3, "tables": [
            {"table": "routes", "state": "failed", "records": 0, "collected_at": 1, "issue_code": "trap"},
            {"table": "hotspot_users", "state": "not_requested", "records": 0, "collected_at": null, "issue_code": null}]});
    let results = [
        json!({"kind": "draft_test", "connection_fingerprint": "f", "at": 1, "elapsed_ms": 2,
               "checks": [{"stage": "table_permission", "state": "not_run", "code": "skipped"}]}),
        snapshot,
        json!({"kind": "candidate", "id": "c", "base_revision": "1", "config_digest": "d",
               "snapshot_id": "s", "draft_digest": "x", "expires_at": 3}),
    ];
    for result in results {
        let op = round_trip::<Operation>(json!({"key": "k", "state": "done", "receipt": null,
            "review": null, "error": null, "discovery": null, "source_result": result}));
        assert!(op.source_result.is_some());
    }
}

#[test]
fn every_policy_variant_round_trips() {
    let filters = [
        json!({"kind": "all", "rules": [{"kind": "any", "rules": [
            {"kind": "match", "field": "active.service", "op": "in", "value": {"kind": "texts", "values": ["pppoe1"]}}]}]}),
        json!({"kind": "match", "field": "route.routing_table", "op": "prefix", "value": {"kind": "text", "value": "wan"}}),
    ];
    let groups = [
        json!({"kind": "by_field", "field": "lease.server"}),
        json!({"kind": "fixed", "name": "DHCP"}),
        json!({"kind": "attach", "node_id": "tm-node-1"}),
    ];
    let parents = [
        json!({"kind": "router"}),
        json!({"kind": "node", "node_id": "tm-node-1"}),
    ];
    let binding = json!([{"selector": "Gold", "plan_id": "plan-1"}]);
    let rates = json!([
        {"kind": "subscriber_override"},
        {"kind": "account_comment", "format": "legacy_configured"},
        {"kind": "active_comment", "format": "legacy_configured"},
        {"kind": "profile_map", "bindings": binding},
        {"kind": "server_map", "bindings": binding},
        {"kind": "profile_rate_limit"},
        {"kind": "fallback", "plan_id": "plan-1"}
    ]);
    let capacities = [
        json!({"kind": "from_plans", "down_factor": "0.5", "up_factor": "1", "minimum_down_bps": "0", "minimum_up_bps": "0"}),
        json!({"kind": "fixed", "down_bps": "100000000", "up_bps": "50000000"}),
    ];
    for (i, filter) in filters.iter().enumerate() {
        let mut p = policy();
        p["filter"] = filter.clone();
        p["group"] = groups[i % groups.len()].clone();
        p["parent"] = parents[i % parents.len()].clone();
        p["capacity"] = capacities[i % capacities.len()].clone();
        p["rates"] = rates.clone();
        p["members"] = json!(["active", "configured"][i]);
        p["writes"] =
            json!([{"access_ref": "wan-writer", "scope": "wan_address_lists", "enabled": false}]);
        round_trip::<SourcePolicyDraft>(p);
    }
    let mut p = policy();
    p["group"] = groups[2].clone();
    round_trip::<SourcePolicyDraft>(p);
    // `writes` is empty by default.
    let mut p = policy();
    p.as_object_mut().unwrap().remove("writes");
    assert!(
        serde_json::from_value::<SourcePolicyDraft>(p)
            .unwrap()
            .writes
            .is_empty()
    );
}

#[test]
fn source_edits_keep_explicit_empty_and_refuse_missing_policies() {
    let wire = json!({"key": "studio-save-0000000001", "expected_revision": 18, "edit": {
        "kind": "router.sources_set", "router_id": "lq3-a1",
        "expected_config_digest": "opaque-config-digest", "policies": []}});
    let request = round_trip::<EditRequest>(wire);
    assert!(
        matches!(request.edit, BuilderEdit::RouterSourcesSet { ref policies, .. } if policies.is_empty())
    );
    let missing =
        json!({"kind": "router.sources_set", "router_id": "r", "expected_config_digest": "d"});
    assert!(serde_json::from_value::<BuilderEdit>(missing).is_err());
    for decision in ["pin", "ignore", "clear"] {
        round_trip::<BuilderEdit>(json!({"kind": "wan.candidate_set", "router_id": "r",
            "expected_config_digest": "d", "interface": "ether1", "decision": decision}));
    }
}

#[test]
fn secrets_never_show_in_debug_and_old_router_edits_keep_their_form() {
    let job: StudioJobRequest = serde_json::from_value(json!({"key": "k", "job": {
        "kind": "draft_test", "connection": connection(), "required_sources": []}}))
    .unwrap();
    assert!(!format!("{job:?}").contains("s3cret"));
    let edit = RouterEdit {
        access_profiles: Some(vec![AccessProfileEdit {
            id: "default".into(),
            label: "Default".into(),
            username: "u".into(),
            password: Some(SecretInput("s3cret".into())),
            supported_writes: vec![WriteScope::WanAddressLists],
        }]),
        ..Default::default()
    };
    assert!(!format!("{edit:?}").contains("s3cret"));
    let old = serde_json::to_value(RouterEdit::default()).unwrap();
    assert!(old.get("access_profiles").is_none());
}

#[test]
fn wire_names_match_the_contract() {
    let names = |v: Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_owned())
            .collect()
    };
    let fields: Vec<FieldKey> = serde_json::from_value(json!([
        "active.service",
        "active.comment",
        "account.profile",
        "account.comment",
        "lease.status",
        "lease.server",
        "server.name",
        "user.profile",
        "hotspot.server",
        "interface.name",
        "route.routing_table"
    ]))
    .unwrap();
    assert_eq!(fields.len(), 11);
    let features = json!([
        "draft_test",
        "access_profiles",
        "source_collect",
        "policy_preview",
        "policy_save",
        "wan_evidence"
    ]);
    let typed: Vec<StudioFeature> = serde_json::from_value(features.clone()).unwrap();
    assert_eq!(names(serde_json::to_value(typed).unwrap()), names(features));
    let tables: Vec<TableId> = serde_json::from_value(json!([
        "interfaces",
        "routes",
        "ppp_secrets",
        "ppp_profiles",
        "ppp_active",
        "dhcp_servers",
        "dhcp_leases",
        "hotspot_users",
        "hotspot_servers",
        "hotspot_profiles",
        "hotspot_active"
    ]))
    .unwrap();
    assert_eq!(tables.len(), 11);
    let states: Vec<ReadState> = serde_json::from_value(json!([
        "complete",
        "partial",
        "failed",
        "not_requested",
        "stale"
    ]))
    .unwrap();
    assert_eq!(states.len(), 5);
    let limits = serde_json::to_value(StudioLimits::default()).unwrap();
    assert_eq!(limits["job_body_bytes"], 32768);
    assert_eq!(limits["page_max"], 100);
    assert_eq!(limits["access_profiles"], 8);
}

const CASES: [&str; 6] = [
    "normal",
    "partial_read",
    "revision_conflict",
    "expired_candidate",
    "lost_reply",
    "legacy_backend",
];

fn fixture(name: &str) -> Value {
    let path = |n: &str| {
        format!(
            "{}/tests/fixtures/studio/{n}.json",
            env!("CARGO_MANIFEST_DIR")
        )
    };
    let read = |n: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(path(n)).unwrap()).unwrap()
    };
    // Each case replaces top-level sections of `normal`.
    let mut base = read("normal");
    for (k, v) in read(name).as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

fn parse<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn fixture_cases_agree_with_the_contract() {
    for name in CASES {
        let case = fixture(name);
        assert_eq!(case["case"], name);
        let caps: Option<StudioCapabilities> = parse(&case["capabilities"]);
        assert_eq!(caps.is_none(), name == "legacy_backend");
        if let Some(caps) = caps {
            assert_eq!(caps.version, VERSION);
            assert_eq!(caps.limits, StudioLimits::default());
        }
        for (id, config) in case["source_configs"].as_object().unwrap() {
            let config: RouterSourceConfig = parse(config);
            assert_eq!(&config.router_id, id);
            assert!(config.access_profiles.len() <= MAX_ACCESS_PROFILES);
        }
        for (job, op) in case["operations"].as_object().unwrap() {
            let op: Operation = parse(op);
            let kind = match op.source_result {
                Some(StudioJobResult::DraftTest(_)) => "draft_test",
                Some(StudioJobResult::Snapshot(_)) => "collect",
                Some(StudioJobResult::Candidate(_)) => "evaluate",
                None => "none",
            };
            assert_eq!(kind, job, "{name}: operation result kind");
        }
        for (kind, refusal) in case["refusals"].as_object().unwrap() {
            assert!(
                ["draft_test", "collect", "evaluate", "router.sources_set"]
                    .contains(&kind.as_str())
            );
            assert!((400..600).contains(&refusal["status"].as_u64().unwrap()));
            let _: StudioError = parse(&refusal["error"]);
        }
        assert!(case["lost_reply"].is_boolean());
        assert!([200, 404, 410].contains(&case["candidate_status"].as_u64().unwrap()));
        let candidate: CandidateSummary = parse(&case["candidate"]);
        let records: Vec<ResolvedRecord> = parse(&case["records"]);
        let issues: Vec<ValidationIssue> = parse(&case["issues"]);
        let evaluated: Operation = parse(&case["operations"]["evaluate"]);
        assert_eq!(
            evaluated.source_result,
            Some(StudioJobResult::Candidate(candidate.reference.clone()))
        );
        let count = |s: Severity| issues.iter().filter(|i| i.severity == s).count() as u64;
        assert_eq!(
            candidate.issues,
            IssueCounts {
                blockers: count(Severity::Blocker),
                warnings: count(Severity::Warning),
                information: count(Severity::Information)
            },
            "{name}: counts cover every issue"
        );
        assert_eq!(
            candidate.publishable,
            candidate.complete && candidate.issues.blockers == 0
        );
        assert!(records.iter().all(|r| r.down_bps.as_deref() != Some("0")));
    }
}

#[test]
fn partial_fixture_never_shows_a_failed_table_as_complete() {
    let case = fixture("partial_read");
    let op: Operation = parse(&case["operations"]["collect"]);
    let Some(StudioJobResult::Snapshot(snapshot)) = op.source_result else {
        panic!("snapshot")
    };
    let failed: Vec<_> = snapshot
        .tables
        .iter()
        .filter(|t| t.state != ReadState::Complete)
        .collect();
    assert!(
        failed
            .iter()
            .any(|t| t.table == TableId::Routes && t.state == ReadState::Failed)
    );
    assert!(failed.iter().all(|t| t.issue_code.is_some()));
    let candidate: CandidateSummary = parse(&case["candidate"]);
    assert!(!candidate.complete && !candidate.publishable);
}
