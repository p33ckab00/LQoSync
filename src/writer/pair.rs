//! `ShapedDevices.csv` and `network.json` text.

use crate::model::{Columns, CsvRow};
use serde::Serialize;

fn field<'a>(row: &'a CsvRow, column: &str) -> &'a str {
    match column {
        "Circuit ID" => &row.circuit_id,
        "Circuit Name" => &row.circuit_name,
        "Device ID" => &row.device_id,
        "Device Name" => &row.device_name,
        "Parent Node" => &row.parent_node,
        "Parent Node ID" => &row.parent_node_id,
        "Anchor Node ID" => &row.anchor_node_id,
        "MAC" => &row.mac,
        "IPv4" => &row.ipv4,
        "IPv6" => &row.ipv6,
        "Download Min Mbps" => &row.download_min,
        "Upload Min Mbps" => &row.upload_min,
        "Download Max Mbps" => &row.download_max,
        "Upload Max Mbps" => &row.upload_max,
        "Comment" => &row.comment,
        _ => "",
    }
}

/// `ShapedDevices.csv` text: header, then rows sorted by parent, name, device, IPv4, MAC.
pub fn render_csv(rows: &[CsvRow], cols: Columns) -> String {
    let mut sorted: Vec<&CsvRow> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        (
            &a.parent_node,
            &a.circuit_name,
            &a.device_name,
            &a.ipv4,
            &a.mac,
        )
            .cmp(&(
                &b.parent_node,
                &b.circuit_name,
                &b.device_name,
                &b.ipv4,
                &b.mac,
            ))
    });
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(Vec::new());
    let names = cols.names();
    // Writes to a Vec do not fail.
    w.write_record(names).expect("write to memory");
    for row in sorted {
        w.write_record(names.iter().map(|c| field(row, c)))
            .expect("write to memory");
    }
    String::from_utf8(w.into_inner().expect("flush to memory"))
        .expect("UTF-8 input gives UTF-8 output")
}

/// `network.json` text: four-space indent, sorted keys, raw UTF-8, and a final newline.
pub fn render_network_json(v: &serde_json::Value) -> String {
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut buf,
        serde_json::ser::PrettyFormatter::with_indent(b"    "),
    );
    v.serialize(&mut ser).expect("a JSON value serializes");
    let mut s = String::from_utf8(buf).expect("serde_json writes UTF-8");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Columns, CsvRow};

    fn row(name: &str, parent: &str) -> CsvRow {
        CsvRow {
            circuit_name: name.into(),
            parent_node: parent.into(),
            ..Default::default()
        }
    }

    #[test]
    fn csv_header_sort_and_quoting() {
        let out = render_csv(
            &[row("b", "P2"), row("a,b", "P1"), row("Niño", "P1")],
            Columns::V2,
        );
        assert!(out.starts_with("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment\n"));
        let names: Vec<&str> = out
            .lines()
            .skip(1)
            .map(|l| l.split(',').nth(1).unwrap())
            .collect();
        // P1 before P2, then byte order ('N' < 'a'). "a,b" is quoted.
        assert_eq!(names, ["Niño", "\"a", "b"]);
        assert!(out.contains(",\"a,b\","));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn csv_quotes_like_python() {
        let r = CsvRow {
            circuit_id: "a,b".into(),
            circuit_name: "say \"hi\"".into(),
            device_id: "Niño".into(),
            ..Default::default()
        };
        assert!(
            render_csv(&[r], Columns::V2)
                .lines()
                .nth(1)
                .unwrap()
                .starts_with("\"a,b\",\"say \"\"hi\"\"\",Niño,,")
        );
    }

    #[test]
    fn csv_headers_for_v2_and_v3() {
        assert!(render_csv(&[], Columns::V2).starts_with(
            "Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,MAC,"
        ));
        assert!(render_csv(&[], Columns::V3).starts_with("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,Anchor Node ID,MAC,"));
        assert_eq!(render_csv(&[], Columns::V3).lines().count(), 1);
    }

    #[test]
    fn network_json_has_python_layout() {
        let v = serde_json::json!({"Router-1":{"children":{"PPP-Router-1":{"children":{},"downloadBandwidthMbps":0.128,"type":"plan","uploadBandwidthMbps":0.128}},"downloadBandwidthMbps":620.0,"type":"site","uploadBandwidthMbps":620.0},"Niño-HS":{"children":{},"downloadBandwidthMbps":157.5,"type":"client","uploadBandwidthMbps":157.5}});
        // Four-space indent, sorted keys, raw UTF-8, final newline.
        assert_eq!(
            render_network_json(&v),
            "{\n    \"Niño-HS\": {\n        \"children\": {},\n        \"downloadBandwidthMbps\": 157.5,\n        \"type\": \"client\",\n        \"uploadBandwidthMbps\": 157.5\n    },\n    \"Router-1\": {\n        \"children\": {\n            \"PPP-Router-1\": {\n                \"children\": {},\n                \"downloadBandwidthMbps\": 0.128,\n                \"type\": \"plan\",\n                \"uploadBandwidthMbps\": 0.128\n            }\n        },\n        \"downloadBandwidthMbps\": 620.0,\n        \"type\": \"site\",\n        \"uploadBandwidthMbps\": 620.0\n    }\n}\n"
        );
        assert_eq!(render_network_json(&serde_json::json!({})), "{}\n");
    }
}
