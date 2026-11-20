use anyhow::{Result, bail, ensure};
use web_gateway::accounts::Accounts;
fn password() -> Result<String> {
    let a = rpassword::prompt_password("Password: ")?;
    let b = rpassword::prompt_password("Repeat password: ")?;
    ensure!(a == b, "passwords differ");
    Ok(a)
}
fn main() -> Result<()> {
    ensure!(
        unsafe { libc::geteuid() } == 0,
        "account administration requires sudo/root"
    );
    let args: Vec<_> = std::env::args().skip(1).collect();
    let store = Accounts::new(
        std::env::var("WEB_ACCOUNTS")
            .unwrap_or("/var/lib/lqosync3-web-accounts/web-accounts.db".into())
            .into(),
    )?;
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["user", "create", name] => store.create(name, &password()?)?,
        ["user", "passwd", name] => store.password(name, &password()?)?,
        ["user", "rename", name, new] => store.rename(name, new)?,
        ["user", "disable", name] => store.disable(name, true)?,
        ["user", "enable", name] => store.disable(name, false)?,
        ["user", "delete", name] => {
            eprint!("Delete {name}? Type the username: ");
            let mut confirm = String::new();
            std::io::stdin().read_line(&mut confirm)?;
            ensure!(confirm.trim() == *name, "cancelled");
            store.delete(name)?;
        }
        ["user", "list"] => {
            for a in store.list()? {
                println!(
                    "{}\tsuperadmin\t{}",
                    a.username,
                    if a.disabled { "disabled" } else { "active" }
                );
            }
        }
        ["user", "show", name] => {
            let a = store
                .list()?
                .into_iter()
                .find(|a| a.username == *name)
                .ok_or_else(|| anyhow::anyhow!("account not found"))?;
            println!(
                "{}\tsuperadmin\t{}",
                a.username,
                if a.disabled { "disabled" } else { "active" }
            );
        }
        _ => bail!(
            "usage: lqosync3-web-admin user <create|passwd|rename|disable|enable|delete|list|show> [username]"
        ),
    }
    Ok(())
}
