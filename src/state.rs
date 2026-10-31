//! CSV read, static rows, seed, and the state file.

use crate::model::{CsvRow, Source, State, StoredCircuit};
use anyhow::{Context, ensure};
use std::collections::HashMap;

/// Read `ShapedDevices.csv` text with 13, 14, or 15 columns, by header name.
/// A row with an empty Circuit Name is skipped. A later row with the same name
/// replaces the earlier row, at the position of the earlier row.
pub fn read_csv(text: &str) -> anyhow::Result<Vec<CsvRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows: Vec<CsvRow> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (n, rec) in reader.deserialize::<CsvRow>().enumerate() {
        let row = rec.with_context(|| format!("ShapedDevices.csv: bad data row {}", n + 1))?;
        if row.circuit_name.trim().is_empty() {
            continue;
        }
        match index.get(&row.circuit_name) {
            Some(&i) => rows[i] = row,
            None => {
                index.insert(row.circuit_name.clone(), rows.len());
                rows.push(row);
            }
        }
    }
    Ok(rows)
}

/// Split the rows into `(static, other)`. A static row has the static comment (case is ignored).
pub fn split_static(rows: Vec<CsvRow>, static_comment: &str) -> (Vec<CsvRow>, Vec<CsvRow>) {
    rows.into_iter()
        .partition(|r| r.comment.trim().eq_ignore_ascii_case(static_comment.trim()))
}

/// Make a first state from the rows of the current CSV. The caller removes the static rows.
pub fn seed_state(rows: &[CsvRow], now: u64) -> State {
    let mut st = State::default();
    for row in rows {
        let source = if row.comment.trim().eq_ignore_ascii_case("PPP") {
            Source::Ppp
        } else if row.circuit_name.starts_with("HS-") {
            Source::Hotspot
        } else if row.circuit_name.starts_with("DHCP-") {
            Source::Dhcp
        } else {
            Source::Unknown
        };
        st.circuits.insert(
            row.circuit_name.clone(),
            StoredCircuit {
                router: None,
                source,
                last_seen: now,
                row: row.clone(),
            },
        );
    }
    st
}

impl State {
    pub fn from_json(s: &str) -> anyhow::Result<State> {
        let st: State = serde_json::from_str(s).context("state.json: bad JSON")?;
        ensure!(
            st.version == 1,
            "state.json: version {} is not supported (this tool reads 1)",
            st.version
        );
        Ok(st)
    }

    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("state serializes");
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CsvRow, Source, State};

    const HDR: &str = "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n";

    #[test]
    fn read_csv_accepts_bom_crlf_and_missing_columns() {
        let t = format!("\u{feff}{}", HDR.replace('\n', "\r\n"))
            + "A1,alice,D1,alice,P,,10.0.0.2,,5.0,5.0,50.0,50.0,PPP\r\n";
        let rows = read_csv(&t).unwrap();
        assert_eq!(rows[0].circuit_id, "A1");
        assert_eq!(rows[0].comment, "PPP");
        let rows = read_csv("Circuit Name,IPv4\nbob,10.0.0.3\n").unwrap();
        assert_eq!(
            (rows[0].circuit_name.as_str(), rows[0].comment.as_str()),
            ("bob", "")
        );
        let v3 = "Circuit ID,Circuit Name,Parent Node,Parent Node ID,Anchor Node ID\nC1,carl,N,lq3-ab,\n";
        assert_eq!(read_csv(v3).unwrap()[0].parent_node_id, "lq3-ab");
    }

    #[test]
    fn read_csv_skips_blank_names_and_later_duplicate_wins() {
        let rows = read_csv(&format!(
            "{HDR}A,x,,,,,,,,,,,1\nB,,,,,,,,,,,,2\nC,x,,,,,,,,,,,3\n"
        ))
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].circuit_id, "C");
    }

    #[test]
    fn split_static_ignores_case() {
        let mk = |n: &str, c: &str| CsvRow {
            circuit_name: n.into(),
            comment: c.into(),
            ..Default::default()
        };
        let (s, o) = split_static(vec![mk("a", "Static"), mk("b", "PPP")], "static");
        assert_eq!((s.len(), o.len()), (1, 1));
    }

    #[test]
    fn seed_infers_source_and_keeps_ids() {
        let mk = |n: &str, c: &str| CsvRow {
            circuit_id: format!("ID-{n}"),
            circuit_name: n.into(),
            comment: c.into(),
            ..Default::default()
        };
        let st = seed_state(
            &[
                mk("alice", "PPP"),
                mk("HS-AABB", "HS"),
                mk("DHCP-pc", "pc"),
                mk("odd", "x"),
            ],
            7,
        );
        assert_eq!(st.circuits["alice"].source, Source::Ppp);
        assert_eq!(st.circuits["HS-AABB"].source, Source::Hotspot);
        assert_eq!(st.circuits["DHCP-pc"].source, Source::Dhcp);
        assert_eq!(st.circuits["odd"].source, Source::Unknown);
        assert_eq!(st.circuits["alice"].row.circuit_id, "ID-alice");
        assert_eq!(
            (
                st.circuits["alice"].router.clone(),
                st.circuits["alice"].last_seen
            ),
            (None, 7)
        );
    }

    #[test]
    fn state_json_round_trip_and_version_check() {
        let st = seed_state(&[], 1);
        assert_eq!(State::from_json(&st.to_json()).unwrap().version, 1);
        assert!(
            State::from_json(r#"{"version":2,"circuits":{},"nodes":{},"routers":{}}"#).is_err()
        );
    }
}
