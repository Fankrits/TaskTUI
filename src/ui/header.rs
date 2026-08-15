use crate::app::{App, Tab};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Brand: ⚡ TASK TUI
            Constraint::Min(36),    // Navigation Tabs (4 equal items)
            Constraint::Length(20), // Interactive Play / Pause Button
            Constraint::Length(24), // Host & Uptime
        ])
        .split(area);

    let theme = &app.theme;

    // 1. Brand Block: TASK TUI
    let brand_text = vec![Line::from(vec![
        Span::styled(" ⚡ ", Style::default().fg(theme.warning)),
        Span::styled(
            "TASK TUI",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " v0.1",
            Style::default()
                .fg(theme.fg_dim)
                .add_modifier(Modifier::DIM),
        ),
    ])];
    let brand_widget = Paragraph::new(brand_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(brand_widget, chunks[0]);

    // 2. Navigation Tabs (4 equal clickable segments)
    let tab_container = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    let nav_inner = tab_container.inner(chunks[1]);
    f.render_widget(tab_container, chunks[1]);

    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(nav_inner);

    let tabs = [
        (Tab::Processes, "1: Tasks", "⚙"),
        (Tab::NetworkPorts, "2: Ports", "🌐"),
        (Tab::SystemDetails, "3: Specs", "📊"),
        (Tab::Help, "4: Help", "❓"),
    ];

    for (idx, (tab_item, name, icon)) in tabs.into_iter().enumerate() {
        let is_active = app.active_tab == tab_item;
        let (style, prefix) = if is_active {
            (
                Style::default()
                    .bg(theme.primary)
                    .fg(Color::Rgb(15, 23, 42))
                    .add_modifier(Modifier::BOLD),
                "▶ ",
            )
        } else {
            (Style::default().fg(theme.fg_dim), "  ")
        };

        let label = format!("{}{} {}", prefix, icon, name);
        let tab_p = Paragraph::new(Line::from(vec![Span::styled(label, style)]))
            .alignment(Alignment::Center);
        f.render_widget(tab_p, tab_chunks[idx]);
    }

    // 3. Play / Pause Button in Header (Fixed width, clean single line, no wrapping)
    let (btn_badge, btn_style, border_color) = if app.paused {
        (
            " ⏸ PAUSED ",
            Style::default()
                .bg(theme.warning)
                .fg(Color::Rgb(15, 23, 42))
                .add_modifier(Modifier::BOLD),
            theme.warning,
        )
    } else {
        (
            " ● LIVE ",
            Style::default()
                .bg(theme.success)
                .fg(Color::Rgb(15, 23, 42))
                .add_modifier(Modifier::BOLD),
            theme.success,
        )
    };

    let play_pause_widget = Paragraph::new(Line::from(vec![
        Span::styled(btn_badge, btn_style),
        Span::styled(" (Space)", Style::default().fg(theme.fg_dim)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(play_pause_widget, chunks[2]);

    // 4. Host & Uptime Info
    let uptime = sysinfo::System::uptime();
    let hours = uptime / 3600;
    let mins = (uptime % 3600) / 60;

    let host_text = vec![Line::from(vec![
        Span::styled(
            &app.monitor.host_name,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{}h{}m", hours, mins),
            Style::default().fg(theme.primary),
        ),
    ])];

    let host_widget = Paragraph::new(host_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        );
    f.render_widget(host_widget, chunks[3]);
}
