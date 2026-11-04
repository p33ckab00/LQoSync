"""Refresh the manually extracted LQoSync4 planning graph with Graphify.

The existing graph is the semantic baseline. Add only relationships supported by
current planning files. Edit this map when a new planning topic is introduced.
"""

import json
import subprocess
from pathlib import Path

from graphify.analyze import god_nodes, suggest_questions, surprising_connections
from graphify.build import build_from_json
from graphify.cluster import cluster, score_all
from graphify.detect import detect
from graphify.export import to_html, to_json
from graphify.report import generate


ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "graphify-out"
raw = json.loads((OUT / "graph.json").read_text())
nodes = {node["id"]: node for node in raw["nodes"]}
edges = list(raw["links"])


def add(key: str, label: str, source: str) -> str:
    path = ROOT / source
    if not path.is_file():
        raise FileNotFoundError(path)
    node_id = f"planning_{key}"
    nodes[node_id] = {
        "id": node_id,
        "label": label,
        "file_type": "concept",
        "source_file": source,
        "source_location": None,
        "source_url": None,
        "_origin": "semantic",
    }
    return node_id


def connect(source: str, target: str, evidence: str) -> None:
    if source not in nodes or target not in nodes:
        raise ValueError(f"missing endpoint: {source}, {target}")
    edge = {
        "source": source,
        "target": target,
        "relation": "documents",
        "confidence": "EXTRACTED",
        "confidence_score": 1.0,
        "source_file": evidence,
        "source_location": None,
        "weight": 1.0,
        "_origin": "semantic",
    }
    if not any(
        {old["source"], old["target"]} == {source, target}
        and old.get("relation") == edge["relation"]
        for old in edges
    ):
        edges.append(edge)


v3 = add("v3_data_flow", "LibreQoS v3 data flow", "docs/LIBREQOS_V3_DATA_FLOW.md")
pair = add("v3_pair", "v3 CSV + network JSON input", "docs/LIBREQOS_V3_DATA_FLOW.md")
native = add("v3_network", "v3 native JSON input", "docs/LIBREQOS_V3_DATA_FLOW.md")
routes = add("route_contract", "Route and component contract", "docs/ROUTES_AND_COMPONENTS.md")
shell = add("app_shell", "AppShell", "docs/ROUTES_AND_COMPONENTS.md")
visual = add("visual_system", "Visual and responsive UI map", "docs/UI_VISUAL_SYSTEM.md")
css = add("css_starter", "CSS starter", "ui-reference.css")
desktop = add("desktop_layout", "Desktop layout", "docs/UI_VISUAL_SYSTEM.md")
mobile = add("mobile_layout", "Mobile layout", "docs/UI_VISUAL_SYSTEM.md")
web_perf = add("web_performance", "Web UI performance and readability", "docs/IMPROVEMENTS.md")
team = add("agent_workflow", "Agent tandem workflow", "brain/AGENT_WORKFLOW.md")
engineering = add("engineering_rules", "Agentic engineering rules", "brain/AGENTIC_ENGINEERING_RULES.md")
completion = add("completion_gate", "Production completion gate", "brain/PRODUCTION_READY_GATE.md")

route_specs = [
    ("dashboard", "/", "docs_product_spec_dashboard"),
    ("routers", "/routers", "docs_product_spec_routers"),
    ("ipoe_subscribers", "/ipoe/subscribers", "docs_product_spec_ipoe"),
    ("ipoe_profiles", "/ipoe/profiles", "docs_product_spec_profiles"),
    ("pppoe_users", "/pppoe/users", "docs_product_spec_pppoe"),
    ("pppoe_profiles", "/pppoe/profiles", "docs_product_spec_profiles"),
    ("libreqos", "/libreqos", "docs_product_spec_libreqos"),
    ("load_balancer", "/load-balancer", "docs_product_spec_load_balancer"),
    ("users", "/users", "docs_product_spec_audit"),
    ("settings", "/settings", "docs_product_spec_web_ui"),
]
for key, label, feature in route_specs:
    route = add(f"route_{key}", label, "docs/ROUTES_AND_COMPONENTS.md")
    connect(routes, route, "docs/ROUTES_AND_COMPONENTS.md")
    connect(route, feature, "docs/ROUTES_AND_COMPONENTS.md")
    connect(shell, route, "docs/ROUTES_AND_COMPONENTS.md")

components = [
    ("summary_cards", "SummaryCards", "route_dashboard"),
    ("health_grid", "HealthGrid", "route_dashboard"),
    ("profile_cards", "ProfileCards", "route_ipoe_profiles"),
    ("assignment_table", "AssignmentTable", "route_load_balancer"),
    ("allocation_cards", "AllocationCards", "route_load_balancer"),
    ("source_preview", "SourcePreview", "route_libreqos"),
]
for key, label, route_key in components:
    component = add(key, label, "docs/ROUTES_AND_COMPONENTS.md")
    connect(component, f"planning_{route_key}", "docs/ROUTES_AND_COMPONENTS.md")
    connect(component, visual, "docs/UI_VISUAL_SYSTEM.md")

for a, b, evidence in [
    (v3, pair, "docs/LIBREQOS_V3_DATA_FLOW.md"),
    (v3, native, "docs/LIBREQOS_V3_DATA_FLOW.md"),
    (v3, "docs_product_spec_libreqos", "docs/LIBREQOS_V3_DATA_FLOW.md"),
    (v3, "docs_feature_function_map_function_map", "docs/FEATURE_FUNCTION_MAP.md"),
    (routes, shell, "docs/ROUTES_AND_COMPONENTS.md"),
    (routes, "docs_feature_function_map_function_map", "docs/FEATURE_FUNCTION_MAP.md"),
    (visual, routes, "docs/ROUTES_AND_COMPONENTS.md"),
    (visual, css, "docs/UI_VISUAL_SYSTEM.md"),
    (visual, desktop, "docs/UI_VISUAL_SYSTEM.md"),
    (visual, mobile, "docs/UI_VISUAL_SYSTEM.md"),
    (visual, web_perf, "docs/UI_VISUAL_SYSTEM.md"),
    (web_perf, "docs_improvements_improvements", "docs/IMPROVEMENTS.md"),
    (web_perf, "docs_product_spec_performance", "docs/PRODUCT_SPEC.md"),
    (team, engineering, "brain/AGENT_WORKFLOW.md"),
    (team, completion, "brain/AGENT_WORKFLOW.md"),
    (team, "brain_decisions_decisions", "brain/DECISIONS.md"),
    (team, "docs_feature_function_map_function_map", "brain/AGENT_WORKFLOW.md"),
    (team, "docs_implementation_plan_phases", "brain/AGENT_WORKFLOW.md"),
    (engineering, "docs_improvements_improvements", "brain/AGENTIC_ENGINEERING_RULES.md"),
    (completion, "ste_requirements_acceptance", "brain/PRODUCTION_READY_GATE.md"),
    ("planning_route_load_balancer", "docs_product_spec_assignments", "docs/ROUTES_AND_COMPONENTS.md"),
    ("planning_route_load_balancer", "docs_product_spec_allocations", "docs/ROUTES_AND_COMPONENTS.md"),
]:
    connect(a, b, evidence)

extraction = {"nodes": list(nodes.values()), "edges": edges, "hyperedges": []}
graph = build_from_json(extraction, root=str(ROOT), directed=False)
communities = cluster(graph)
cohesion = score_all(graph, communities)
labels = {}
for cid, members in communities.items():
    top = max(members, key=lambda node_id: graph.degree(node_id))
    labels[cid] = f"{graph.nodes[top]['label']} and related topics"

commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
if not to_json(graph, communities, str(OUT / "graph.json"), community_labels=labels, built_at_commit=commit):
    raise RuntimeError("Graphify refused to write a smaller graph")
if not to_html(graph, communities, str(OUT / "graph.html"), community_labels=labels):
    raise RuntimeError("Graphify refused to write the HTML graph")
report = generate(
    graph,
    communities,
    cohesion,
    labels,
    god_nodes(graph),
    surprising_connections(graph, communities),
    detect(ROOT),
    {"input": 0, "output": 0},
    str(ROOT),
    suggested_questions=suggest_questions(graph, communities, labels),
    built_at_commit=commit,
)
(OUT / "GRAPH_REPORT.md").write_text(report)
print(f"Graph: {graph.number_of_nodes()} nodes, {graph.number_of_edges()} edges")
for cid, members in communities.items():
    print(f"  {cid}: {labels[cid]} ({len(members)} nodes)")
