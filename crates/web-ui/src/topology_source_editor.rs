//! Typed local policy editing. Persistence belongs to the Studio save task.
use crate::topology_studio_model::*;
use leptos::prelude::*;
use web_contract::source_studio::*;
use web_contract::topology::TopologySummary;

fn parse<T: serde::de::DeserializeOwned>(s: String) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(s)).ok()
}
fn edit(draft: RwSignal<Option<StudioDraft>>, f: impl FnOnce(&mut PolicyDraft)) {
    draft.update(|d| {
        if let Some(d) = d {
            d.edit(f)
        }
    });
}
fn current(draft: RwSignal<Option<StudioDraft>>) -> Option<PolicyDraft> {
    draft.with(|d| d.as_ref().and_then(|d| d.current().cloned()))
}
fn change_filter(
    draft: RwSignal<Option<StudioDraft>>,
    path: &[usize],
    f: impl FnOnce(&mut FilterExpr),
) {
    edit(draft, |p| {
        if let Some(q) = filter_at_mut(&mut p.policy.filter, path) {
            f(q)
        }
    });
}
fn glyph(path: &'static str) -> impl IntoView {
    crate::topology_canvas::icon(path)
}

#[component]
fn FilterTree(
    draft: RwSignal<Option<StudioDraft>>,
    caps: StudioCapabilities,
    source: SourceKind,
    path: Vec<usize>,
) -> impl IntoView {
    let path = StoredValue::new(path);
    let caps = StoredValue::new(caps);
    let query = move || {
        current(draft).and_then(|p| filter_at(&p.policy.filter, &path.get_value()).cloned())
    };
    let shape = Memo::new(move |_| match query() {
        Some(FilterExpr::All { rules }) => (0, rules.len()),
        Some(FilterExpr::Any { rules }) => (1, rules.len()),
        Some(FilterExpr::Match { .. }) => (2, 0),
        None => (3, 0),
    });
    view! {<div class="studio-filter">
      {move||match shape.get().0{
        0|1=>view!{
          <div class="studio-inline">
            <label>"Match rules"<select aria-label="Match rules" prop:value=move||if shape.get().0==0{"all"}else{"any"}
              on:change=move|ev|{let all=event_target_value(&ev)=="all";change_filter(draft,&path.get_value(),|q|{let rules=match q{FilterExpr::All{rules}|FilterExpr::Any{rules}=>std::mem::take(rules),_=>vec![]};*q=if all{FilterExpr::All{rules}}else{FilterExpr::Any{rules}};});}>
              <option value="all">"All"</option><option value="any">"Any"</option></select></label>
            <button type="button" class="canvas-tool" aria-label="Add query rule" title="Add query rule" disabled=move||current(draft).is_none_or(|p|filter_leaves(&p.policy.filter)>=caps.with_value(|c|c.limits.filter_leaves.min(MAX_FILTER_LEAVES as u64) as usize))
              on:click=move|_|{let next=caps.with_value(|c|fields(c,source,false).first().copied().and_then(|field|operators(c,source,field).first().copied().map(|op|FilterExpr::Match{field,op,value:FilterValue::Text{value:String::new()}})));if let Some(next)=next{change_filter(draft,&path.get_value(),|q|if let FilterExpr::All{rules}|FilterExpr::Any{rules}=q{rules.push(next)});}}>{glyph("M12 5v14M5 12h14")}</button>
            <button type="button" class="canvas-tool" aria-label="Add query group" title="Add query group" disabled=move||path.with_value(|p|p.len()+2>=caps.with_value(|c|c.limits.filter_depth.min(MAX_FILTER_DEPTH as u64) as usize))
              on:click=move|_|change_filter(draft,&path.get_value(),|q|if let FilterExpr::All{rules}|FilterExpr::Any{rules}=q{rules.push(FilterExpr::All{rules:vec![]})})>{glyph("M3 5h6v14H3M15 5h6v14h-6")}</button>
          </div>
          <Show when=move||shape.get().1==0><p class="field-help">"No filter rules: select all records in this member set."</p></Show>
          <For each={move||(0..shape.get().1).collect::<Vec<_>>()} key=|i|*i children=move|i|{
            let mut child=path.get_value();child.push(i);
            view!{<div class="studio-filter-child"><FilterTree draft caps=caps.get_value() source path=child/>
              <button type="button" class="canvas-tool" aria-label="Remove query rule" title="Remove query rule" on:click=move|_|change_filter(draft,&path.get_value(),|q|if let FilterExpr::All{rules}|FilterExpr::Any{rules}=q&&i<rules.len(){rules.remove(i);})>{glyph("M6 6l12 12M18 6 6 18")}</button></div>}
          }/>
        }.into_any(),
        2=>{
          let field=move||match query(){Some(FilterExpr::Match{field,..})=>field,_=>FieldKey::AccountProfile};
          view!{<div class="studio-filter-leaf">
            <label>"Query field"<select aria-label="Query field" prop:value=move||key(&field()) on:change={move|ev|if let Some(field)=parse::<FieldKey>(event_target_value(&ev)){
                let op=caps.with_value(|c|operators(c,source,field).first().copied());if let Some(op)=op{change_filter(draft,&path.get_value(),|q|*q=FilterExpr::Match{field,op,value:FilterValue::Text{value:String::new()}});}
              }}>
              {move||{let fs=caps.with_value(|c|fields(c,source,false));let saved=field();view!{
                {(!fs.contains(&saved)).then(||view!{<option value=key(&saved) disabled>{format!("{} (unsupported)",key(&saved))}</option>})}
                {fs.into_iter().map(|f|view!{<option value=key(&f)>{key(&f)}</option>}).collect_view()}
              }}}
            </select></label>
            <label>"Operator"<select aria-label="Operator" prop:value=move||match query(){Some(FilterExpr::Match{op,..})=>key(&op),_=>String::new()}
              on:change={move|ev|if let Some(next)=parse::<FilterOp>(event_target_value(&ev)){change_filter(draft,&path.get_value(),|q|if let FilterExpr::Match{op,value,..}=q{*op=next;let text=match value{FilterValue::Text{value}=>value.clone(),FilterValue::Texts{values}=>values.join(",")};*value=if next==FilterOp::In{FilterValue::Texts{values:text.split(',').map(|v|v.trim().to_string()).collect()}}else{FilterValue::Text{value:text}};});}}>
              {move||{let allowed=caps.with_value(|c|operators(c,source,field()));let saved=match query(){Some(FilterExpr::Match{op,..})=>Some(op),_=>None};view!{
                {saved.filter(|op|!allowed.contains(op)).map(|op|view!{<option value=key(&op) disabled>{format!("{} (unsupported)",key(&op))}</option>})}
                {allowed.into_iter().map(|op|view!{<option value=key(&op)>{key(&op)}</option>}).collect_view()}
              }}}
            </select></label>
            <label>"Query value"<input aria-label="Query value" prop:value=move||match query(){Some(FilterExpr::Match{value:FilterValue::Text{value},..})=>value,Some(FilterExpr::Match{value:FilterValue::Texts{values},..})=>values.join(","),_=>String::new()}
              on:input=move|ev|{let text=event_target_value(&ev);change_filter(draft,&path.get_value(),|q|if let FilterExpr::Match{op,value,..}=q{*value=if *op==FilterOp::In{FilterValue::Texts{values:text.split(',').map(|v|v.trim().to_string()).collect()}}else{FilterValue::Text{value:text}};});}/></label>
          </div>}.into_any()
        },_=>().into_any()
      }}
    </div>}
}

#[component]
fn PlanChoice(
    label: &'static str,
    data: RwSignal<Option<TopologySummary>>,
    value: Signal<String>,
    changed: Callback<String>,
) -> impl IntoView {
    view! {<label>{label}<select aria-label=label prop:value=move||value.get() on:change=move|ev|changed.run(event_target_value(&ev))>
      <option value="">"Select a saved speed plan"</option>
      {move||{let selected=value.get();let plans=data.with(|d|d.as_ref().map(|d|d.plans.clone()).unwrap_or_default());view!{
        {(!selected.is_empty()&&!plans.iter().any(|p|p.id==selected)).then(||view!{<option value=selected.clone() disabled>"Saved plan unavailable"</option>})}
        {plans.into_iter().map(|p|view!{<option value=p.id>{format!("{} · {} / {} Mbps",p.name,p.down_mbps,p.up_mbps)}</option>}).collect_view()}
      }}}
    </select></label>}
}

#[component]
fn RateRow(
    limit: usize,
    draft: RwSignal<Option<StudioDraft>>,
    index: usize,
    data: RwSignal<Option<TopologySummary>>,
) -> impl IntoView {
    let rate = move || current(draft).and_then(|p| p.policy.rates.get(index).cloned());
    let bindings = move || match rate() {
        Some(RateSource::ProfileMap { bindings } | RateSource::ServerMap { bindings }) => bindings,
        _ => vec![],
    };
    let map = matches!(
        rate(),
        Some(RateSource::ProfileMap { .. } | RateSource::ServerMap { .. })
    );
    let fallback = matches!(rate(), Some(RateSource::Fallback { .. }));
    view! {<li><div class="studio-inline"><strong class="studio-rate-label">{move||rate().as_ref().map(rate_label)}</strong>
      <button type="button" class="canvas-tool" aria-label="Move rate up" title="Move rate up" disabled=index==0 on:click=move|_|draft.update(|d|if let Some(d)=d{d.move_rate(index,-1)})>{glyph("M6 14l6-6 6 6")}</button>
      <button type="button" class="canvas-tool" aria-label="Move rate down" title="Move rate down" disabled=move||current(draft).is_none_or(|p|index+1>=p.policy.rates.len()) on:click=move|_|draft.update(|d|if let Some(d)=d{d.move_rate(index,1)})>{glyph("M6 10l6 6 6-6")}</button>
      <button type="button" class="canvas-tool" aria-label="Remove rate source" title="Remove rate source" on:click=move|_|edit(draft,|p|{if index<p.policy.rates.len(){p.policy.rates.remove(index);}})>{glyph("M6 6l12 12M18 6 6 18")}</button>
    </div>
    {map.then(||view!{
      <For each={move||(0..bindings().len()).collect::<Vec<_>>()} key=|i|*i children=move|i|view!{
        <div class="studio-inline"><label>"Exact selector"<input aria-label="Exact selector" prop:value=move||bindings().get(i).map(|b|b.selector.clone()).unwrap_or_default()
          on:input=move|ev|{let value=event_target_value(&ev);edit(draft,|p|if let Some(RateSource::ProfileMap{bindings}|RateSource::ServerMap{bindings})=p.policy.rates.get_mut(index)&&let Some(b)=bindings.get_mut(i){b.selector=value;});}/></label>
        <PlanChoice label="Speed plan" data value=Signal::derive(move||bindings().get(i).map(|b|b.plan_id.clone()).unwrap_or_default())
          changed=Callback::new(move|value:String|edit(draft,|p|if let Some(RateSource::ProfileMap{bindings}|RateSource::ServerMap{bindings})=p.policy.rates.get_mut(index)&&let Some(b)=bindings.get_mut(i){b.plan_id=value;}))/>

        <button type="button" class="canvas-tool" aria-label="Remove plan binding" title="Remove plan binding" on:click=move|_|edit(draft,|p|if let Some(RateSource::ProfileMap{bindings}|RateSource::ServerMap{bindings})=p.policy.rates.get_mut(index)&&i<bindings.len(){bindings.remove(i);})>{glyph("M6 6l12 12M18 6 6 18")}</button></div>
      }/>
      <button type="button" class="canvas-tool" aria-label="Add plan binding" title="Add plan binding" disabled=move||current(draft).is_none_or(|p|binding_count(&p.policy)>=limit.min(MAX_PLAN_BINDINGS))
        on:click=move|_|edit(draft,|p|if let Some(RateSource::ProfileMap{bindings}|RateSource::ServerMap{bindings})=p.policy.rates.get_mut(index){bindings.push(PlanBinding{selector:String::new(),plan_id:String::new()});})>{glyph("M12 5v14M5 12h14")}</button>
    })}
    {fallback.then(||view!{<PlanChoice label="Fallback speed plan" data value=Signal::derive(move||match rate(){Some(RateSource::Fallback{plan_id})=>plan_id,_=>String::new()})
      changed=Callback::new(move|value:String|edit(draft,|p|if let Some(RateSource::Fallback{plan_id})=p.policy.rates.get_mut(index){*plan_id=value;}))/>})}

    </li>}
}

#[component]
pub(crate) fn SourceEditor(
    draft: RwSignal<Option<StudioDraft>>,
    caps: StudioCapabilities,
    data: RwSignal<Option<TopologySummary>>,
) -> impl IntoView {
    let caps = StoredValue::new(caps);
    let selection = Memo::new(move |_| {
        draft.with(|d| {
            d.as_ref().and_then(|d| {
                d.selected
                    .map(|i| (d.router_id.clone(), i, d.policies[i].policy.source))
            })
        })
    });
    view! {<div class="studio-source-editor">
      <nav class="studio-source-rail" aria-label="Source type">
        {SOURCES.into_iter().enumerate().map(move|(i,(kind,label))|view!{
          <button type="button" class="canvas-tool" aria-label=label title=if caps.with_value(|c|fields(c,kind,false).is_empty()){format!("{label}: no query fields advertised")}else{label.to_string()} aria-pressed=move||selection.get().is_some_and(|(_,_,s)|s==kind).to_string()
            disabled=caps.with_value(|c|fields(c,kind,false).is_empty()) on:click=move|_|draft.update(|d|if let Some(d)=d{d.select_source(kind,&caps.get_value())})>{glyph(["M4 8h16M4 16h16M8 4v16M16 4v16","M4 5h16v14H4zM8 9h8M8 13h5","M5 10a10 10 0 0 1 14 0M8 13a6 6 0 0 1 8 0M11 16h2","M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18M3 12h18M12 3v18"][i])}</button>
        }).collect_view()}
      </nav>
      {move||selection.get().map(|(_,_,source)|{
        let group_mode=Memo::new(move |_|current(draft).map(|p|match p.policy.group{GroupRule::ByField{..}=>"field",GroupRule::Fixed{..}=>"fixed",GroupRule::Attach{..}=>"attach"}).unwrap_or("fixed"));
        let capacity_mode=Memo::new(move |_|current(draft).map(|p|match p.policy.capacity{CapacityRule::Fixed{..}=>"fixed",_=>"plans"}).unwrap_or("plans"));
        view!{<div class="studio-policy-form">
          <h3>{source_label(source)}</h3>
          <div class="studio-inline"><label>"Selected policy"<select aria-label="Selected policy" prop:value=move||draft.with(|d|d.as_ref().and_then(|d|d.selected)).map(|i|i.to_string()).unwrap_or_default()
            on:change={move|ev|if let Ok(i)=event_target_value(&ev).parse::<usize>(){draft.update(|d|if let Some(d)=d&&i<d.policies.len(){d.selected=Some(i);});}}>
            {move||draft.with(|d|d.as_ref().map(|d|d.policies.iter().enumerate().filter(|(_,p)|p.policy.source==source).map(|(i,p)|view!{<option value=i.to_string()>{p.policy.name.clone()}</option>}).collect_view()))}
          </select></label>
          <button type="button" class="canvas-tool" aria-label="Add source policy" title="Add source policy" disabled=move||draft.with(|d|d.as_ref().is_none_or(|d|d.policies.len()>=caps.with_value(|c|c.limits.policies_per_router.min(MAX_POLICIES as u64) as usize))) on:click=move|_|draft.update(|d|if let Some(d)=d{d.add_policy(source,&caps.get_value());})>{glyph("M12 5v14M5 12h14")}</button></div>
          <button type="button" class="canvas-tool" aria-label="Remove source policy" title="Remove this policy from the local draft; save explicitly to persist the set" on:click=move|_|draft.update(|d|if let Some(d)=d&&let Some(i)=d.selected&&i<d.policies.len(){d.policies.remove(i);d.selected=if d.policies.is_empty(){None}else{Some(i.min(d.policies.len()-1))};d.epoch=d.epoch.wrapping_add(1);})>{glyph("M6 6l12 12M18 6 6 18")}</button>
          <label>"Policy name"<input aria-label="Policy name" maxlength="128" prop:value=move||current(draft).map(|p|p.policy.name).unwrap_or_default() on:input=move|ev|{let v=event_target_value(&ev);edit(draft,|p|p.policy.name=v);}/></label>
          <div class="studio-inline"><label class="check"><input type="checkbox" aria-label="Enable policy" prop:checked=move||current(draft).is_some_and(|p|p.policy.enabled) on:change=move|ev|{let v=event_target_checked(&ev);edit(draft,|p|p.policy.enabled=v);}/>"Enable policy"</label>
          <label>"Priority"<input aria-label="Priority" inputmode="numeric" prop:value=move||current(draft).map(|p|p.priority).unwrap_or_default() on:input=move|ev|{let v=event_target_value(&ev);edit(draft,|p|p.priority=v);}/></label></div>
          <label>"Read access profile"<select aria-label="Read access profile" prop:value=move||current(draft).map(|p|p.policy.read_access_ref).unwrap_or_default() on:change=move|ev|{let v=event_target_value(&ev);edit(draft,|p|p.policy.read_access_ref=v);}>
            {move||draft.with(|d|d.as_ref().map(|d|d.profiles.iter().map(|p|view!{<option value=p.id.clone()>{p.label.clone()}</option>}).collect_view()))}
          </select></label>
          <label>"Member set"<select aria-label="Member set" prop:value=move||current(draft).map(|p|key(&p.policy.members)).unwrap_or_default() on:change={move|ev|if let Some(v)=parse(event_target_value(&ev)){edit(draft,|p|p.policy.members=v);}}>
            <option value="active">"Active records"</option><option value="configured">"Configured records"</option><option value="joined">"Active records joined to accounts"</option></select></label>
          <h4>"Typed query"</h4><p class="field-help">"Only advertised fields and operators appear. For in, separate exact values with commas. No RouterOS script runs."</p>
          <FilterTree draft caps=caps.get_value() source path=vec![]/>
          <h4>"Node grouping"</h4>
          <label>"Group mode"<select aria-label="Group mode" prop:value=move||group_mode.get() on:change=move|ev|{let mode=event_target_value(&ev);edit(draft,|p|p.policy.group=match mode.as_str(){"field"=>GroupRule::ByField{field:caps.with_value(|c|fields(c,source,true))[0]},"attach"=>GroupRule::Attach{node_id:String::new()},_=>GroupRule::Fixed{name:String::new()}});}>
            <option value="field" disabled=caps.with_value(|c|fields(c,source,true).is_empty())>"Group by field"</option><option value="fixed">"One named node"</option><option value="attach">"Attach to saved node"</option></select></label>
          {move||match group_mode.get(){"field"=>view!{<label>"Group field"<select aria-label="Group field" prop:value=move||match current(draft).map(|p|p.policy.group){Some(GroupRule::ByField{field})=>key(&field),_=>String::new()} on:change={move|ev|if let Some(field)=parse(event_target_value(&ev)){edit(draft,|p|p.policy.group=GroupRule::ByField{field});}}>
              {caps.with_value(|c|fields(c,source,true)).into_iter().map(|f|view!{<option value=key(&f)>{key(&f)}</option>}).collect_view()}
            </select></label>}.into_any(),"attach"=>view!{<label>"Group node"<select aria-label="Group node" prop:value=move||match current(draft).map(|p|p.policy.group){Some(GroupRule::Attach{node_id})=>node_id,_=>String::new()} on:change=move|ev|{let node_id=event_target_value(&ev);edit(draft,|p|p.policy.group=GroupRule::Attach{node_id});}>
              <option value="">"Select a saved node"</option>{move||data.with(|d|d.as_ref().map(|d|d.nodes.iter().filter(|n|n.active).map(|n|view!{<option value=n.id.clone()>{n.name.clone()}</option>}).collect_view()))}
            </select></label>}.into_any(),_=>view!{<label>"Group name"<input aria-label="Group name" prop:value=move||match current(draft).map(|p|p.policy.group){Some(GroupRule::Fixed{name})=>name,_=>String::new()} on:input=move|ev|{let name=event_target_value(&ev);edit(draft,|p|p.policy.group=GroupRule::Fixed{name});}/></label>}.into_any()}}
          <label>"Parent node"<select aria-label="Parent node" prop:value=move||match current(draft).map(|p|p.policy.parent){Some(ParentRule::Node{node_id})=>node_id,_=>String::new()} on:change=move|ev|{let node_id=event_target_value(&ev);edit(draft,|p|p.policy.parent=if node_id.is_empty(){ParentRule::Router}else{ParentRule::Node{node_id}});}>
            <option value="">"This router"</option>{move||data.with(|d|d.as_ref().map(|d|d.nodes.iter().filter(|n|n.active).map(|n|view!{<option value=n.id.clone()>{n.name.clone()}</option>}).collect_view()))}
          </select></label>
          <h4>"Speed plan precedence"</h4><p class="field-help">"The first rate source that resolves wins. Exact selectors map to saved speed plans. The service validates compatibility."</p>
          <ol class="studio-rates"><For each={move||current(draft).map(|p|p.policy.rates.iter().enumerate().map(|(i,r)|(i,rate_label(r))).collect::<Vec<_>>()).unwrap_or_default()} key=|x|*x children=move|(index,_)|view!{<RateRow draft index data limit=caps.with_value(|c|c.limits.plan_bindings as usize)/>}/></ol>
          <label>"Add rate source"<select aria-label="Add rate source" prop:value="" on:change={move|ev|if let Ok(i)=event_target_value(&ev).parse::<usize>()&&let Some(r)=caps.with_value(|c|rate_choices(source,c)).get(i).cloned(){edit(draft,|p|p.policy.rates.push(r));}}>
            <option value="">"Select a rate source"</option>{caps.with_value(|c|rate_choices(source,c)).into_iter().enumerate().map(|(i,r)|view!{<option value=i.to_string()>{rate_label(&r)}</option>}).collect_view()}</select></label>
          <h4>"Bandwidth capacity"</h4><label>"Capacity mode"<select aria-label="Capacity mode" prop:value=move||capacity_mode.get() on:change=move|ev|{let fixed=event_target_value(&ev)=="fixed";edit(draft,|p|p.policy.capacity=if fixed{CapacityRule::Fixed{down_bps:String::new(),up_bps:String::new()}}else{CapacityRule::FromPlans{down_factor:"1".into(),up_factor:"1".into(),minimum_down_bps:"0".into(),minimum_up_bps:"0".into()}});}>
            <option value="plans">"Sum of plans × directional factors"</option><option value="fixed">"Fixed capacity"</option></select></label>
          {move||{let fixed=capacity_mode.get()=="fixed";let labels=if fixed{vec!["Download capacity (bps)","Upload capacity (bps)"]}else{vec!["Download factor","Upload factor","Minimum download (bps)","Minimum upload (bps)"]};
            view!{<div class="studio-capacity">{labels.into_iter().enumerate().map(move|(i,label)|view!{<label>{label}<input aria-label=label inputmode="decimal" prop:value=move||current(draft).map(|p|match p.policy.capacity{CapacityRule::Fixed{down_bps,up_bps}=>[down_bps,up_bps].get(i).cloned().unwrap_or_default(),CapacityRule::FromPlans{down_factor,up_factor,minimum_down_bps,minimum_up_bps}=>[down_factor,up_factor,minimum_down_bps,minimum_up_bps].get(i).cloned().unwrap_or_default()}).unwrap_or_default()
              on:input=move|ev|{let v=event_target_value(&ev);edit(draft,|p|match &mut p.policy.capacity{CapacityRule::Fixed{down_bps,up_bps}=>{if i==0{*down_bps=v}else{*up_bps=v}},CapacityRule::FromPlans{down_factor,up_factor,minimum_down_bps,minimum_up_bps}=>{let target=match i{0=>down_factor,1=>up_factor,2=>minimum_down_bps,_=>minimum_up_bps};*target=v;}});}/></label>}).collect_view()}</div>}
          }}
          <h4>"Explicit writes"</h4><p class="field-help">"Read credentials do not enable writes. The only version 1 scope is WAN address lists."</p>
          <Show when=move||source==SourceKind::Wan>
            <label>"Write access profile"<select aria-label="Write access profile" prop:value=move||current(draft).and_then(|p|p.policy.writes.first().map(|w|w.access_ref.clone())).unwrap_or_default() on:change=move|ev|{let access_ref=event_target_value(&ev);edit(draft,|p|p.policy.writes=if access_ref.is_empty(){vec![]}else{vec![WriteGrant{access_ref,scope:WriteScope::WanAddressLists,enabled:false}]});}>
              <option value="">"No writes"</option>{move||draft.with(|d|d.as_ref().map(|d|d.profiles.iter().filter(|p|p.supported_writes.contains(&WriteScope::WanAddressLists)).map(|p|view!{<option value=p.id.clone()>{p.label.clone()}</option>}).collect_view()))}
            </select></label><label class="check"><input type="checkbox" aria-label="Enable WAN list writes" disabled=move||current(draft).is_none_or(|p|p.policy.writes.is_empty()) prop:checked=move||current(draft).is_some_and(|p|p.policy.writes.first().is_some_and(|w|w.enabled)) on:change=move|ev|{let enabled=event_target_checked(&ev);edit(draft,|p|if let Some(w)=p.policy.writes.first_mut(){w.enabled=enabled;});}/>"Enable WAN list writes"</label>
          </Show>
          <p class="field-help">"Identity uses the service registry. A dynamic lease needs no comment on the router. These are local drafts until the save task is available."</p>
        </div>}
      })}
    </div>}
}
