# Planning knowledge graph

`graph.json`, `graph.html`, and `GRAPH_REPORT.md` describe the LQoSync4 planning documents. Graphify builds them from a manually reviewed semantic map. This is a planning graph, not a graph of LQoSync4 source code.

Open `graph.html` locally or query `graph.json` with Graphify. The graph includes the [LibreQoS v3 flow](../docs/LIBREQOS_V3_DATA_FLOW.md), [route/component contract](../docs/ROUTES_AND_COMPONENTS.md), [visual/responsive map](../docs/UI_VISUAL_SYSTEM.md), and [agent tandem workflow](../brain/AGENT_WORKFLOW.md). To refresh after a planning change, review its concepts and relationships in `refresh_graph.py`, then run:

```bash
/home/pi/.local/share/uv/tools/graphifyy/bin/python docs/lqosync4/graphify-out/refresh_graph.py
```

The refresh script uses the current `graph.json` as its baseline and adds or updates the relationships it declares. Keep the baseline graph when updating; do not treat the script as an automatic extraction of every document sentence.
