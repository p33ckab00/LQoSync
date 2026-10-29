use crate::{
    speeds::DhcpServerRule,
    store,
    web::{App, Shared, check, csrf, error},
};
use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use maud::{Markup, html};
use serde::Deserialize;
fn key(router: &str, server: &str) -> String {
    serde_json::to_string(&[router, server]).expect("string tuple serializes")
}
fn fields(policy: Option<&DhcpServerRule>) -> Markup {
    let mode = policy.map(|p| p.mode.as_str()).unwrap_or("auto");
    html! {div.formgrid{
        label{"Speed source" select name="mode"{@for (value,label) in [("auto","Auto · comment → name → manual"),("comment","DHCP server comment"),("name","DHCP server name"),("manual","Manual per subscriber")]{option value=(value) selected[value==mode]{(label)}}}}
        label{"Manual download · Mbps per subscriber" input type="number" step="any" min="0" name="download" value=(policy.and_then(|p|p.download).map(|v|v.to_string()).unwrap_or_default());}
        label{"Manual upload · Mbps per subscriber" input type="number" step="any" min="0" name="upload" value=(policy.and_then(|p|p.upload).map(|v|v.to_string()).unwrap_or_default());}
    }}
}
pub(crate) fn controls(a: &App) -> Markup {
    let revision = crate::core_web::revision(&a.settings, &None);
    let choices = crate::sources_web::choices(a, "dhcp");
    html! {section.card{
        h2{"DHCP server speeds"}
        p{"Bound leases inherit their actual DHCP server's speed. Lease comments and lease rates do not override this policy."}
        p.help{"Comment example: Site A | 25M/5M. Name examples: DHCP-25M or DHCP-25M/5M. All three sources use subscriber download/upload order."}
        p.help{"Manual values apply to each subscriber, separately from branch capacity and WAN limits. Auto falls back only when a source has no speed. Malformed rules stay unresolved."}
        @for policy in a.settings.speed_rules.dhcp_servers.iter().take(200){
            @let selection=key(&policy.router,&policy.server);
            details{summary{(&policy.router) " · " (&policy.server) " · " (&policy.mode)}
                @if !choices.iter().any(|c|c.value==selection){p.warning{"Current server metadata is unavailable. Collect fresh inventory before publication."}}
                form method="post" action="/speed-rules/dhcp"{(csrf(a))input type="hidden" name="revision" value=(&revision);input type="hidden" name="selection" value=(&selection);
                    (fields(Some(policy)))button name="action" value="save"{"Save server speed"} " " button name="action" value="remove" formnovalidate{"Remove rule · use Auto"}
                }
            }
        }
        @if a.settings.speed_rules.dhcp_servers.len()>200{p.help{"First 200 configured rules shown. Additional rules remain in settings.toml."}}
        form method="post" action="/speed-rules/dhcp"{(csrf(a))input type="hidden" name="revision" value=(&revision);
            label{"Add or replace a DHCP server rule" select name="selection" required{option value=""{"Select a router and DHCP server"}@for choice in choices.iter().take(200){option value=(&choice.value){(&choice.label)}}}}
            (fields(None))button name="action" value="save"{"Save server speed"}
        }
        @if choices.is_empty(){p.help{"Read subscriber router inventory to discover DHCP servers."}}
        @if choices.len()>200{p.help{"First 200 discovered servers shown."}}
        p.help{"Servers without a local rule use Auto. Saving triggers the existing automatic local publication checks; router configuration stays unchanged."}
    }}
}
#[derive(Deserialize)]
pub struct ServerForm {
    token: String,
    revision: String,
    selection: String,
    mode: String,
    #[serde(default)]
    download: String,
    #[serde(default)]
    upload: String,
    action: String,
}
pub async fn save(State(s): State<Shared>, Form(f): Form<ServerForm>) -> Response {
    let mut a = s.lock().await;
    if let Err(r) = check(&a, &f.token) {
        return r.into_response();
    }
    if f.revision != crate::core_web::revision(&a.settings, &None) {
        return error(&a, "Settings changed; reload DHCP server rules");
    }
    let selection: [String; 2] = match serde_json::from_str(&f.selection) {
        Ok(v) => v,
        Err(_) => return error(&a, "Select an exact router and DHCP server"),
    };
    let mut next = a.settings.clone();
    let exists = next
        .speed_rules
        .dhcp_servers
        .iter()
        .any(|p| p.router == selection[0] && p.server == selection[1]);
    if f.action == "remove" {
        if !exists {
            return error(&a, "DHCP server rule no longer exists");
        }
        next.speed_rules
            .dhcp_servers
            .retain(|p| !(p.router == selection[0] && p.server == selection[1]));
    } else if f.action == "save" {
        if !exists
            && !crate::sources_web::choices(&a, "dhcp")
                .iter()
                .any(|c| c.value == f.selection)
        {
            return error(
                &a,
                "Server metadata changed or is unavailable; read inventory again",
            );
        }
        let rate = |text: &str| -> anyhow::Result<Option<f64>> {
            if text.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.trim().parse().map_err(|_| {
                    anyhow::anyhow!("Invalid DHCP manual rate")
                })?))
            }
        };
        let (download, upload) = match (rate(&f.download), rate(&f.upload)) {
            (Ok(d), Ok(u)) => (d, u),
            _ => return error(&a, "Invalid DHCP manual rates"),
        };
        next.speed_rules
            .dhcp_servers
            .retain(|p| !(p.router == selection[0] && p.server == selection[1]));
        next.speed_rules.dhcp_servers.push(DhcpServerRule {
            router: selection[0].clone(),
            server: selection[1].clone(),
            mode: f.mode,
            download,
            upload,
        });
        next.speed_rules
            .dhcp_servers
            .sort_by(|a, b| (&a.router, &a.server).cmp(&(&b.router, &b.server)));
    } else {
        return error(&a, "Unknown DHCP rule action");
    }
    if let Err(e) = store::save_settings(&a.root, &next) {
        return error(&a, e);
    }
    a.settings = next;
    Redirect::to("/speed-rules").into_response()
}
