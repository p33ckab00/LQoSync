//! Shared wide-terminal shell. Screen bodies migrate independently.

use super::{
    app::{App, Tab},
    theme::Role,
};
use crate::config::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn enabled(area: Rect) -> bool {
    area.width >= 140 && area.height >= 28
}

pub fn sidebar_width(width: u16) -> u16 {
    ((u32::from(width) * 14 / 100) as u16).clamp(22, 32)
}

fn local_clock(now: u64) -> String {
    let Some(at) = (now > 0)
        .then(|| libc::time_t::try_from(now).ok())
        .flatten()
    else {
        return "time unknown".into();
    };
    let mut time = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: both pointers are valid; localtime_r initializes time only on success.
    if unsafe { libc::localtime_r(&at, time.as_mut_ptr()) }.is_null() {
        return "time unknown".into();
    }
    // SAFETY: localtime_r returned a non-null pointer to the initialized tm.
    let time = unsafe { time.assume_init() };
    let offset = time.tm_gmtoff.unsigned_abs();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} {}{:02}:{:02}",
        i64::from(time.tm_year) + 1900,
        time.tm_mon + 1,
        time.tm_mday,
        time.tm_hour,
        time.tm_min,
        if time.tm_gmtoff < 0 { '-' } else { '+' },
        offset / 3600,
        offset % 3600 / 60
    )
}

pub struct Shell {
    pub content: Rect,
    pub navigation: Vec<Rect>,
    pub theme: Rect,
}

pub fn draw(f: &mut Frame, area: Rect, app: &App) -> Shell {
    let (tab, data, now, read_only, palette) =
        (app.tab, &app.data, app.now, app.read_only, &app.palette);
    let mut targets = Vec::with_capacity(Tab::ALL.len());
    let width = sidebar_width(area.width);
    let sidebar = Rect::new(area.x, area.y, width, area.height);
    let (bg, fg, dim, border, selected) = match palette.theme {
        Theme::Dark => (
            Color::Rgb(20, 21, 23),
            Color::Rgb(241, 243, 245),
            Color::Rgb(178, 184, 191),
            Color::Rgb(43, 46, 50),
            Color::Rgb(44, 54, 64),
        ),
        Theme::Light => (
            Color::Rgb(233, 239, 243),
            Color::Rgb(36, 55, 68),
            Color::Rgb(96, 117, 131),
            Color::Rgb(160, 175, 185),
            Color::Rgb(212, 229, 236),
        ),
        Theme::Mono => (
            Color::Reset,
            Color::Reset,
            Color::Reset,
            Color::Reset,
            Color::Reset,
        ),
    };
    let base = Style::new().bg(bg).fg(fg);
    f.render_widget(
        Block::default()
            .style(base)
            .borders(Borders::RIGHT)
            .border_style(Style::new().fg(border)),
        sidebar,
    );
    let line = |f: &mut Frame, row: u16, text: &str, style: Style| {
        f.render_widget(
            Paragraph::new(text.to_owned()).style(style),
            Rect::new(sidebar.x + 1, sidebar.y + row, width - 2, 1),
        );
    };
    line(f, 1, "LQoSync3", base);
    line(f, 2, "TERMINAL CONSOLE", base.fg(dim));
    for (row, text) in [
        (6, "OPERATIONS"),
        (12, "MANAGEMENT"),
        (16, "ROADMAP"),
        (17, "Profiles   planned"),
        (18, "WAN plans  planned"),
    ] {
        line(f, row, text, base.fg(dim));
    }
    for (i, (route, row)) in Tab::ALL
        .into_iter()
        .zip([4, 7, 8, 9, 10, 13, 14])
        .enumerate()
    {
        let rect = Rect::new(sidebar.x + 1, sidebar.y + row, width - 2, 1);
        targets.push(rect);
        let active = tab == route || (tab == Tab::Network && route == Tab::Devices);
        let style = if active {
            if palette.theme == Theme::Mono {
                palette.selection()
            } else {
                base.bg(selected)
            }
        } else {
            base
        };
        f.render_widget(
            Paragraph::new(format!("{} {}", i + 1, route.title())).style(style),
            rect,
        );
        if active {
            f.render_widget(
                Paragraph::new("▌").style(palette.role(Role::Ok).bg(bg)),
                Rect::new(sidebar.x, sidebar.y + row, 1, 1),
            );
        }
    }
    let st = data.status.as_deref();
    let field = |key: &str| st.and_then(|s| s[key].as_str()).unwrap_or("unknown");
    let service = data
        .service
        .as_ref()
        .map_or("unknown", |s| s.active.as_str());
    for (row, label, value) in [
        (area.height - 6, "SERVICE", service),
        (area.height - 5, "MODE", field("mode")),
        (area.height - 4, "TARGET", field("target")),
    ] {
        line(f, row, &format!("{label:<8}{value}"), base);
    }
    if read_only {
        line(
            f,
            area.height - 2,
            "READ-ONLY",
            palette.role(Role::Warning).bg(bg),
        );
    }
    let content = Rect::new(area.x + width, area.y, area.width - width, area.height);
    let age = st
        .and_then(|s| s["updated"].as_u64())
        .filter(|&at| at <= now)
        .map_or("unknown".to_owned(), |at| format!("{}s", now - at));
    let theme = match palette.theme {
        Theme::Dark => "Dark",
        Theme::Light => "Light",
        Theme::Mono => "Mono",
    };
    let theme_label = format!("[T {theme}]");
    let theme_target = Rect::new(
        content.right() - theme_label.len() as u16 - 1,
        area.y,
        theme_label.len() as u16,
        1,
    );
    let detail = format!("SYNC AGE {age} | {} | ", local_clock(now));
    let title = format!(" {}", tab.title());
    let padding =
        (content.width as usize).saturating_sub(title.len() + detail.len() + theme_label.len() + 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(title),
            Span::raw(" ".repeat(padding)),
            Span::styled(detail, palette.role(Role::Secondary)),
            Span::styled(theme_label, palette.role(Role::Accent)),
        ])),
        Rect::new(content.x, content.y, content.width, 1),
    );
    Shell {
        content: Rect::new(content.x, content.y + 1, content.width, content.height - 1),
        navigation: targets,
        theme: theme_target,
    }
}
