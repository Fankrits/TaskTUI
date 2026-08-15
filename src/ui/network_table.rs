use crate::app::{App, NetworkSortColumn, SortDirection};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

pub fn render_network_table(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter / Search Bar
            Constraint::Min(5),    // Sockets Table
        ])
        .split(area);

    let theme = &app.theme;

    // 1. Search & Filter Bar
    {
        let search_title = if app.net_search_active {
            " 🔍 Search Ports (Type to filter, [Enter/Esc] to exit edit) "
        } else {
            " 🔍 Filter Ports (Press [/] to search, [L] toggle Listening only) "
        };

        let search_border_style = if app.net_search_active {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.border)
        };

        let display_text = if app.net_search_query.is_empty() {
            let filter_mode = if app.listening_only {
                " [Showing LISTEN ports only] "
            } else {
                " [Showing All sockets] "
            };
            if app.net_search_active {
                Line::from(vec![
                    Span::styled("Type port, protocol, PID, or process...", Style::default().fg(theme.fg_dim)),
                    Span::styled("█", Style::default().fg(theme.primary)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(filter_mode, Style::default().fg(theme.warning)),
                    Span::styled(" (Press ", Style::default().fg(theme.fg_dim)),
                    Span::styled("/", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(" to search, ", Style::default().fg(theme.fg_dim)),
                    Span::styled("L", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(" toggle listen filter)", Style::default().fg(theme.fg_dim)),
                ])
            }
        } else {
            let cursor = if app.net_search_active { "█" } else { "" };
            Line::from(vec![
                Span::styled("Query: ", Style::default().fg(theme.primary)),
                Span::styled(&app.net_search_query, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled(cursor, Style::default().fg(theme.primary)),
                Span::styled(
                    format!("  ({} matching sockets)", app.filtered_sockets.len()),
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
        let sort_icon = |col: NetworkSortColumn| -> &str {
            if app.net_sort_column == col {
                match app.net_sort_direction {
                    SortDirection::Ascending => " ▲",
                    SortDirection::Descending => " ▼",
                }
            } else {
                ""
            }
        };

        let header_cells = [
            format!("PORT{}", sort_icon(NetworkSortColumn::Port)),
            format!("PROTO{}", sort_icon(NetworkSortColumn::Protocol)),
            format!("STATE{}", sort_icon(NetworkSortColumn::State)),
            "LOCAL ADDRESS".to_string(),
            "REMOTE ADDRESS".to_string(),
            format!("PID{}", sort_icon(NetworkSortColumn::Pid)),
            format!("PROCESS NAME{}", sort_icon(NetworkSortColumn::Name)),
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
            .height(1);

        let rows = app.filtered_sockets.iter().enumerate().map(|(idx, &sock_idx)| {
            let s = &app.monitor.sockets[sock_idx];
            let is_even = idx % 2 == 0;
            let row_bg = if is_even {
                Color::Rgb(15, 23, 42)
            } else {
                Color::Rgb(20, 30, 55)
            };

            let state_color = match s.state.as_str() {
                "LISTEN" => theme.success,
                "ESTABLISHED" => theme.primary,
                "TIME_WAIT" | "CLOSE_WAIT" => theme.warning,
                "CLOSED" => theme.danger,
                _ => theme.fg_dim,
            };

            let pids_str = if s.pids.is_empty() {
                "-".to_string()
            } else {
                s.pids
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let names_str = if s.process_names.is_empty() {
                "-".to_string()
            } else {
                s.process_names.join(", ")
            };

            let cells = vec![
                Cell::from(Span::styled(
                    format!(":{}", s.local_port),
                    Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    &s.protocol,
                    Style::default().fg(theme.secondary),
                )),
                Cell::from(Span::styled(
                    &s.state,
                    Style::default().fg(state_color).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(&s.local_addr, Style::default().fg(theme.fg))),
                Cell::from(Span::styled(&s.remote_addr, Style::default().fg(theme.fg_dim))),
                Cell::from(Span::styled(pids_str, Style::default().fg(theme.primary))),
                Cell::from(Span::styled(
                    names_str,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                )),
            ];

            Row::new(cells).style(Style::default().bg(row_bg)).height(1)
        });

        let table_title = format!(
            " Port Sockets [ Total: {} │ Matching: {} ] ",
            app.monitor.sockets.len(),
            app.filtered_sockets.len()
        );

        let table = Table::new(
            rows,
            [
                Constraint::Length(10), // Port
                Constraint::Length(8),  // Protocol
                Constraint::Length(14), // State
                Constraint::Length(24), // Local Addr
                Constraint::Length(24), // Remote Addr
                Constraint::Length(12), // PID
                Constraint::Min(20),    // Process Name
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(table_title)
                .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
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

        f.render_stateful_widget(table, chunks[1], &mut app.network_table_state);
    }
}
