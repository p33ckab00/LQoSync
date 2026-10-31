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
