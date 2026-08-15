use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

pub fn render_help_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let theme = &app.theme;

    // Left Column
    {
        let left_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                " 📑 TAB NAVIGATION & VIEWS",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   Tab / BackTab   ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Cycle through tabs (1: Processes, 2: Ports, 3: Specs, 4: Help)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   1, 2, 3, 4      ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Jump directly to corresponding tab number",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   ↑ / ↓, j / k    ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Navigate rows up and down in table",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   PgUp / PgDn     ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Scroll 15 rows up / down", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Home / End      ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Jump to top / bottom of process list",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " ⚙️ PROCESS MANAGEMENT & INSPECTION",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   k / Delete      ",
                    Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Terminate selected process (Confirmation prompt)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   K (Shift+k)     ",
                    Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Force kill selected process", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Enter / d       ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Inspect deep process details (Paths, Args, Disk I/O, Sockets)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " 🎛️ DASHBOARD & REAL-TIME CONTROLS",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   v               ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Cycle dashboard views (Combined ⇄ Top Rankings ⇄ Graphs)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Space           ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Pause / Resume real-time background metric updates",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   F5              ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Manually refresh all system and socket data",
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];

        let left_block = Block::default()
            .title(" Navigation & Process Controls ")
            .title_style(
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));

        f.render_widget(Paragraph::new(left_lines).block(left_block), chunks[0]);
    }

    // Right Column
    {
        let right_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                " 🔍 SEARCH & FILTERING",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   /               ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Open real-time search query (Type name, PID, port, or user)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Esc             ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Exit search input mode or clear current filter query",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   L (Ports tab)   ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Toggle filter: Listening ports only vs All active sockets",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " 📊 COLUMN SORTING (HIGH ⇄ LOW)",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   m               ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Sort by Memory usage (RAM) — Press again to toggle High/Low",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   c               ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Sort by CPU % usage — Press again to toggle High/Low",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   p / n / o       ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Sort by: (p) PID, (n) Process Name, (o) Port Number",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   r               ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Directly invert sort order (High → Low ▼ ⇄ Low → High ▲)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " 🖱️ MOUSE INTERACTIONS",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   Click Tab Bar   ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Switch between views (1: Processes, 2: Ports, 3: Specs, 4: Help)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Click Header    ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Click any column header to sort (PID, Name, CPU %, Mem, Ports)",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Click Row       ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Click row to select; Click again / Double-Click to inspect",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Right-Click Row ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Directly open Kill Confirmation prompt for clicked process",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Scroll Wheel    ",
                    Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Scroll table rows up and down smoothly",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " 🚪 APPLICATION QUIT",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "   q / Ctrl+C      ",
                    Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Cleanly exit Task TUI and restore normal terminal mode",
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];

        let right_block = Block::default()
            .title(" Filtering, Sorting & Mouse Gestures ")
            .title_style(
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));

        f.render_widget(Paragraph::new(right_lines).block(right_block), chunks[1]);
    }
}
