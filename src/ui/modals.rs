use crate::app::{App, Modal, ToastKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render_modals(f: &mut Frame, app: &App) {
    match &app.active_modal {
        Modal::None => {}
        Modal::ConfirmKill { pid, name, force } => {
            render_confirm_kill(f, app, *pid, name, *force);
        }
        Modal::ProcessDetails(pid) => {
            render_process_details(f, app, *pid);
        }
        Modal::Help => {
            render_help_dialog(f, app);
        }
    }

    render_toasts(f, app);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_confirm_kill(f: &mut Frame, app: &App, pid: u32, name: &str, force: bool) {
    let area = centered_rect(50, 35, f.area());
    f.render_widget(Clear, area);

    let theme = &app.theme;
    let border_color = if force { theme.danger } else { theme.warning };

    let proc_info = app.monitor.processes.iter().find(|p| p.pid == pid);
    let mem_str = proc_info
        .map(|p| format!("{:.2} MB", p.memory_bytes as f64 / (1024.0 * 1024.0)))
        .unwrap_or_else(|| "Unknown".to_string());
    let user_str = proc_info
        .map(|p| p.user.as_str())
        .unwrap_or("-");

    let action_title = if force {
        " ⚠️ FORCE KILL PROCESS "
    } else {
        " ⚠️ TERMINATE PROCESS "
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" Are you sure you want to end this process?", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  • Process Name: ", Style::default().fg(theme.fg_dim)),
            Span::styled(name, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  • Process PID:  ", Style::default().fg(theme.fg_dim)),
            Span::styled(pid.to_string(), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  • Memory Usage: ", Style::default().fg(theme.fg_dim)),
            Span::styled(mem_str, Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("  • User:         ", Style::default().fg(theme.fg_dim)),
            Span::styled(user_str, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Y / Enter] ", Style::default().bg(theme.danger).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD)),
            Span::styled(" Terminate    ", Style::default().fg(theme.fg)),
            Span::styled(" [F] ", Style::default().bg(theme.warning).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD)),
            Span::styled(" Force Kill    ", Style::default().fg(theme.fg)),
            Span::styled(" [Esc / N] ", Style::default().bg(theme.border).fg(theme.fg).add_modifier(Modifier::BOLD)),
            Span::styled(" Cancel", Style::default().fg(theme.fg_dim)),
        ]),
    ];

    let block = Block::default()
        .title(action_title)
        .title_alignment(Alignment::Center)
        .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Rgb(15, 23, 42)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_process_details(f: &mut Frame, app: &App, pid: u32) {
    let area = centered_rect(65, 60, f.area());
    f.render_widget(Clear, area);

    let theme = &app.theme;

    let proc_info = app.monitor.processes.iter().find(|p| p.pid == pid);

    let lines = if let Some(p) = proc_info {
        let uptime_min = p.run_time_secs / 60;
        let uptime_sec = p.run_time_secs % 60;
        let ports_str = if p.ports.is_empty() {
            "None".to_string()
        } else {
            p.ports
                .iter()
                .map(|port| format!(":{}", port))
                .collect::<Vec<_>>()
                .join(", ")
        };

        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" Name:          ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(&p.name, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled("   (PID: ", Style::default().fg(theme.fg_dim)),
                Span::styled(p.pid.to_string(), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                Span::styled(")", Style::default().fg(theme.fg_dim)),
            ]),
            Line::from(vec![
                Span::styled(" Status:        ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(p.status, Style::default().fg(theme.status_color(p.status))),
                Span::styled("   User: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&p.user, Style::default().fg(theme.fg)),
                Span::styled("   Session: ", Style::default().fg(theme.fg_dim)),
                Span::styled(
                    p.session_id.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" CPU Usage:     ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:.1}%", p.cpu_usage), Style::default().fg(theme.cpu_color(p.cpu_usage))),
                Span::styled("   Memory (RSS): ", Style::default().fg(theme.fg_dim)),
                Span::styled(
                    format!("{:.2} MB ({:.1}%)", p.memory_bytes as f64 / (1024.0 * 1024.0), p.memory_percent),
                    Style::default().fg(theme.memory_color(p.memory_percent)),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Virtual Mem:   ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:.2} MB", p.virtual_memory_bytes as f64 / (1024.0 * 1024.0)),
                    Style::default().fg(theme.fg_dim),
                ),
                Span::styled("   Running Time: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{}m {}s", uptime_min, uptime_sec), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(" Bound Ports:   ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(ports_str, Style::default().fg(theme.warning)),
            ]),
            Line::from(vec![
                Span::styled(" Disk Read:     ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:.2} MB", p.disk_read_bytes as f64 / (1024.0 * 1024.0)),
                    Style::default().fg(theme.success),
                ),
                Span::styled("   Disk Written: ", Style::default().fg(theme.fg_dim)),
                Span::styled(
                    format!("{:.2} MB", p.disk_written_bytes as f64 / (1024.0 * 1024.0)),
                    Style::default().fg(theme.secondary),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Executable Path:", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("   {}", if p.exe_path.is_empty() { "-" } else { &p.exe_path }), Style::default().fg(theme.fg_dim)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Command Line:", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("   {}", if p.cmd.is_empty() { "-" } else { &p.cmd }), Style::default().fg(theme.fg)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [k] ", Style::default().bg(theme.danger).fg(Color::Rgb(15, 23, 42)).add_modifier(Modifier::BOLD)),
                Span::styled(" Kill Process    ", Style::default().fg(theme.fg)),
                Span::styled(" [Esc / Enter] ", Style::default().bg(theme.border).fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled(" Close Inspector", Style::default().fg(theme.fg_dim)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("Process no longer exists.", Style::default().fg(theme.danger))),
            Line::from(""),
            Line::from(Span::styled("Press [Esc] to close.", Style::default().fg(theme.fg_dim))),
        ]
    };

    let title = format!(" 🔍 Process Details (PID {}) ", pid);
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(Color::Rgb(15, 23, 42)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_help_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(65, 70, f.area());
    f.render_widget(Clear, area);

    let theme = &app.theme;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  NAVIGATION & VIEWS", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Tab / 1-4        ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Switch active tabs (Processes / Ports / Specs / Help)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   ↑/↓, j/k         ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate rows up / down", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   PgUp / PgDn      ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Scroll page up / down", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   Home / End       ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Jump to top / bottom", Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  PROCESS ACTIONS", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   k / Delete       ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled("Terminate selected process (Confirmation prompt)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   K (Shift+k)      ", Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)),
            Span::styled("Force kill selected process", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   Enter / d        ", Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled("Inspect process full details (Path, Args, Sockets, I/O)", Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  FILTERING & SORTING", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   /                ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Open real-time search filter query", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   m / c / p / n / o", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Sort by: (m) Memory, (c) CPU, (p) PID, (n) Name, (o) Port", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   r                ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle sort direction: High → Low ⇄ Low → High", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   v                ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Span::styled("Cycle dashboard views (Combined / Top Rankings / Graphs)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   L                ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle Listening Ports only filter (Ports tab)", Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  GENERAL CONTROLS", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Space            ", Style::default().fg(theme.info).add_modifier(Modifier::BOLD)),
            Span::styled("Pause / Resume real-time background metrics refresh", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   ? / F1           ", Style::default().fg(theme.info).add_modifier(Modifier::BOLD)),
            Span::styled("Open / Close this help cheatsheet", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   q / Esc / Ctrl+C ", Style::default().fg(theme.info).add_modifier(Modifier::BOLD)),
            Span::styled("Quit application or close active dialog", Style::default().fg(theme.fg)),
        ]),
    ];

    let block = Block::default()
        .title(" ⌨ Keyboard Shortcuts & Cheatsheet ")
        .title_alignment(Alignment::Center)
        .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_focused))
        .style(Style::default().bg(Color::Rgb(15, 23, 42)));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_toasts(f: &mut Frame, app: &App) {
    if app.toasts.is_empty() {
        return;
    }

    let theme = &app.theme;
    let screen = f.area();

    // Render toasts stacked at top right
    let mut current_y = screen.y + 1;

    for toast in app.toasts.iter().rev().take(3) {
        let (icon, bg, fg) = match toast.kind {
            ToastKind::Success => (" ✔ ", theme.success, Color::Rgb(15, 23, 42)),
            ToastKind::Error => (" ✖ ", theme.danger, Color::Rgb(15, 23, 42)),
            ToastKind::Info => (" ℹ ", theme.info, Color::Rgb(15, 23, 42)),
        };

        let content = format!("{} {} ", icon, toast.message);
        let width = (content.len() as u16 + 4).min(screen.width.saturating_sub(4)).max(30);
        let height = 3;

        let toast_area = Rect {
            x: screen.x + screen.width.saturating_sub(width + 2),
            y: current_y,
            width,
            height,
        };

        f.render_widget(Clear, toast_area);

        let toast_widget = Paragraph::new(Line::from(vec![
            Span::styled(icon, Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(&toast.message, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(bg))
                .style(Style::default().bg(Color::Rgb(15, 23, 42))),
        );

        f.render_widget(toast_widget, toast_area);
        current_y += 3;
    }
}
