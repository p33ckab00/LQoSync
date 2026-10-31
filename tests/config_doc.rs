use lqosync3::config::Config;

#[test]
fn example_in_configuration_md_parses() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/CONFIGURATION.md"
    ))
    .unwrap();
    let start = doc.find("```toml\n").expect("toml block") + "```toml\n".len();
    let end = start + doc[start..].find("```").expect("end of toml block");
    Config::from_toml_str(&doc[start..end]).unwrap();
}

/// Each key of the example outside [[routers]] must have the value of `config/defaults.toml`, so the copy cannot drift.
#[test]
fn example_values_match_the_defaults_file() {
    fn leaves(prefix: &str, t: &toml::Table, out: &mut Vec<(String, toml::Value)>) {
        for (k, v) in t {
            let key = format!("{prefix}{k}");
            match v {
                toml::Value::Table(sub) => leaves(&format!("{key}."), sub, out),
                _ => out.push((key, v.clone())),
            }
        }
    }
    fn get<'a>(t: &'a toml::Table, key: &str) -> Option<&'a toml::Value> {
        let mut parts = key.split('.');
        let mut v = t.get(parts.next()?)?;
        for p in parts {
            v = v.as_table()?.get(p)?;
        }
        Some(v)
    }
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/CONFIGURATION.md"
    ))
    .unwrap();
    let start = doc.find("```toml\n").expect("toml block") + "```toml\n".len();
    let end = start + doc[start..].find("```").expect("end of toml block");
    let mut example: toml::Table = toml::from_str(&doc[start..end]).unwrap();
    let defaults: toml::Table = toml::from_str(lqosync3::config::DEFAULTS_TOML).unwrap();
    // [[routers]] shows overrides on purpose. The parse test checks that its keys are valid.
    example.remove("routers").expect("example has [[routers]]");
    let mut wrong = Vec::new();
    let mut l = Vec::new();
    leaves("", &example, &mut l);
    for (k, v) in l {
        if get(&defaults, &k) != Some(&v) {
            wrong.push(format!("{k} = {v}"));
        }
    }
    assert!(wrong.is_empty(), "not the default value: {wrong:#?}");
}
