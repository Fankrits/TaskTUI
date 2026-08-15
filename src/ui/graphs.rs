use crate::app::{App, DashboardView};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph, Sparkline};

pub fn render_graphs(f: &mut Frame, app: &App, area: Rect) {
    match app.dashboard_view {
        DashboardView::Combined => render_combined_view(f, app, area),
        DashboardView::TopRank => render_rank_only_view(f, app, area),
        DashboardView::GraphsOnly => render_graphs_only_view(f, app, area),
    }
}

fn render_combined_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left: CPU & Memory Graphs
            Constraint::Percentage(50), // Right: Top Process Rankings
        ])
        .split(area);

    // Left: System Graphs
    let left_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // CPU
            Constraint::Percentage(50), // Memory
        ])
        .split(chunks[0]);

    render_cpu_box(f, app, left_chunks[0]);
    render_memory_box(f, app, left_chunks[1]);

    // Right: Top Rankings
    let right_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Top CPU
            Constraint::Percentage(50), // Top Memory
        ])
        .split(chunks[1]);

    render_top_cpu_rank(f, app, right_chunks[0], 4);
    render_top_memory_rank(f, app, right_chunks[1], 4);
}

fn render_rank_only_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Top CPU
            Constraint::Percentage(50), // Top Memory
        ])
        .split(area);

    render_top_cpu_rank(f, app, chunks[0], 5);
    render_top_memory_rank(f, app, chunks[1], 5);
}

fn render_graphs_only_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34), // CPU
            Constraint::Percentage(33), // Memory
            Constraint::Percentage(33), // Network
        ])
        .split(area);

    render_cpu_box(f, app, chunks[0]);
    render_memory_box(f, app, chunks[1]);
    render_network_box(f, app, chunks[2]);
}

fn render_cpu_box(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let cpu_block = theme.block("CPU Usage", false);
    let inner = cpu_block.inner(area);
    f.render_widget(cpu_block, area);

    let cpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top label
            Constraint::Length(1), // Line gauge
            Constraint::Min(2),    // Sparkline history
        ])
        .split(inner);

    let cpu_percent = app.monitor.global_cpu.clamp(0.0, 100.0);
    let cpu_color = theme.cpu_color(cpu_percent);

    let header_line = Line::from(vec![
        Span::styled("Total: ", Style::default().fg(theme.fg_dim)),
        Span::styled(
            format!("{:>5.1}%", cpu_percent),
            Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} Cores)", app.monitor.cpu_core_count),
            Style::default().fg(theme.fg_dim),
        ),
    ]);
    f.render_widget(Paragraph::new(header_line), cpu_chunks[0]);

    let cpu_gauge = LineGauge::default()
        .ratio((cpu_percent / 100.0) as f64)
        .filled_style(Style::default().fg(cpu_color))
        .unfilled_style(Style::default().fg(Color::Rgb(30, 41, 59)))
        .filled_symbol("━")
        .unfilled_symbol("─");
    f.render_widget(cpu_gauge, cpu_chunks[1]);

    let sparkline = Sparkline::default()
        .data(app.monitor.cpu_history)
        .max(100)
        .style(Style::default().fg(cpu_color));
    f.render_widget(sparkline, cpu_chunks[2]);
}

fn render_memory_box(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let mem_block = theme.block("Memory & Swap", false);
    let inner = mem_block.inner(area);
    f.render_widget(mem_block, area);

    let mem_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Mem info line
            Constraint::Length(1), // Mem gauge
            Constraint::Min(2),    // Mem sparkline history
        ])
        .split(inner);

    let total_mem_gb = app.monitor.total_memory as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_mem_gb = app.monitor.used_memory as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_percent = if total_mem_gb > 0.0 {
        (used_mem_gb / total_mem_gb * 100.0) as f32
    } else {
        0.0
    };
    let mem_color = theme.memory_color(mem_percent);

    let mem_header = Line::from(vec![
        Span::styled("RAM: ", Style::default().fg(theme.fg_dim)),
        Span::styled(
            format!("{:.2} / {:.2} GB", used_mem_gb, total_mem_gb),
            Style::default().fg(theme.fg),
        ),
        Span::styled(
            format!(" ({:.1}%)", mem_percent),
            Style::default().fg(mem_color).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(mem_header), mem_chunks[0]);

    let ratio = if total_mem_gb > 0.0 {
        (used_mem_gb / total_mem_gb).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mem_gauge = LineGauge::default()
        .ratio(ratio)
        .filled_style(Style::default().fg(mem_color))
        .unfilled_style(Style::default().fg(Color::Rgb(30, 41, 59)))
        .filled_symbol("━")
        .unfilled_symbol("─");
    f.render_widget(mem_gauge, mem_chunks[1]);

    let sparkline = Sparkline::default()
        .data(app.monitor.mem_history)
        .max(100)
        .style(Style::default().fg(theme.sparkline_mem));
    f.render_widget(sparkline, mem_chunks[2]);
}

fn render_network_box(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let net_block = theme.block("Network Activity", false);
    let inner = net_block.inner(area);
    f.render_widget(net_block, area);

    let net_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner);

    let rx_rate = app.monitor.current_net_rx_rate;
    let tx_rate = app.monitor.current_net_tx_rate;

    let format_rate = |rate_bytes: f64| -> String {
        if rate_bytes >= 1024.0 * 1024.0 {
            format!("{:.2} MB/s", rate_bytes / (1024.0 * 1024.0))
        } else if rate_bytes >= 1024.0 {
            format!("{:.1} KB/s", rate_bytes / 1024.0)
        } else {
            format!("{:.0} B/s", rate_bytes)
        }
    };

    let net_info = Line::from(vec![
        Span::styled("▼ RX: ", Style::default().fg(theme.success)),
        Span::styled(format_rate(rx_rate), Style::default().fg(theme.fg)),
        Span::raw("   "),
        Span::styled("▲ TX: ", Style::default().fg(theme.secondary)),
        Span::styled(format_rate(tx_rate), Style::default().fg(theme.fg)),
    ]);
    f.render_widget(Paragraph::new(net_info), net_chunks[0]);

    let max_rx = app
        .monitor
        .net_rx_history
        .iter()
        .copied()
        .max()
        .unwrap_or(10)
        .max(10);
    let rx_sparkline = Sparkline::default()
        .data(app.monitor.net_rx_history)
        .max(max_rx)
        .style(Style::default().fg(theme.success));
    f.render_widget(rx_sparkline, net_chunks[1]);

    let max_tx = app
        .monitor
        .net_tx_history
        .iter()
        .copied()
        .max()
        .unwrap_or(10)
        .max(10);
    let tx_sparkline = Sparkline::default()
        .data(app.monitor.net_tx_history)
        .max(max_tx)
        .style(Style::default().fg(theme.secondary));
    f.render_widget(tx_sparkline, net_chunks[2]);
}

fn render_top_cpu_rank(f: &mut Frame, app: &App, area: Rect, limit: usize) {
    let theme = &app.theme;
    let block = Block::default()
        .title(" 🔥 Top CPU Consumers ")
        .title_style(
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    let rank_colors = [
        Color::Rgb(251, 191, 36),  // #1 Gold
        Color::Rgb(226, 232, 240), // #2 Silver
        Color::Rgb(205, 127, 50),  // #3 Bronze
        Color::Rgb(148, 163, 184), // #4 Dim
        Color::Rgb(100, 116, 139), // #5 Muted
    ];

    for (idx, &proc_idx) in app.top_cpu_indices.iter().take(limit).enumerate() {
        if let Some(p) = app.monitor.processes.get(proc_idx) {
            let rank_color = rank_colors.get(idx).copied().unwrap_or(theme.fg_dim);
            let rank_num = format!("#{} ", idx + 1);

            let proc_name = if p.name.chars().count() > 14 {
                let prefix: String = p.name.chars().take(13).collect();
                format!("{}…", prefix)
            } else {
                p.name.clone()
            };

            let cpu_color = theme.cpu_color(p.cpu_usage);
            let bar_width = 8;
            let filled = ((p.cpu_usage.max(0.0) / 100.0) * bar_width as f32).round() as usize;
            let filled = filled.min(bar_width);
            let bar_str = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width - filled));

            lines.push(Line::from(vec![
                Span::styled(
                    rank_num,
                    Style::default().fg(rank_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<14}", proc_name),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(bar_str, Style::default().fg(cpu_color)),
                Span::styled(
                    format!(" {:>5.1}%", p.cpu_usage),
                    Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No processes active",
            Style::default().fg(theme.fg_dim),
        )));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_top_memory_rank(f: &mut Frame, app: &App, area: Rect, limit: usize) {
    let theme = &app.theme;
    let block = Block::default()
        .title(" 🧠 Top Memory Consumers ")
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    let rank_colors = [
        Color::Rgb(251, 191, 36),  // #1 Gold
        Color::Rgb(226, 232, 240), // #2 Silver
        Color::Rgb(205, 127, 50),  // #3 Bronze
        Color::Rgb(148, 163, 184), // #4 Dim
        Color::Rgb(100, 116, 139), // #5 Muted
    ];

    for (idx, &proc_idx) in app.top_mem_indices.iter().take(limit).enumerate() {
        if let Some(p) = app.monitor.processes.get(proc_idx) {
            let rank_color = rank_colors.get(idx).copied().unwrap_or(theme.fg_dim);
            let rank_num = format!("#{} ", idx + 1);

            let proc_name = if p.name.chars().count() > 14 {
                let prefix: String = p.name.chars().take(13).collect();
                format!("{}…", prefix)
            } else {
                p.name.clone()
            };

            let mem_color = theme.memory_color(p.memory_percent);
            let bar_width = 8;
            let filled = ((p.memory_percent.max(0.0) / 100.0) * bar_width as f32).round() as usize;
            let filled = filled.min(bar_width);
            let bar_str = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width - filled));

            let mem_str = format_memory_compact(p.memory_bytes);

            lines.push(Line::from(vec![
                Span::styled(
                    rank_num,
                    Style::default().fg(rank_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<14}", proc_name),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(bar_str, Style::default().fg(mem_color)),
                Span::styled(
                    format!(" {:>7}", mem_str),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No processes active",
            Style::default().fg(theme.fg_dim),
        )));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn format_memory_compact(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    let gb = mb / 1024.0;
    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else {
        format!("{:.0} MB", mb)
    }
}
