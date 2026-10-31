---
source_file: "src/state.rs"
type: "code"
community: "File Safety"
location: "L11"
tags:
  - graphify/code
  - graphify/EXTRACTED
  - community/File_Safety
---

# read_csv()

## Connections
- [[CsvRow]] - `references` [EXTRACTED]
- [[diff.rs]] - `imports_from` [EXTRACTED]
- [[diff_pair()]] - `calls` [INFERRED]
- [[files.rs]] - `imports_from` [EXTRACTED]
- [[read_csv_accepts_bom_crlf_and_missing_columns()]] - `calls` [EXTRACTED]
- [[read_csv_skips_blank_names_and_keeps_repeated_names()]] - `calls` [EXTRACTED]
- [[read_live()]] - `calls` [INFERRED]
- [[scenarios.rs]] - `imports_from` [EXTRACTED]
- [[scenarios_match_expected_output()]] - `calls` [INFERRED]
- [[state.rs]] - `contains` [EXTRACTED]

#graphify/code #graphify/EXTRACTED #community/File_Safety