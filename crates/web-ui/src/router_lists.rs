//! Router lists: exclusions and DHCP client overrides. A save replaces a whole list in the
//! configuration only; it makes no router I/O.
use crate::router_form::{Ctx, Saved, ctx, follow, loaded_digest, send};
use crate::router_form_model::{ClientRow, ServerRow, bad_ips, clients, lines, servers};
use gloo_net::http::Request;
use leptos::prelude::*;
use web_contract::routers::{RouterEdit, RouterEditForm, RouterLists, RouterSummary};

fn saved(id: String, reply: web_contract::routers::RouterEditReply, what: &'static str) -> Saved {
    Saved {
        router_id: id,
        digest: reply.digest,
        reload: reply.reload,
        warnings: reply.warnings,
        what,
        disable: false,
    }
}

/// The lists of one router. They load again when the service loads a new configuration.
#[component]
pub(crate) fn ListsSection(router: RouterSummary) -> impl IntoView {
    let c = ctx();
    let lists = RwSignal::new(None::<Result<RouterLists, String>>);
    let id = router.id.clone();
    // Load again when the loaded configuration changes. The previous digest stops a repeat load.
    Effect::new(move |previous: Option<String>| {
        let digest = c
            .inventory
            .with(|i| i.as_ref().map(|i| i.config_digest.clone()))
            .unwrap_or_default();
        if previous.as_ref() == Some(&digest) {
            return digest;
        }
        let id = id.clone();
        leptos::task::spawn_local(async move {
            let (signal, _timer) = crate::app::deadline();
            let r = Request::get(&format!("/api/v1/routers/{id}"))
                .abort_signal(Some(&signal))
                .send()
                .await;
            let out = match r {
                Ok(r) if r.ok() => r
                    .json::<RouterSummary>()
                    .await
                    .map(|r| r.lists.unwrap_or_default())
                    .map_err(|_| "The router lists are not valid.".to_string()),
                Ok(r) if r.status() == 401 => {
                    c.session.set(None);
                    return;
                }
                _ => Err("The router lists could not load. Try again later.".to_string()),
            };
            lists.try_set(Some(out));
        });
        digest
    });
    let id = router.id.clone();
    view! {
      <section class="router-lists" aria-label="Router lists">
        {move||match lists.get() {
          None=>view!{<p class="router-empty" role="status">"Loading the router lists…"</p>}.into_any(),
          Some(Err(e))=>view!{<p class="notice" role="status">{e}</p>}.into_any(),
          Some(Ok(l))=>view!{
            <Exclusions id=id.clone() lists=l.clone()/>
            <Clients id=id.clone() lists=l.clone()/>
            <Servers id=id.clone() lists=l/>
          }.into_any(),
        }}
      </section>
    }
}

fn show(list: &[String]) -> String {
    if list.is_empty() {
        "None".into()
    } else {
        list.join(", ")
    }
}

#[component]
fn Exclusions(id: String, lists: RouterLists) -> impl IntoView {
    let c = ctx();
    let editing = RwSignal::new(false);
    let fields = [
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    ];
    let ip_error = RwSignal::new(String::new());
    let failure = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let digest = RwSignal::new(String::new());
    let l = StoredValue::new(lists);
    let id = StoredValue::new(id);
    let old = move || {
        l.with_value(|l| {
            [
                l.exclude_dhcp_servers.clone(),
                l.exclude_ppp_profiles.clone(),
                l.exclude_ips.clone(),
            ]
        })
    };
    let open = move |_| {
        for (f, list) in fields.iter().zip(old()) {
            f.set(list.join("\n"));
        }
        ip_error.set(String::new());
        failure.set(String::new());
        digest.set(loaded_digest(c));
        editing.set(true);
    };
    let save = move || {
        if busy.get_untracked() {
            return;
        }
        let new = fields.map(|f| lines(&f.get_untracked()));
        let bad = bad_ips(&new[2]);
        if !bad.is_empty() {
            ip_error.set(format!(
                "These lines are not IPv4 addresses or networks: {}.",
                bad.join(", ")
            ));
            return;
        }
        ip_error.set(String::new());
        let before = old();
        let pick = |i: usize| (new[i] != before[i]).then(|| new[i].clone());
        let edit = RouterEdit {
            expected_digest: digest.get_untracked(),
            exclude_dhcp_servers: pick(0),
            exclude_ppp_profiles: pick(1),
            exclude_ips: pick(2),
            ..Default::default()
        };
        if edit.exclude_dhcp_servers.is_none()
            && edit.exclude_ppp_profiles.is_none()
            && edit.exclude_ips.is_none()
        {
            failure.set("There is no change to save.".into());
            return;
        }
        busy.set(true);
        failure.set(String::new());
        let id = id.get_value();
        leptos::task::spawn_local(async move {
            match send(
                c,
                Some(id.clone()),
                RouterEditForm {
                    edit,
                    current_password: None,
                },
            )
            .await
            {
                Ok(reply) => {
                    editing.set(false);
                    follow(c, saved(id, reply, "Exclusions saved."));
                }
                Err(f) => failure.set(f.message),
            }
            busy.try_set(false);
        });
    };
    let labels = [
        (
            "exclude-dhcp",
            "DHCP servers to skip",
            "One server name for each line. * matches any text.",
        ),
        (
            "exclude-ppp",
            "PPP profiles to skip",
            "One profile name for each line. * matches any text.",
        ),
        (
            "exclude-ips",
            "IP addresses and networks to skip",
            "One IPv4 address or network for each line, such as 100.64.9.0/24.",
        ),
    ];
    view! {
      <h3>"Exclusions"</h3>
      <Show when=move||editing.get() fallback=move||{let [a,b,ip]=old();view!{
        <dl class="router-facts">
          <div><dt>"DHCP servers to skip"</dt><dd>{show(&a)}</dd></div>
          <div><dt>"PPP profiles to skip"</dt><dd>{show(&b)}</dd></div>
          <div><dt>"IP addresses and networks to skip"</dt><dd>{show(&ip)}</dd></div>
        </dl>
        <div class="form-actions start">
          <button type="button" class="secondary" disabled=move||c.inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty())) on:click=open>"Edit exclusions"</button>
        </div>
      }}>
        <form class="router-form inline" novalidate on:submit=move|ev|{ev.prevent_default();save();}>
          <p class="field-help">"The service joins these lists with the global exclusions. An excluded subscriber leaves the shaping after the grace time."</p>
          {labels.into_iter().zip(fields).map(|((fid,label,help),f)|view!{
            <div class="field">
              <label for=fid>{label}</label>
              <textarea id=fid rows="4" spellcheck="false" aria-describedby=format!("{fid}-help {fid}-error")
                prop:value=move||f.get() on:input=move|ev|f.set(event_target_value(&ev))></textarea>
              <p class="field-help" id=format!("{fid}-help")>{help}</p>
              <p class="field-error" id=format!("{fid}-error") aria-live="polite">{move||if fid=="exclude-ips"{ip_error.get()}else{String::new()}}</p>
            </div>
          }).collect_view()}
          <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
          <div class="form-actions start">
            <button type="submit" class="primary" disabled=move||busy.get()>{move||if busy.get(){"Saving…"}else{"Save exclusions"}}</button>
            <button type="button" class="quiet" on:click=move|_|editing.set(false)>"Cancel"</button>
          </div>
        </form>
      </Show>
    }
}

#[component]
fn Clients(id: String, lists: RouterLists) -> impl IntoView {
    let c: Ctx = ctx();
    let editing = RwSignal::new(false);
    // Each row has a stable key, so typing in one row never rebuilds the others.
    let rows = RwSignal::new(Vec::<(u32, ClientRow)>::new());
    let next = RwSignal::new(0u32);
    let errors = RwSignal::new(Vec::<(usize, String)>::new());
    let failure = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let digest = RwSignal::new(String::new());
    let truncated = lists.truncated;
    let l = StoredValue::new(lists);
    let id = StoredValue::new(id);
    let open = move |_| {
        let list: Vec<(u32, ClientRow)> = l.with_value(|l| {
            l.dhcp_clients
                .iter()
                .enumerate()
                .map(|(i, c)| (i as u32, ClientRow::from(c)))
                .collect()
        });
        next.set(list.len() as u32);
        rows.set(list);
        errors.set(Vec::new());
        failure.set(String::new());
        digest.set(loaded_digest(c));
        editing.set(true);
    };
    let add = move |_| {
        let k = next.get_untracked();
        next.set(k + 1);
        rows.update(|r| r.push((k, ClientRow::default())));
        request_animation_frame(move || {
            if let Some(e) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&format!("client-mac-{k}")))
                .and_then(|e| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(e).ok())
            {
                let _ = e.focus();
            }
        });
    };
    let save = move || {
        if busy.get_untracked() {
            return;
        }
        let list: Vec<ClientRow> =
            rows.with_untracked(|r| r.iter().map(|(_, row)| row.clone()).collect());
        let checked = match clients(&list) {
            Ok(v) => v,
            Err(e) => {
                errors.set(e);
                return;
            }
        };
        errors.set(Vec::new());
        if l.with_value(|l| l.dhcp_clients == checked) {
            failure.set("There is no change to save.".into());
            return;
        }
        busy.set(true);
        failure.set(String::new());
        let id = id.get_value();
        let edit = RouterEdit {
            expected_digest: digest.get_untracked(),
            dhcp_clients: Some(checked),
            ..Default::default()
        };
        leptos::task::spawn_local(async move {
            match send(
                c,
                Some(id.clone()),
                RouterEditForm {
                    edit,
                    current_password: None,
                },
            )
            .await
            {
                Ok(reply) => {
                    editing.set(false);
                    follow(c, saved(id, reply, "Client overrides saved."));
                }
                Err(f) => failure.set(f.message),
            }
            busy.try_set(false);
        });
    };
    let field = move |k: u32, get: fn(&ClientRow) -> String, put: fn(&mut ClientRow, String)| {
        (
            move || {
                rows.with(|r| {
                    r.iter()
                        .find(|(x, _)| *x == k)
                        .map(|(_, row)| get(row))
                        .unwrap_or_default()
                })
            },
            move |ev: leptos::ev::Event| {
                let v = event_target_value(&ev);
                rows.update(|r| {
                    if let Some((_, row)) = r.iter_mut().find(|(x, _)| *x == k) {
                        put(row, v);
                    }
                })
            },
        )
    };
    view! {
      <h3>"DHCP client overrides"</h3>
      <Show when=move||editing.get() fallback=move||view!{
        {l.with_value(|l| if l.dhcp_clients.is_empty() {
          view!{<p class="router-empty">"No client overrides. A client override replaces the rate of one DHCP lease, found by its MAC address."</p>}.into_any()
        } else {
          view!{
            <div class="table-scroll" role="region" aria-label="DHCP client overrides" tabindex="0">
              <table class="router-table compact">
                <thead><tr><th scope="col">"MAC address"</th><th scope="col" class="num">"Download Mbps"</th><th scope="col" class="num">"Upload Mbps"</th><th scope="col">"Name"</th></tr></thead>
                <tbody>{l.dhcp_clients.iter().map(|o|view!{<tr>
                  <th scope="row"><code>{o.mac.clone()}</code></th><td class="num">{o.download_mbps}</td><td class="num">{o.upload_mbps}</td><td>{o.name.clone().unwrap_or_default()}</td>
                </tr>}).collect_view()}</tbody>
              </table>
            </div>
          }.into_any()
        })}
        {truncated.then(||view!{<p class="notice" role="status">"This router has more client overrides than the page can load. Edit them in the TUI, so no entry is lost."</p>})}
        <div class="form-actions start">
          <button type="button" class="secondary" on:click=open
            disabled=move||truncated||c.inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty()))>"Edit client overrides"</button>
        </div>
      }>
        <form class="router-form inline clients-form" novalidate on:submit=move|ev|{ev.prevent_default();save();}>
          <Show when=move||!errors.with(Vec::is_empty)>
            <div class="form-failure" role="alert"><p>"Correct these rows, then save again:"</p>
              <ul>{move||errors.get().into_iter().map(|(i,m)|view!{<li>{format!("Row {}: {m}",i+1)}</li>}).collect_view()}</ul></div>
          </Show>
          <ol class="client-rows">
            <For each=move||rows.get() key=|(k,_)|*k children=move|(k,_)|{
              let (mac,set_mac)=field(k,|r|r.mac.clone(),|r,v|r.mac=v);
              let (down,set_down)=field(k,|r|r.down.clone(),|r,v|r.down=v);
              let (up,set_up)=field(k,|r|r.up.clone(),|r,v|r.up=v);
              let (name,set_name)=field(k,|r|r.name.clone(),|r,v|r.name=v);
              let invalid=move||{let pos=rows.with(|r|r.iter().position(|(x,_)|*x==k));errors.with(|e|e.iter().any(|(i,_)|Some(*i)==pos)).to_string()};
              view!{<li class="client-row">
                <div class="field"><label for=format!("client-mac-{k}")>"MAC address"</label>
                  <input id=format!("client-mac-{k}") type="text" autocomplete="off" spellcheck="false" maxlength="17" aria-invalid=invalid prop:value=mac on:input=set_mac/></div>
                <div class="field"><label for=format!("client-down-{k}")>"Download Mbps"</label>
                  <input id=format!("client-down-{k}") type="text" inputmode="decimal" autocomplete="off" class="short" prop:value=down on:input=set_down/></div>
                <div class="field"><label for=format!("client-up-{k}")>"Upload Mbps"</label>
                  <input id=format!("client-up-{k}") type="text" inputmode="decimal" autocomplete="off" class="short" prop:value=up on:input=set_up/></div>
                <div class="field"><label for=format!("client-name-{k}")>"Name (optional)"</label>
                  <input id=format!("client-name-{k}") type="text" autocomplete="off" maxlength="64" prop:value=name on:input=set_name/></div>
                <button type="button" class="quiet" aria-label=move||format!("Remove the override {}",rows.with(|r|r.iter().find(|(x,_)|*x==k).map(|(_,row)|row.mac.clone()).unwrap_or_default()))
                  on:click=move|_|rows.update(|r|r.retain(|(x,_)|*x!=k))>"Remove"</button>
              </li>}
            }/>
          </ol>
          <button type="button" class="secondary" on:click=add>"Add override"</button>
          <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
          <div class="form-actions start">
            <button type="submit" class="primary" disabled=move||busy.get()>{move||if busy.get(){"Saving…"}else{"Save client overrides"}}</button>
            <button type="button" class="quiet" on:click=move|_|editing.set(false)>"Cancel"</button>
          </div>
        </form>
      </Show>
    }
}

fn mode_label(m: &str) -> &'static str {
    match m {
        "per_plan" => "One node for each plan",
        _ => "One node for each server",
    }
}

#[component]
fn Servers(id: String, lists: RouterLists) -> impl IntoView {
    let c: Ctx = ctx();
    let editing = RwSignal::new(false);
    let default_mode = RwSignal::new(String::new());
    let rows = RwSignal::new(Vec::<(u32, ServerRow)>::new());
    let next = RwSignal::new(0u32);
    let errors = RwSignal::new(Vec::<(usize, String)>::new());
    let failure = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let digest = RwSignal::new(String::new());
    let l = StoredValue::new(lists);
    let id = StoredValue::new(id);
    let open = move |_| {
        let list: Vec<(u32, ServerRow)> = l.with_value(|l| {
            l.dhcp_servers
                .iter()
                .enumerate()
                .map(|(i, s)| (i as u32, ServerRow::from(s)))
                .collect()
        });
        next.set(list.len() as u32);
        rows.set(list);
        default_mode.set(l.with_value(|l| l.dhcp_server_mode.clone()));
        errors.set(Vec::new());
        failure.set(String::new());
        digest.set(loaded_digest(c));
        editing.set(true);
    };
    let add = move |_| {
        let k = next.get_untracked();
        next.set(k + 1);
        let mode = default_mode.get_untracked();
        rows.update(|r| {
            r.push((
                k,
                ServerRow {
                    mode,
                    ..Default::default()
                },
            ))
        });
        request_animation_frame(move || {
            if let Some(e) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&format!("server-name-{k}")))
                .and_then(|e| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(e).ok())
            {
                let _ = e.focus();
            }
        });
    };
    let save = move || {
        if busy.get_untracked() {
            return;
        }
        let list: Vec<ServerRow> =
            rows.with_untracked(|r| r.iter().map(|(_, row)| row.clone()).collect());
        let checked = match servers(&list) {
            Ok(v) => v,
            Err(e) => {
                errors.set(e);
                return;
            }
        };
        errors.set(Vec::new());
        let mode = default_mode.get_untracked();
        let (same_servers, same_mode) =
            l.with_value(|l| (l.dhcp_servers == checked, l.dhcp_server_mode == mode));
        if same_servers && same_mode {
            failure.set("There is no change to save.".into());
            return;
        }
        busy.set(true);
        failure.set(String::new());
        let id = id.get_value();
        let edit = RouterEdit {
            expected_digest: digest.get_untracked(),
            dhcp_server_mode: (!same_mode).then_some(mode),
            dhcp_servers: (!same_servers).then_some(checked),
            ..Default::default()
        };
        leptos::task::spawn_local(async move {
            match send(
                c,
                Some(id.clone()),
                RouterEditForm {
                    edit,
                    current_password: None,
                },
            )
            .await
            {
                Ok(reply) => {
                    editing.set(false);
                    follow(c, saved(id, reply, "DHCP server settings saved."));
                }
                Err(f) => failure.set(f.message),
            }
            busy.try_set(false);
        });
    };
    let field = move |k: u32, get: fn(&ServerRow) -> String, put: fn(&mut ServerRow, String)| {
        (
            move || {
                rows.with(|r| {
                    r.iter()
                        .find(|(x, _)| *x == k)
                        .map(|(_, row)| get(row))
                        .unwrap_or_default()
                })
            },
            move |ev: leptos::ev::Event| {
                let v = event_target_value(&ev);
                rows.update(|r| {
                    if let Some((_, row)) = r.iter_mut().find(|(x, _)| *x == k) {
                        put(row, v);
                    }
                })
            },
        )
    };
    let number = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or("Default".into());
    view! {
      <h3>"DHCP server settings"</h3>
      <Show when=move||editing.get() fallback=move||view!{
        <p class="field-help">{l.with_value(|l|format!("A server without its own settings: {}.",mode_label(&l.dhcp_server_mode)))}</p>
        {l.with_value(|l| if l.dhcp_servers.is_empty() {
          view!{<p class="router-empty">"No server has its own settings."</p>}.into_any()
        } else {
          view!{
            <div class="table-scroll" role="region" aria-label="DHCP server settings" tabindex="0">
              <table class="router-table compact">
                <thead><tr><th scope="col">"Server"</th><th scope="col">"Mode"</th><th scope="col">"Speed sources"</th><th scope="col" class="num">"Plan down / up Mbps"</th><th scope="col" class="num">"Factors down / up"</th></tr></thead>
                <tbody>{l.dhcp_servers.iter().map(|s|view!{<tr>
                  <th scope="row">{s.name.clone()}</th><td>{mode_label(&s.mode)}</td>
                  <td>{s.speed_sources.as_ref().map(|v|v.join(", ")).unwrap_or("Mode default".into())}</td>
                  <td class="num">{format!("{} / {}",number(s.default_plan_down_mbps),number(s.default_plan_up_mbps))}</td>
                  <td class="num">{format!("{} / {}",number(s.download_factor),number(s.upload_factor))}</td>
                </tr>}).collect_view()}</tbody>
              </table>
            </div>
          }.into_any()
        })}
        <div class="form-actions start">
          <button type="button" class="secondary" on:click=open
            disabled=move||c.inventory.with(|i|i.as_ref().is_none_or(|i|i.config_digest.is_empty()))>"Edit DHCP server settings"</button>
        </div>
      }>
        <form class="router-form inline servers-form" novalidate on:submit=move|ev|{ev.prevent_default();save();}>
          <div class="field">
            <label for="server-default-mode">"Mode of a server without its own settings"</label>
            <select id="server-default-mode" prop:value=move||default_mode.get() on:change=move|ev|default_mode.set(event_target_value(&ev))>
              <option value="per_site">"One node for each server (per_site)"</option>
              <option value="per_plan">"One node for each plan (per_plan)"</option>
            </select>
          </div>
          <Show when=move||!errors.with(Vec::is_empty)>
            <div class="form-failure" role="alert"><p>"Correct these rows, then save again:"</p>
              <ul>{move||errors.get().into_iter().map(|(i,m)|view!{<li>{format!("Row {}: {m}",i+1)}</li>}).collect_view()}</ul></div>
          </Show>
          <ol class="client-rows">
            <For each=move||rows.get() key=|(k,_)|*k children=move|(k,_)|{
              let (name,set_name)=field(k,|r|r.name.clone(),|r,v|r.name=v);
              let (mode,set_mode)=field(k,|r|r.mode.clone(),|r,v|r.mode=v);
              let (src,set_src)=field(k,|r|r.sources.clone(),|r,v|r.sources=v);
              let (down,set_down)=field(k,|r|r.down.clone(),|r,v|r.down=v);
              let (up,set_up)=field(k,|r|r.up.clone(),|r,v|r.up=v);
              let (df,set_df)=field(k,|r|r.down_factor.clone(),|r,v|r.down_factor=v);
              let (uf,set_uf)=field(k,|r|r.up_factor.clone(),|r,v|r.up_factor=v);
              let invalid=move||{let pos=rows.with(|r|r.iter().position(|(x,_)|*x==k));errors.with(|e|e.iter().any(|(i,_)|Some(*i)==pos)).to_string()};
              view!{<li class="client-row server-row">
                <div class="field"><label for=format!("server-name-{k}")>"Server name"</label>
                  <input id=format!("server-name-{k}") type="text" autocomplete="off" spellcheck="false" maxlength="64" aria-invalid=invalid prop:value=name on:input=set_name/></div>
                <div class="field"><label for=format!("server-mode-{k}")>"Mode"</label>
                  <select id=format!("server-mode-{k}") prop:value=mode on:change=set_mode>
                    <option value="per_site">"per_site"</option><option value="per_plan">"per_plan"</option>
                  </select></div>
                <div class="field wide"><label for=format!("server-sources-{k}")>"Speed sources, in order"</label>
                  <input id=format!("server-sources-{k}") type="text" autocomplete="off" spellcheck="false" placeholder="Mode default" aria-describedby="server-sources-help" prop:value=src on:input=set_src/></div>
                <div class="field"><label for=format!("server-down-{k}")>"Plan down Mbps"</label>
                  <input id=format!("server-down-{k}") type="text" inputmode="decimal" class="short" placeholder="Default" prop:value=down on:input=set_down/></div>
                <div class="field"><label for=format!("server-up-{k}")>"Plan up Mbps"</label>
                  <input id=format!("server-up-{k}") type="text" inputmode="decimal" class="short" placeholder="Default" prop:value=up on:input=set_up/></div>
                <div class="field"><label for=format!("server-df-{k}")>"Down factor"</label>
                  <input id=format!("server-df-{k}") type="text" inputmode="decimal" class="short" placeholder="Default" prop:value=df on:input=set_df/></div>
                <div class="field"><label for=format!("server-uf-{k}")>"Up factor"</label>
                  <input id=format!("server-uf-{k}") type="text" inputmode="decimal" class="short" placeholder="Default" prop:value=uf on:input=set_uf/></div>
                <button type="button" class="quiet" aria-label=move||format!("Remove the server settings {}",rows.with(|r|r.iter().find(|(x,_)|*x==k).map(|(_,row)|row.name.clone()).unwrap_or_default()))
                  on:click=move|_|rows.update(|r|r.retain(|(x,_)|*x!=k))>"Remove"</button>
              </li>}
            }/>
          </ol>
          <p class="field-help" id="server-sources-help">"Speed sources: server_comment, lease_comment, settings, separated by commas, in the order to try. Empty uses the default of the mode. per_site cannot use lease_comment."</p>
          <button type="button" class="secondary" on:click=add>"Add server settings"</button>
          <Show when=move||!failure.get().is_empty()><p class="form-failure" role="alert">{move||failure.get()}</p></Show>
          <div class="form-actions start">
            <button type="submit" class="primary" disabled=move||busy.get()>{move||if busy.get(){"Saving…"}else{"Save DHCP server settings"}}</button>
            <button type="button" class="quiet" on:click=move|_|editing.set(false)>"Cancel"</button>
          </div>
        </form>
      </Show>
    }
}
