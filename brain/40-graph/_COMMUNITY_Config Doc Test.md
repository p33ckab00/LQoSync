---
type: community
cohesion: 1.00
members: 2
---

# Config Doc Test

**Cohesion:** 1.00 - tightly connected
**Members:** 2 nodes

## Members
- [[config_doc.rs]] - code - tests/config_doc.rs
- [[example_in_configuration_md_parses()]] - code - tests/config_doc.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Config_Doc_Test
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Config Loading]]

## Top bridge nodes
- [[config_doc.rs]] - degree 2, connects to 1 community