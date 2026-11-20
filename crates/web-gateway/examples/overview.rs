//! Read only the allowlisted public cache projection. No collectors run.
fn main() -> anyhow::Result<()> {
    let view = web_gateway::overview::project(
        std::path::Path::new("/var/lib/lqosync3"),
        web_gateway::broker::now(),
    );
    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}
