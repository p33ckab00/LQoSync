use web_gateway::journal::{allowed, args, page};

fn line(at: u64, prio: &str, msg: serde_json::Value) -> String {
    serde_json::json!({"__REALTIME_TIMESTAMP": at.to_string(), "PRIORITY": prio, "MESSAGE": msg})
        .to_string()
}

#[test]
fn only_allowlisted_units_and_fixed_arguments_reach_journalctl() {
    assert!(allowed("lqosync3.service"));
    for bad in [
        "sshd.service",
        "lqosync3.service --all",
        "-ulqosync3.service",
        "",
    ] {
        assert!(!allowed(bad), "{bad}");
    }
    let a = args("lqosync3.service", 4, Some(1_700_000_000_000_001));
    assert_eq!(&a[..2], ["-u", "lqosync3.service"]);
    assert!(a.contains(&"0..4".to_string()));
    assert_eq!(
        a.last().unwrap(),
        "--until=@1700000000.000000",
        "one microsecond earlier"
    );
    assert!(args("lqosync3.service", 9, None).contains(&"0..7".to_string()));
}

#[test]
fn a_page_keeps_the_newest_lines_cuts_long_messages_and_reads_binary_messages() {
    let mut out: Vec<String> = (0..250u64)
        .map(|i| line(1_000 + i, "6", serde_json::json!(format!("cycle {i}"))))
        .collect();
    out.push(line(2_000, "3", serde_json::json!([104, 105])));
    out.push(line(2_001, "4", serde_json::json!("x".repeat(5000))));
    out.push("not json".into());
    let p = page("lqosync3.service", &out.join("\n"));
    assert!(p.more && p.cut);
    assert_eq!(p.entries.len(), web_contract::journal::PAGE);
    let last = &p.entries[p.entries.len() - 2..];
    assert_eq!((last[0].message.as_str(), last[0].priority), ("hi", 3));
    assert_eq!(
        last[1].message.chars().count(),
        web_contract::journal::MAX_MESSAGE
    );
    assert!(
        p.entries.windows(2).all(|w| w[0].at_usec <= w[1].at_usec),
        "oldest first"
    );
    assert!(serde_json::to_vec(&p).unwrap().len() <= 60 * 1024);
    let empty = page("lqosync3.service", "");
    assert!(!empty.more && empty.entries.is_empty());
}
