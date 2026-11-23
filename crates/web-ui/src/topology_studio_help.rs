//! Optional canvas help. It reads no router and changes no policy.
use leptos::prelude::*;
use web_contract::topology::Node;

#[component]
pub(crate) fn StudioHelp(selected: Option<Node>) -> impl IntoView {
    let context = selected.map(|node| {
        let explanation = match node.kind.as_str() {
            "wan" => "A WAN card has a configured capacity. A source pin does not change MikroTik routing.",
            "group" => "A subscriber plan limits one circuit. Group capacity is a separate total with directional factors and minimums.",
            _ if node.router_id.is_some() => "Exclude from export tree keeps this card and its source reads. Export promotes its child groups.",
            _ => "A parent connection places the group in the shaping hierarchy. Routing remains a separate service operation.",
        };
        view! {<section class="studio-help-context"><h3>{format!("About {}",node.name)}</h3><p>{explanation}</p></section>}
    });
    view! {
      <div class="studio-help">
        <p>"This guide is optional. Close it to return to the same selection, draft, and canvas position."</p>
        {context}
        <h3>"Configuration steps"</h3>
        <ol>
          <li><strong>"Register a router."</strong>" Use Routers for its connection and credentials. Enable collection explicitly."</li>
          <li><strong>"Select sources."</strong>" Use Router inventory for current PPPoE, DHCP/IPoE, Hotspot, and WAN sources."</li>
          <li><strong>"Set groups and rates."</strong>" Keep the subscriber plan separate from node capacity and WAN allocation."</li>
          <li><strong>"Connect parents."</strong>" One card can use several WAN parents. Each circuit keeps one effective shaping parent."</li>
          <li><strong>"Inspect a preview."</strong>" Preview and evidence shows cached observations, conflicts, and publication state. It starts no router read."</li>
          <li><strong>"Check the result."</strong>" Valid edits save automatically. Saved policy, written files, and runtime acceptance are separate states."</li>
        </ol>
        <p>"A failed or partial read does not prove that subscribers disappeared. Check source freshness before a change."</p>
        <p class="field-help">"Router source configuration edits local source-policy drafts on supported backends. Preview and source-policy save are unavailable in this build."</p>
      </div>
    }
}
