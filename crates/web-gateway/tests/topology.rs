use web_contract::topology::*;
use web_gateway::topology::{BUDGET, answer, load};

fn subscriber(i: usize, plan: Option<&str>, active: bool) -> Subscriber {
    Subscriber {
        id: format!("sub-{i:05}"),
        identities: vec![Identity {
            router_id: "lq3-aa".into(),
            source_id: "ppp-a".into(),
            kind: "ppp".into(),
            account_ref: format!("ppp-name/v1/user-{i}"),
        }],
        plan_id: plan.map(str::to_owned),
        pin: None,
        assignment: None,
        active: active.then(|| Active {
            node: "wan1".into(),
            ips: vec!["192.0.2.1".into()],
            min_mbps: [1.0, 1.0],
            max_mbps: [2.0, 2.0],
            fallback: false,
        }),
    }
}

fn observation(i: usize, kind: &str, router: &str, linked: bool) -> Observation {
    Observation {
        router_id: format!("lq3-{router}"),
        router: router.into(),
        kind: kind.into(),
        name: format!("user-{i}"),
        ipv4: "192.0.2.1".into(),
        mac: String::new(),
        state: "online".into(),
        link: if linked { "verified" } else { "none" }.into(),
        subscriber_id: linked.then(|| format!("sub-{i:05}")),
        plan_id: None,
        pin: None,
        node: None,
    }
}

fn view(n: usize) -> TopologyView {
    TopologyView {
        version: VERSION,
        updated: 1_000,
        fresh_for: 60,
        snapshot_id: "r7-c1".into(),
        registry_revision: Some("7".into()),
        mode: "observe".into(),
        writer: "legacy".into(),
        subscribers: (0..n)
            .map(|i| subscriber(i, (i % 2 == 0).then_some("plan-a"), i % 3 == 0))
            .collect(),
        observations: (0..n)
            .flat_map(|i| {
                [
                    observation(i, "ppp", "aa", true),
                    observation(i, "dhcp", "bb", false),
                ]
            })
            .collect(),
        conflicts: (0..500)
            .map(|i| Conflict {
                kind: "identity".into(),
                subject: format!("s{i}"),
                detail: "x".repeat(40),
            })
            .collect(),
        nodes: (0..2_000)
            .map(|i| Node {
                id: format!("node-{i}"),
                name: format!("Node {i}"),
                parent: None,
                kind: "group".into(),
                source_id: None,
                capacity: Capacity::Automatic,
                alias: false,
                calculated_mbps: None,
                exported_mbps: None,
                members: None,
                geo: None,
                position: None,
                router_id: None,
                auto_members: None,
                wan_health: None,
                address_list: None,
                routed: None,
                active: true,
                tree_excluded: false,
            })
            .collect(),
        ..Default::default()
    }
}

fn reply_fits(reply: &TopologyReply) {
    let whole = web_gateway::broker::Reply {
        ok: true,
        topology: Some(reply.clone()),
        ..Default::default()
    };
    assert!(serde_json::to_vec(&whole).unwrap().len() < 65536);
    assert!(serde_json::to_vec(reply).unwrap().len() <= BUDGET);
}

fn obs_page(v: &TopologyView, q: &PageQuery) -> Page<Observation> {
    match answer(v, &Read::Observations(q.clone())) {
        TopologyReply::Observations(p) => p,
        other => panic!("{other:?}"),
    }
}

use web_gateway::topology::Read;

#[test]
fn the_summary_counts_everything_and_bounds_its_lists() {
    let v = view(3_000);
    let TopologyReply::Summary(s) = answer(&v, &Read::Summary) else {
        panic!()
    };
    assert_eq!((s.subscribers_total, s.subscribers_active), (3_000, 1_000));
    assert_eq!((s.ppp_total, s.dhcp_total), (3_000, 3_000));
    assert_eq!(s.conflicts_total, 500);
    assert!(s.conflicts.len() < 500 && !s.conflicts.is_empty());
    assert!(s.nodes_truncated && !s.nodes.is_empty());
    assert_eq!(s.nodes[0].id, "node-0", "keeps catalog order");
    reply_fits(&TopologyReply::Summary(s));
}

#[test]
fn source_pages_filter_before_paging_and_cover_every_row_once() {
    let v = view(1_000);
    let q = PageQuery {
        kind: Some("dhcp".into()),
        ..Default::default()
    };
    let mut seen = std::collections::HashSet::new();
    let mut next = q.clone();
    loop {
        let page = obs_page(&v, &next);
        reply_fits(&TopologyReply::Observations(page.clone()));
        assert_eq!(page.total, 1_000);
        assert!(page.items.len() <= MAX_PAGE);
        for o in &page.items {
            assert_eq!(o.kind, "dhcp");
            assert!(seen.insert(o.name.clone()), "duplicate {}", o.name);
        }
        match page.next_cursor {
            Some(c) => next.cursor = Some(c),
            None => break,
        }
    }
    assert_eq!(seen.len(), 1_000);
}

#[test]
fn filters_combine_and_text_search_matches_names() {
    let v = view(100);
    let q = PageQuery {
        kind: Some("ppp".into()),
        link: Some("verified".into()),
        q: Some("user-42".into()),
        ..Default::default()
    };
    let page = obs_page(&v, &q);
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].subscriber_id.as_deref(), Some("sub-00042"));
    let none = obs_page(
        &v,
        &PageQuery {
            kind: Some("dhcp".into()),
            link: Some("verified".into()),
            ..Default::default()
        },
    );
    assert_eq!(none.total, 0);
}

#[test]
fn a_cursor_from_another_snapshot_or_filter_expires() {
    let v = view(1_000);
    let q = PageQuery {
        kind: Some("ppp".into()),
        ..Default::default()
    };
    let cursor = obs_page(&v, &q).next_cursor.unwrap();
    let changed = PageQuery {
        kind: Some("dhcp".into()),
        cursor: Some(cursor.clone()),
        ..Default::default()
    };
    assert_eq!(
        answer(&v, &Read::Observations(changed)),
        TopologyReply::Expired
    );
    let mut newer = v.clone();
    newer.snapshot_id = "r8-c2".into();
    let same = PageQuery {
        cursor: Some(cursor),
        ..q.clone()
    };
    assert_eq!(
        answer(&newer, &Read::Observations(same)),
        TopologyReply::Expired
    );
    let junk = PageQuery {
        cursor: Some("junk".into()),
        ..q
    };
    assert_eq!(
        answer(&v, &Read::Observations(junk)),
        TopologyReply::Expired
    );
}

#[test]
fn assignments_filter_by_plan_and_state() {
    let v = view(300);
    let q = PageQuery {
        plan_id: Some("plan-a".into()),
        state: Some("active".into()),
        ..Default::default()
    };
    let TopologyReply::Assignments(page) = answer(&v, &Read::Assignments(q)) else {
        panic!()
    };
    // Even and divisible by three: 0, 6, 12, ... 294.
    assert_eq!(page.total, 50);
    assert!(
        page.items
            .iter()
            .all(|s| s.active.is_some() && s.plan_id.as_deref() == Some("plan-a"))
    );
    let offline = PageQuery {
        state: Some("offline".into()),
        ..Default::default()
    };
    let TopologyReply::Assignments(page) = answer(&v, &Read::Assignments(offline)) else {
        panic!()
    };
    assert_eq!(page.total, 200);
}

#[test]
fn one_subscriber_with_its_rows_or_not_found() {
    let v = view(10);
    let TopologyReply::Subscriber(d) = answer(&v, &Read::Subscriber("sub-00004".into())) else {
        panic!()
    };
    assert_eq!(d.subscriber.id, "sub-00004");
    assert_eq!(d.observations.len(), 1);
    assert_eq!(
        answer(&v, &Read::Subscriber("sub-99999".into())),
        TopologyReply::NotFound
    );
}

#[test]
fn a_missing_cache_or_wrong_version_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).is_none());
    let mut v = view(2);
    v.version = VERSION + 1;
    std::fs::write(dir.path().join(CACHE), serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(load(dir.path()).is_none());
    // A real 50,000-subscriber cache is larger than the 2 MiB limit of the small caches.
    let big = view(20_000);
    std::fs::write(dir.path().join(CACHE), serde_json::to_vec(&big).unwrap()).unwrap();
    assert_eq!(load(dir.path()).unwrap().subscribers.len(), 20_000);
}

/// Release gate: 50,000 subscribers and 5,000 nodes through the real file read. Run with
/// `cargo test --release -p web-gateway --test topology -- --ignored --nocapture`.
#[test]
#[ignore]
fn release_scale_reads_stay_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let mut big = view(50_000);
    big.nodes = (0..5_000)
        .map(|i| Node {
            id: format!("node-{i}"),
            name: format!("Node {i}"),
            parent: (i > 0).then(|| format!("node-{}", (i - 1) / 4)),
            kind: "group".into(),
            source_id: None,
            capacity: Capacity::Automatic,
            alias: false,
            calculated_mbps: None,
            exported_mbps: None,
            members: Some(10),
            geo: None,
            position: None,
            router_id: None,
            auto_members: None,
            wan_health: None,
            address_list: None,
            routed: None,
            active: true,
            tree_excluded: false,
        })
        .collect();
    let bytes = serde_json::to_vec(&big).unwrap();
    std::fs::write(dir.path().join(CACHE), &bytes).unwrap();
    let started = std::time::Instant::now();
    let loaded = load(dir.path()).unwrap();
    let read = started.elapsed();
    let reads = [
        Read::Summary,
        Read::Assignments(PageQuery::default()),
        Read::Observations(PageQuery {
            kind: Some("dhcp".into()),
            q: Some("user-49".into()),
            ..Default::default()
        }),
        Read::Subscriber("sub-49999".into()),
    ];
    for r in &reads {
        let started = std::time::Instant::now();
        let reply = answer(&loaded, r);
        let took = started.elapsed();
        let size = serde_json::to_vec(&reply).unwrap().len();
        eprintln!("{r:?}: {took:?}, {size} bytes");
        assert!(size <= BUDGET);
    }
    eprintln!("cache {} bytes, load {read:?}", bytes.len());
}

#[test]
fn the_summary_carries_the_layout_revision_and_card_positions() {
    let mut v = view(1);
    v.layout_revision = Some("4".into());
    v.nodes[0].position = Some([10.0, 20.5]);
    let TopologyReply::Summary(s) = answer(&v, &Read::Summary) else {
        panic!("not a summary");
    };
    assert_eq!(s.layout_revision.as_deref(), Some("4"));
    assert_eq!(s.nodes[0].position, Some([10.0, 20.5]));
}
