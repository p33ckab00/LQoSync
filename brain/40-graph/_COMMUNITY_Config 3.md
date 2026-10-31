---
type: community
members: 3
---

# Config 3

**Members:** 3 nodes

## Members
- [[config_doc.rs]] - code - tests/config_doc.rs
- [[example_in_configuration_md_parses()]] - code - tests/config_doc.rs
- [[example_values_match_the_defaults_file()]] - code - tests/config_doc.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Config_3
SORT file.name ASC
```

## Connections to other communities
- 1 edge to [[_COMMUNITY_Config]]

## Top bridge nodes
- [[config_doc.rs]] - degree 3, connects to 1 community