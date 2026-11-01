//! The ShapedDevices tab: a table of the rows with search, sort, flags, and a detail panel.

use super::app::split_main;
use super::data::{Data, FLAG_DUP_IP, FLAG_NO_PARENT, FLAG_RATE};
use crate::model::CsvRow;
use crate::tui::theme::{BoxKind, Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};

use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState, Wrap};
use std::collections::HashSet;
use std::sync::Arc;

/// The sort columns, in the order of the `o` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Column {
    #[default]
    Name,
    Ipv4,
    Down,
    Up,
    Parent,
    Source,
}

impl Column {
    const ALL: [Column; 6] = [
        Column::Name,
        Column::Ipv4,
        Column::Down,
        Column::Up,
        Column::Parent,
        Column::Source,
    ];
    fn title(self) -> &'static str {
        match self {
            Column::Name => "name",
            Column::Ipv4 => "IPv4",
            Column::Down => "download max",
            Column::Up => "upload max",
            Column::Parent => "parent",
            Column::Source => "source",
        }
    }
}

/// The words of the flag bits.
pub fn flag_words(f: u8) -> Vec<&'static str> {
    [
        (FLAG_DUP_IP, "dup IP"),
        (FLAG_NO_PARENT, "no parent"),
        (FLAG_RATE, "bad rate"),
    ]
    .into_iter()
    .filter(|(b, _)| f & b != 0)
    .map(|(_, w)| w)
    .collect()
}

/// The source and router of a circuit, for the screen. A circuit not in `state.json` is static.
fn source_of<'a>(d: &'a Data, name: &str) -> (&'static str, &'a str) {
    match d.sources.get(name) {
        Some((s, r)) => (
            match s.as_str() {
                "ppp" => "PPPoE",
                "hotspot" => "Hotspot",
                "dhcp" => "DHCP",
                _ => "unknown",
            },
            r.as_str(),
        ),
        None => ("static", "-"),
    }
}

#[derive(Debug, Default)]
pub struct DevicesView {
    pub total_circuits: usize,
    pub filtered_circuits: usize,
    pub query: String,
    /// The search box has the keys.
    pub editing: bool,
    /// The column and `true` for a reverse order.
    pub sort: (Column, bool),
    /// Row indexes after the filter and the sort.
    pub index: Vec<u32>,
    /// Circuit ID and Device ID of the selected row, so the selection stays after a refresh.
    pub selected: Option<(String, String)>,
    pub cursor: usize,
    offset: usize,
    rows: Option<Arc<Vec<CsvRow>>>,
    /// Table rows made by the last draw (for the test of the visible window).
    pub(crate) rows_built: usize,
}

impl DevicesView {
    /// New data: rebuild only if the rows changed.
    pub fn on_data(&mut self, d: &Data) {
        if self.rows.as_ref().is_none_or(|r| !Arc::ptr_eq(r, &d.rows)) {
            self.total_circuits = d
                .rows
                .iter()
                .map(|r| &r.circuit_id)
                .collect::<HashSet<_>>()
                .len();
            self.rows = Some(d.rows.clone());
            self.rebuild(d);
        }
    }

    /// Filter and sort. Called only when the data, the query, or the sort changes.
    pub fn rebuild(&mut self, d: &Data) {
        let q = self.query.to_lowercase();
        self.index = (0..d.rows.len() as u32)
            .filter(|&i| {
                q.is_empty() || d.row_search.get(i as usize).is_some_and(|s| s.contains(&q))
            })
            .collect();
        self.filtered_circuits = self
            .index
            .iter()
            .map(|&i| &d.rows[i as usize].circuit_id)
            .collect::<HashSet<_>>()
            .len();
        let rows = &d.rows;
        let (col, rev) = self.sort;
        let num = |s: &str| s.trim().parse::<f64>().unwrap_or(f64::NEG_INFINITY);
        match col {
            Column::Down | Column::Up => {
                // The keys are made one time, not in each comparison.
                let mut keyed: Vec<(f64, u32)> = self
                    .index
                    .iter()
                    .map(|&i| {
                        let r = &rows[i as usize];
                        (
                            num(if col == Column::Down {
                                &r.download_max
                            } else {
                                &r.upload_max
                            }),
                            i,
                        )
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                self.index = keyed.into_iter().map(|(_, i)| i).collect();
            }
            Column::Source => {
                self.index
                    .sort_by_cached_key(|&i| source_of(d, &rows[i as usize].circuit_name).0);
            }
            _ => {
                let field = |r: &CsvRow| -> String {
                    match col {
                        Column::Ipv4 => r.ipv4.clone(),
                        Column::Parent => r.parent_node.to_lowercase(),
                        _ => r.circuit_name.to_lowercase(),
                    }
                };
                self.index.sort_by_cached_key(|&i| field(&rows[i as usize]));
            }
        }
        if rev {
            self.index.reverse();
        }
        self.cursor = self
            .selected
            .as_ref()
            .and_then(|(c, dv)| {
                self.index.iter().position(|&i| {
                    let r = &rows[i as usize];
                    &r.circuit_id == c && &r.device_id == dv
                })
            })
            .unwrap_or(self.cursor.min(self.index.len().saturating_sub(1)));
        self.select(d);
    }

    fn select(&mut self, d: &Data) {
        self.selected = self
            .index
            .get(self.cursor)
            .map(|&i| &d.rows[i as usize])
            .map(|r| (r.circuit_id.clone(), r.device_id.clone()));
    }

    /// A key. Returns the parent node name for `Enter`.
    pub fn on_key(&mut self, k: KeyEvent, d: &Data) -> Option<String> {
        if self.editing {
            match k.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.query.clear();
                }
                KeyCode::Enter => self.editing = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                _ => return None,
            }
            self.rebuild(d);
            return None;
        }
        let last = self.index.len().saturating_sub(1);
        match k.code {
            KeyCode::Char('/') => self.editing = true,
            KeyCode::Down | KeyCode::Char('j') => self.cursor = (self.cursor + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::PageDown => self.cursor = (self.cursor + 20).min(last),
            KeyCode::PageUp => self.cursor = self.cursor.saturating_sub(20),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = last,
            KeyCode::Char('o') => {
                let i = Column::ALL
                    .iter()
                    .position(|c| *c == self.sort.0)
                    .unwrap_or(0);
                self.sort.0 = Column::ALL[(i + 1) % Column::ALL.len()];
                self.rebuild(d);
            }
            KeyCode::Char('i') => {
                self.sort.1 = !self.sort.1;
                self.rebuild(d);
            }
            KeyCode::Enter => {
                return self
                    .index
                    .get(self.cursor)
                    .map(|&i| d.rows[i as usize].parent_node.clone())
                    .filter(|p| !p.is_empty());
            }
            _ => return None,
        }
        self.select(d);
        None
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, d: &Data, palette: &Palette) {
        let (main, side) = if area.width < 100 {
            let [main, side] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Min(0)]).areas(area);
            (main, side)
        } else {
            split_main(area)
        };
        // Borders and the header row.
        let height = main.height.saturating_sub(3) as usize;
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if height > 0 && self.cursor >= self.offset + height {
            self.offset = self.cursor + 1 - height;
        }
        let end = (self.offset + height).min(self.index.len());
        self.rows_built = 0;
        let mut rows = Vec::with_capacity(end.saturating_sub(self.offset));
        let compact = main.width < 145;
        let columns: &[usize] = if compact {
            &[0, 1, 3, 4, 8]
        } else {
            &[0, 1, 2, 3, 4, 5, 6, 7, 8]
        };
        for (n, &i) in self.index[self.offset.min(end)..end].iter().enumerate() {
            let pos = self.offset + n;
            let selected = pos == self.cursor;
            let r = &d.rows[i as usize];
            let fl = d.flags.get(i as usize).copied().unwrap_or(0);
            let (src, router) = source_of(d, &r.circuit_name);
            let mut badges = Vec::new();
            for (bit, label, role) in [
                (FLAG_DUP_IP, "dup IP", Role::Error),
                (FLAG_NO_PARENT, "no parent", Role::Warning),
                (FLAG_RATE, "bad rate", Role::Error),
            ] {
                if fl & bit != 0 {
                    if !badges.is_empty() {
                        badges.push(Span::raw(" "));
                    }
                    badges.push(palette.badge(label, role, selected));
                }
            }
            let cells = [
                Cell::from(r.circuit_name.as_str()),
                Cell::from(r.ipv4.as_str()),
                Cell::from(r.mac.as_str()),
                Cell::from(
                    Line::from(format!("{}/{}", r.download_min, r.download_max)).right_aligned(),
                ),
                Cell::from(
                    Line::from(format!("{}/{}", r.upload_min, r.upload_max)).right_aligned(),
                ),
                Cell::from(r.parent_node.as_str()),
                Cell::from(src),
                Cell::from(router),
                Cell::from(Line::from(badges)),
            ];
            rows.push(
                Row::new(
                    columns
                        .iter()
                        .map(|&j| cells[j].clone())
                        .collect::<Vec<_>>(),
                )
                .style(palette.row(pos, selected)),
            );
            self.rows_built += 1;
        }
        let labels = [
            "Circuit Name",
            "IPv4",
            "MAC",
            "Down min/max",
            "Up min/max",
            "Parent Node",
            "Source",
            "Router",
            "Flags",
        ];
        let header = Row::new(columns.iter().map(|&j| labels[j])).style(palette.role(Role::Accent));
        let all_widths = [
            Constraint::Fill(3),
            Constraint::Length(15),
            Constraint::Length(17),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Fill(2),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(26),
        ];
        let widths = columns.iter().map(|&j| all_widths[j]).collect::<Vec<_>>();
        let counts = format!(
            "rows {}/{} | circuits {}/{}",
            self.index.len(),
            d.rows.len(),
            self.filtered_circuits,
            self.total_circuits
        );
        let title = if main.width < 100 {
            format!(" ShapedDevices circuits {} ", self.total_circuits)
        } else {
            format!(" ShapedDevices {counts} ")
        };
        let summary = format!(
            " sort {}{} | /{}{} ",
            self.sort.0.title(),
            if self.sort.1 { " reverse" } else { "" },
            self.query,
            if self.editing { "_" } else { "" }
        );
        let mut state =
            TableState::default().with_selected(Some(self.cursor - self.offset.min(self.cursor)));
        f.render_stateful_widget(
            Table::new(rows, widths)
                .header(header)
                .block(
                    palette
                        .panel(BoxKind::Cpu, &title, true)
                        .title_bottom(summary),
                )
                .row_highlight_style(palette.selection()),
            main,
            &mut state,
        );
        f.render_widget(
            Paragraph::new(if side.height < 20 {
                self.compact_detail(d, side.width.saturating_sub(2) as usize)
            } else {
                self.detail(d)
            })
            .wrap(Wrap { trim: false })
            .block(
                palette
                    .panel(BoxKind::Mem, "circuit", false)
                    .title_bottom(counts),
            ),
            side,
        );
    }

    fn compact_detail(&self, d: &Data, width: usize) -> Vec<Line<'static>> {
        let Some(&i) = self.index.get(self.cursor) else {
            return vec![Line::from("no row")];
        };
        let r = &d.rows[i as usize];
        let (source, router) = source_of(d, &r.circuit_name);
        let flags = flag_words(d.flags.get(i as usize).copied().unwrap_or(0));
        let part = width.saturating_sub(20) / 2;
        [
            format!(
                "Circuit: {} | ID: {}",
                super::theme::clip(&r.circuit_name, part),
                super::theme::clip(&r.circuit_id, part)
            ),
            format!(
                "Device: {} | ID: {}",
                super::theme::clip(&r.device_name, part),
                super::theme::clip(&r.device_id, part)
            ),
            format!("MAC: {} | IPv4: {}", r.mac, r.ipv4),
            format!(
                "Parent Node: {} | ID: {}",
                super::theme::clip(&r.parent_node, part),
                super::theme::clip(&r.parent_node_id, part)
            ),
            format!("Source: {source} | Router: {router}"),
            format!(
                "Flags: {}",
                if flags.is_empty() {
                    "none".into()
                } else {
                    flags.join(", ")
                }
            ),
            format!(
                "Down: {}/{} | Up: {}/{} Mbps",
                r.download_min, r.download_max, r.upload_min, r.upload_max
            ),
            format!("Comment: {}", r.comment),
        ]
        .into_iter()
        .map(|line| Line::from(super::theme::clip(&line, width)))
        .collect()
    }

    fn detail(&self, d: &Data) -> Vec<Line<'static>> {
        let Some(&i) = self.index.get(self.cursor) else {
            return vec![Line::from("no row")];
        };
        let r = &d.rows[i as usize];
        let (src, router) = source_of(d, &r.circuit_name);
        let fl = d.flags.get(i as usize).copied().unwrap_or(0);
        let flags = flag_words(fl);
        [
            ("Circuit Name", r.circuit_name.clone()),
            ("Circuit ID", r.circuit_id.clone()),
            ("Device Name", r.device_name.clone()),
            ("Device ID", r.device_id.clone()),
            ("MAC", r.mac.clone()),
            ("IPv4", r.ipv4.clone()),
            ("IPv6", r.ipv6.clone()),
            (
                "Download min/max",
                format!("{} / {} Mbps", r.download_min, r.download_max),
            ),
            (
                "Upload min/max",
                format!("{} / {} Mbps", r.upload_min, r.upload_max),
            ),
            ("Parent Node", r.parent_node.clone()),
            ("Parent Node ID", r.parent_node_id.clone()),
            ("Comment", r.comment.clone()),
            ("Source", src.to_string()),
            ("Router", router.to_string()),
            (
                "Flags",
                if flags.is_empty() {
                    "none".into()
                } else {
                    flags.join(", ")
                },
            ),
        ]
        .into_iter()
        .map(|(k, v)| Line::from(format!("{k}: {v}")))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::data::{FLAG_DUP_IP, FLAG_RATE};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashMap;

    fn row(cid: &str, name: &str, ip: &str, mac: &str, dmax: &str, parent: &str) -> CsvRow {
        CsvRow {
            circuit_id: cid.into(),
            circuit_name: name.into(),
            device_id: format!("d-{cid}"),
            device_name: name.into(),
            parent_node: parent.into(),
            mac: mac.into(),
            ipv4: ip.into(),
            download_min: "1".into(),
            upload_min: "1".into(),
            download_max: dmax.into(),
            upload_max: "10".into(),
            ..CsvRow::default()
        }
    }

    pub(crate) fn data(rows: Vec<CsvRow>) -> Data {
        let search = rows
            .iter()
            .map(|r| {
                format!("{} {} {} {}", r.circuit_name, r.circuit_id, r.mac, r.ipv4).to_lowercase()
            })
            .collect();
        let flags = vec![0; rows.len()];
        Data {
            rows: Arc::new(rows),
            row_search: Arc::new(search),
            flags: Arc::new(flags),
            ..Data::default()
        }
    }

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn names(v: &DevicesView, d: &Data) -> Vec<String> {
        v.index
            .iter()
            .map(|&i| d.rows[i as usize].circuit_name.clone())
            .collect()
    }

    fn sample() -> Data {
        data(vec![
            row("c1", "Alice", "10.0.0.2", "AA:BB:CC:00:00:01", "10", "AP"),
            row("c2", "bob", "10.0.0.3", "aa:bb:cc:00:00:02", "9", "AP"),
            row("c3", "Carol", "10.0.1.4", "", "100", "Site"),
        ])
    }

    #[test]
    fn search_matches_name_ip_and_mac_case_insensitive() {
        let d = sample();
        let mut v = DevicesView::default();
        v.on_data(&d);
        for (q, want) in [
            ("ALICE", vec!["Alice"]),
            ("10.0.1", vec!["Carol"]),
            ("cc:00:00:02", vec!["bob"]),
        ] {
            v.on_key(key(KeyCode::Char('/')), &d);
            for c in q.chars() {
                v.on_key(key(KeyCode::Char(c)), &d);
            }
            v.on_key(key(KeyCode::Enter), &d);
            assert_eq!(names(&v, &d), want, "{q}");
            v.on_key(key(KeyCode::Char('/')), &d);
            v.on_key(key(KeyCode::Esc), &d);
            assert_eq!(v.index.len(), 3);
        }
    }

    #[test]
    fn sort_by_download_max_is_numeric() {
        let d = sample();
        let mut v = DevicesView {
            sort: (Column::Down, false),
            ..Default::default()
        };
        v.on_data(&d);
        assert_eq!(names(&v, &d), ["bob", "Alice", "Carol"]);
        v.on_key(key(KeyCode::Char('i')), &d);
        assert_eq!(names(&v, &d), ["Carol", "Alice", "bob"]);
    }

    #[test]
    fn selection_stays_on_the_same_circuit_after_a_refresh_with_new_rows() {
        let d = sample();
        let mut v = DevicesView::default();
        v.on_data(&d);
        v.on_key(key(KeyCode::Down), &d);
        assert_eq!(v.selected.as_ref().unwrap().0, "c2");
        let mut rows = (*d.rows).clone();
        rows.insert(0, row("c0", "Aaron", "10.0.0.9", "", "5", "AP"));
        let d2 = data(rows);
        v.on_data(&d2);
        assert_eq!(d2.rows[v.index[v.cursor] as usize].circuit_id, "c2");
    }

    #[test]
    fn draw_builds_rows_only_for_the_visible_window() {
        let rows = (0..20_000)
            .map(|i| {
                row(
                    &format!("c{i}"),
                    &format!("n{i:05}"),
                    "10.0.0.1",
                    "",
                    "10",
                    "AP",
                )
            })
            .collect();
        let d = data(rows);
        let mut v = DevicesView::default();
        v.on_data(&d);
        let mut t = Terminal::new(TestBackend::new(120, 33)).unwrap();
        t.draw(|f| v.draw(f, f.area(), &d, &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        assert!(v.rows_built > 0 && v.rows_built <= 30, "{}", v.rows_built);
    }

    #[test]
    fn enter_opens_the_parent_node() {
        let d = sample();
        let mut v = DevicesView::default();
        v.on_data(&d);
        assert_eq!(v.on_key(key(KeyCode::Enter), &d), Some("AP".to_string()));
    }

    #[test]
    fn devices_tab_renders_flags_as_words() {
        let mut d = sample();
        d.flags = Arc::new(vec![FLAG_DUP_IP | FLAG_RATE, 0, 0]);
        d.sources = Arc::new(HashMap::from([(
            "Alice".to_string(),
            ("ppp".to_string(), "R1".to_string()),
        )]));
        let mut v = DevicesView::default();
        v.on_data(&d);
        let mut t = Terminal::new(TestBackend::new(260, 20)).unwrap();
        t.draw(|f| v.draw(f, f.area(), &d, &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        let buf = t.backend().buffer();
        let screen: String = (0..20)
            .map(|y| (0..260).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect();
        for want in ["dup IP", "bad rate", "PPPoE", "R1", "static"] {
            assert!(screen.contains(want), "{want}\n{screen}");
        }
    }

    #[test]
    #[ignore = "release timing: cargo test --release -- --ignored perf"]
    fn perf_20k_rows_key_press_under_16ms() {
        let rows = (0..20_000)
            .map(|i| {
                row(
                    &format!("c{i}"),
                    &format!("n{i:05}"),
                    &format!("10.{}.{}.1", i / 256, i % 256),
                    "",
                    &format!("{}", i % 97),
                    "AP",
                )
            })
            .collect();
        let d = data(rows);
        let mut v = DevicesView {
            sort: (Column::Down, false),
            ..Default::default()
        };
        v.on_data(&d);
        let mut t = Terminal::new(TestBackend::new(200, 50)).unwrap();
        v.on_key(key(KeyCode::Char('/')), &d);
        let mut worst = std::time::Duration::ZERO;
        for c in "n1".chars() {
            let start = std::time::Instant::now();
            v.on_key(key(KeyCode::Char(c)), &d);
            t.draw(|f| v.draw(f, f.area(), &d, &Palette::new(crate::config::Theme::Dark)))
                .unwrap();
            worst = worst.max(start.elapsed());
        }
        eprintln!("worst key press with 20 000 rows: {worst:?}");
        assert!(worst.as_millis() < 16, "{worst:?}");
    }
    #[test]
    fn circuits_count_distinct_ids() {
        let d = data(vec![
            row("c1", "Alice", "1", "m", "10", "AP"),
            row("c1", "Alice phone", "2", "m2", "10", "AP"),
            row("c2", "Bob", "3", "m3", "10", "AP"),
        ]);
        let mut v = DevicesView::default();
        v.on_data(&d);
        assert_eq!(v.total_circuits, 2);
        v.query = "alice".into();
        v.rebuild(&d);
        assert_eq!(v.filtered_circuits, 1);
    }
    #[test]
    fn table_styles_alignment_and_compact_detail() {
        let mut d = sample();
        d.flags = Arc::new(vec![7, 0, 0]);
        let mut v = DevicesView::default();
        v.on_data(&d);
        for theme in [
            crate::config::Theme::Dark,
            crate::config::Theme::Light,
            crate::config::Theme::Mono,
        ] {
            let p = Palette::new(theme);
            let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
            t.draw(|f| v.draw(f, f.area(), &d, &p)).unwrap();
            let b = t.backend().buffer();
            assert_eq!(b[(2, 2)].bg, p.selection().bg.unwrap());
            assert_eq!(b[(2, 3)].bg, p.row(1, false).bg.unwrap());
            let line = |y| (0..80).map(|x| b[(x, y)].symbol()).collect::<String>();
            let h = line(1);
            let r = line(2);
            let down = h.find("Down").unwrap();
            let up = h.find("Up min").unwrap();
            let flags = h.find("Flags").unwrap();
            assert!(r[down..up].ends_with("1/10 "), "{r}");
            assert!(r[up..flags].ends_with("1/10 "), "{r}");
            assert!(
                v.detail(&d)
                    .iter()
                    .any(|l| l.to_string().contains("dup IP, no parent, bad rate"))
            );
        }
    }
    #[test]
    fn compact_detail_fields_are_actually_visible() {
        let mut d = sample();
        d.flags = Arc::new(vec![7, 0, 0]);
        d.sources = Arc::new(HashMap::from([(
            "Alice".into(),
            ("ppp".into(), "FixtureRouter".into()),
        )]));
        let mut v = DevicesView::default();
        v.on_data(&d);
        let p = Palette::new(crate::config::Theme::Dark);
        let mut t = Terminal::new(TestBackend::new(80, 19)).unwrap();
        t.draw(|f| v.draw(f, f.area(), &d, &p)).unwrap();
        let b = t.backend().buffer();
        let s = (0..19)
            .map(|y| (0..80).map(|x| b[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        for text in [
            "Parent",
            "AP",
            "PPPoE",
            "FixtureRouter",
            "dup IP, no parent, bad rate",
            "MAC",
        ] {
            assert!(s.contains(text), "{text}: {s}");
        }
    }
}
