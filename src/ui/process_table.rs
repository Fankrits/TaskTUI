use crate::app::{App, SortColumn, SortDirection};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};

pub fn render_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search / Filter Input Bar
            Constraint::Min(5),    // Table Area
        ])
        .split(area);

    let theme = &app.theme;

    // 1. Search & Sort Indicator Bar
    {
        let search_title = if app.search_active {
            " 🔍 Search Filter (Type to filter, [Enter/Esc] to exit edit) "
        } else {
            " 🔍 Filter Processes & Sort Controls "
        };

        let search_border_style = if app.search_active {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.border)
        };

        let sort_summary = app.get_sort_summary();

        let display_text = if app.search_query.is_empty() {
            if app.search_active {
                Line::from(vec![
                    Span::styled(
                        "Type name, PID, port, or user...",
                        Style::default().fg(theme.fg_dim),
                    ),
                    Span::styled("█", Style::default().fg(theme.primary)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" Sort: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(
                        format!(" {} ", sort_summary),
                        Style::default()
                            .bg(theme.table_selected_bg)
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" (Press ", Style::default().fg(theme.fg_dim)),
                    Span::styled(
                        "r",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to invert Low⇄High, ", Style::default().fg(theme.fg_dim)),
                    Span::styled(
                        "m/c/p/n/o",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to sort, ", Style::default().fg(theme.fg_dim)),
                    Span::styled(
                        "/",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to search)", Style::default().fg(theme.fg_dim)),
                ])
            }
        } else {
            let cursor = if app.search_active { "█" } else { "" };
            Line::from(vec![
                Span::styled("Query: ", Style::default().fg(theme.primary)),
                Span::styled(
                    &app.search_query,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(cursor, Style::default().fg(theme.primary)),
                Span::styled(
                    format!(
                        "  ({} matching)  │  Sort: {}",
                        app.filtered_processes.len(),
                        sort_summary
                    ),
                    Style::default().fg(theme.fg_dim),
                ),
            ])
        };

        let search_widget = Paragraph::new(display_text).block(
            Block::default()
                .title(search_title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(search_border_style),
        );
        f.render_widget(search_widget, chunks[0]);
    }

    // 2. Table Area
    {
        let sort_icon = |col: SortColumn| -> &str {
            if app.sort_column == col {
                match app.sort_direction {
                    SortDirection::Ascending => " ▲",
                    SortDirection::Descending => " ▼",
                }
            } else {
                ""
            }
        };

        let header_cells = [
            format!("PID{}", sort_icon(SortColumn::Pid)),
            format!("NAME{}", sort_icon(SortColumn::Name)),
            format!("CPU %{}", sort_icon(SortColumn::Cpu)),
            format!("MEM{}", sort_icon(SortColumn::Memory)),
            "MEM %".to_string(),
            format!("PORTS{}", sort_icon(SortColumn::Port)),
            format!("STATUS{}", sort_icon(SortColumn::Status)),
            format!("USER / SESS{}", sort_icon(SortColumn::User)),
            "COMMAND".to_string(),
        ]
        .into_iter()
        .map(|h| {
            Cell::from(Span::styled(
                h,
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            ))
        });

        let header = Row::new(header_cells)
            .style(Style::default().bg(theme.table_header_bg))
            .height(1)
            .bottom_margin(0);

        let rows = app
            .filtered_processes
            .iter()
            .enumerate()
            .map(|(display_idx, &real_idx)| {
                let p = &app.monitor.processes[real_idx];
                let is_even = display_idx % 2 == 0;
                let row_bg = if is_even {
                    Color::Rgb(15, 23, 42)
                } else {
                    Color::Rgb(20, 30, 55)
                };

                let cpu_color = theme.cpu_color(p.cpu_usage);
                let mem_color = theme.memory_color(p.memory_percent);

                let status_icon = match p.status {
                    "Run" => "● Run",
                    "Sleep" => "○ Slp",
                    "Idle" => "▲ Idl",
                    "Zombie" | "Dead" => "✕ Ded",
                    _ => "• Oth",
                };

                let ports_str = if p.ports.is_empty() {
                    "-".to_string()
                } else {
                    p.ports
                        .iter()
                        .take(3)
                        .map(|port| format!(":{}", port))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let user_sess = match p.session_id {
                    Some(sess) => format!("{} (S:{})", p.user, sess),
                    None => p.user.clone(),
                };

                let cells = vec![
                    Cell::from(Span::styled(
                        p.pid.to_string(),
                        Style::default().fg(theme.primary),
                    )),
                    Cell::from(Span::styled(
                        &p.name,
                        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        format!("{:>5.1}%", p.cpu_usage),
                        Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        format_memory(p.memory_bytes),
                        Style::default().fg(theme.fg),
                    )),
                    Cell::from(Span::styled(
                        format!("{:>4.1}%", p.memory_percent),
                        Style::default().fg(mem_color),
                    )),
                    Cell::from(Span::styled(
                        ports_str,
                        Style::default().fg(if p.ports.is_empty() {
                            theme.fg_dim
                        } else {
                            theme.warning
                        }),
                    )),
                    Cell::from(Span::styled(
                        status_icon,
                        Style::default().fg(theme.status_color(p.status)),
                    )),
                    Cell::from(Span::styled(user_sess, Style::default().fg(theme.fg_dim))),
                    Cell::from(Span::styled(&p.cmd, Style::default().fg(theme.fg_dim))),
                ];

                Row::new(cells).style(Style::default().bg(row_bg)).height(1)
            });

        let selected_pid_str = app
            .get_selected_process()
            .map(|p| format!(" │ Selected PID: {} ({})", p.pid, p.name))
            .unwrap_or_default();

        let table_title = format!(
            " Tasks [ Total: {} │ Matching: {}{} ] ",
            app.monitor.processes.len(),
            app.filtered_processes.len(),
            selected_pid_str
        );

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),  // PID
                Constraint::Length(22), // Name
                Constraint::Length(9),  // CPU %
                Constraint::Length(11), // Memory
                Constraint::Length(8),  // Memory %
                Constraint::Length(16), // Ports
                Constraint::Length(9),  // Status
                Constraint::Length(18), // User / Session
                Constraint::Min(20),    // Command line
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(table_title)
                .title_style(
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused)),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme.table_selected_bg)
                .fg(theme.table_selected_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

        f.render_stateful_widget(table, chunks[1], &mut app.process_table_state);
    }
}

fn format_memory(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;

    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else if mb >= 1.0 {
        format!("{:.1} MB", mb)
    } else if kb >= 1.0 {
        format!("{:.0} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}
