use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};

pub fn render_system_info(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Host & CPU specs
            Constraint::Min(8),    // Disks Table
            Constraint::Length(8), // Network Interfaces
        ])
        .split(area);

    let theme = &app.theme;

    // 1. Host & CPU Specs Block
    {
        let block = theme.block("System & Hardware Specifications", true);
        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        let left_lines = vec![
            Line::from(vec![
                Span::styled(
                    "Host Name:    ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&app.monitor.host_name, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "OS:           ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} {}", app.monitor.os_name, app.monitor.os_version),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Kernel:       ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &app.monitor.kernel_version,
                    Style::default().fg(theme.fg_dim),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(left_lines), col_chunks[0]);

        let right_lines = vec![
            Line::from(vec![
                Span::styled(
                    "Processor:    ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&app.monitor.cpu_brand, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Cores:        ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} Physical/Logical Cores", app.monitor.cpu_core_count),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Total RAM:    ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "{:.2} GB",
                        app.monitor.total_memory as f64 / (1024.0 * 1024.0 * 1024.0)
                    ),
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(right_lines), col_chunks[1]);
    }

    // 2. Storage Disks Table
    {
        let disks = app.monitor.get_disks_info();
        let header = Row::new(vec![
            Cell::from(Span::styled(
                "DRIVE / NAME",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "MOUNT POINT",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "FS TYPE",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "USED",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "AVAILABLE",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "TOTAL",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "USAGE %",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .style(Style::default().bg(theme.table_header_bg))
        .height(1);

        let rows = disks.iter().enumerate().map(|(idx, d)| {
            let used_bytes = d.total_bytes.saturating_sub(d.available_bytes);
            let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let avail_gb = d.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let total_gb = d.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let percent = if total_gb > 0.0 {
                (used_gb / total_gb * 100.0) as f32
            } else {
                0.0
            };

            let percent_color = if percent > 85.0 {
                theme.danger
            } else if percent > 70.0 {
                theme.warning
            } else {
                theme.success
            };

            let row_bg = if idx % 2 == 0 {
                Color::Rgb(15, 23, 42)
            } else {
                Color::Rgb(20, 30, 55)
            };

            let cells = vec![
                Cell::from(Span::styled(
                    &d.name,
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(&d.mount_point, Style::default().fg(theme.fg))),
                Cell::from(Span::styled(
                    &d.file_system,
                    Style::default().fg(theme.fg_dim),
                )),
                Cell::from(Span::styled(
                    format!("{:.1} GB", used_gb),
                    Style::default().fg(theme.fg),
                )),
                Cell::from(Span::styled(
                    format!("{:.1} GB", avail_gb),
                    Style::default().fg(theme.success),
                )),
                Cell::from(Span::styled(
                    format!("{:.1} GB", total_gb),
                    Style::default().fg(theme.fg),
                )),
                Cell::from(Span::styled(
                    format!("{:>5.1}%", percent),
                    Style::default()
                        .fg(percent_color)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            Row::new(cells).style(Style::default().bg(row_bg)).height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(18),
                Constraint::Length(16),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Storage Disks & Partitions ")
                .title_style(
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        );

        f.render_widget(table, chunks[1]);
    }

    // 3. Network Interfaces
    {
        let mut net_rows = Vec::new();
        for (idx, (interface_name, data)) in app.monitor.networks.iter().enumerate() {
            let row_bg = if idx % 2 == 0 {
                Color::Rgb(15, 23, 42)
            } else {
                Color::Rgb(20, 30, 55)
            };

            let rx_mb = data.total_received() as f64 / (1024.0 * 1024.0);
            let tx_mb = data.total_transmitted() as f64 / (1024.0 * 1024.0);

            let cells = vec![
                Cell::from(Span::styled(
                    interface_name.as_str(),
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    format!("{:.2} MB", rx_mb),
                    Style::default().fg(theme.success),
                )),
                Cell::from(Span::styled(
                    format!("{:.2} MB", tx_mb),
                    Style::default().fg(theme.info),
                )),
                Cell::from(Span::styled(
                    format!("{} pkts", data.total_packets_received()),
                    Style::default().fg(theme.fg_dim),
                )),
                Cell::from(Span::styled(
                    format!("{} pkts", data.total_packets_transmitted()),
                    Style::default().fg(theme.fg_dim),
                )),
            ];
            net_rows.push(Row::new(cells).style(Style::default().bg(row_bg)).height(1));
        }

        let header = Row::new(vec![
            Cell::from(Span::styled(
                "INTERFACE",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "TOTAL RECEIVED",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "TOTAL TRANSMITTED",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "PACKETS RX",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "PACKETS TX",
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .style(Style::default().bg(theme.table_header_bg))
        .height(1);

        let table = Table::new(
            net_rows,
            [
                Constraint::Length(24),
                Constraint::Length(18),
                Constraint::Length(18),
                Constraint::Length(16),
                Constraint::Min(16),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title(" Network Interfaces ")
                .title_style(
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        );

        f.render_widget(table, chunks[2]);
    }
}
