use lqosync_v3::model::*;

fn node(id: &str, parent: &str, mode: &str, down: f64, up: f64) -> Node {
    Node {
        id: id.into(),
        name: id.into(),
        parent: parent.into(),
        kind: if parent.is_empty() { "wan" } else { "pppoe" }.into(),
        router: String::new(),
        source: String::new(),
        mode: mode.into(),
        download: down,
        upload: up,
        download_factor: 0.35,
        upload_factor: 0.5,
    }
}
fn subscriber(id: &str, node: &str, down: f64, up: f64) -> Subscriber {
    Subscriber {
        id: id.into(),
        name: format!("Customer, {id}"),
        ipv4: format!("100.64.0.{id}"),
        node: node.into(),
        download: down,
        upload: up,
        download_min: 1.0,
        upload_min: 1.0,
    }
}
fn settings() -> Settings {
    Settings {
        interval_seconds: 30,
        auto_stage: true,
        speed_rules: Default::default(),
        routers: vec![],
        nodes: vec![
            node("wan", "", "manual", 620.0, 100.0),
            node("ppp", "wan", "computed", 0.0, 0.0),
        ],
    }
}
#[test]
fn computed_directions_cap_independently_and_preserve_subscriber_rates() {
    let plan = compile(
        &settings(),
        &[
            subscriber("1", "ppp", 1000.0, 300.0),
            subscriber("2", "ppp", 1000.0, 100.0),
        ],
    )
    .unwrap();
    let c = &plan.capacities["ppp"];
    assert_eq!((c.raw_download, c.raw_upload), (700.0, 200.0));
    assert_eq!((c.download, c.upload), (620.0, 100.0));
    let rows = csv::Reader::from_reader(plan.csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(&rows[0][1], "Customer, 1");
    assert_eq!(&rows[0][4], "ppp");
    assert_eq!(&rows[0][10], "1000");
    let json: serde_json::Value = serde_json::from_str(&plan.network).unwrap();
    assert_eq!(json["wan"]["children"]["ppp"]["uploadBandwidthMbps"], 100.0);
    assert!(!plan.network.contains("virtual"));
}
#[test]
fn nested_subscribers_count_once_and_use_immediate_parent_cap() {
    let mut s = settings();
    s.nodes[1] = node("group", "wan", "manual", 200.0, 50.0);
    s.nodes.push(node("ppp", "group", "computed", 0.0, 0.0));
    let p = compile(&s, &[subscriber("1", "ppp", 1000.0, 200.0)]).unwrap();
    assert_eq!(p.capacities["wan"].subscribers, 1);
    assert_eq!(p.capacities["ppp"].download, 200.0);
    assert_eq!(p.capacities["ppp"].upload, 50.0);
}
#[test]
fn cycles_and_missing_parents_are_rejected() {
    let mut s = settings();
    s.nodes[0].parent = "ppp".into();
    assert!(validate(&s).is_err());
    let mut s = settings();
    s.nodes[1].parent = "absent".into();
    assert!(validate(&s).is_err());
}
#[test]
fn explicit_over_parent_and_duplicate_ip_are_rejected() {
    let mut s = settings();
    s.nodes[1] = node("ppp", "wan", "manual", 700.0, 20.0);
    assert!(compile(&s, &[]).is_err());
    let a = subscriber("1", "ppp", 100.0, 20.0);
    let mut b = a.clone();
    b.id = "2".into();
    assert!(compile(&settings(), &[a, b]).is_err());
}
#[test]
fn missing_rates_and_unknown_nodes_do_not_publish() {
    assert!(compile(&settings(), &[subscriber("1", "absent", 10.0, 10.0)]).is_err());
    assert!(compile(&settings(), &[subscriber("1", "ppp", 0.0, 10.0)]).is_err());
    assert!(compile(&settings(), &[]).is_err());
}
