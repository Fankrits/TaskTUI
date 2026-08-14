use crate::app::{App, Tab};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let key_hint = |key: &str, desc: &str| -> Vec<Span> {
        vec![
            Span::styled(
                format!(" {} ", key),
                Style::default()
                    .bg(theme.table_header_bg)
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {} ", desc), Style::default().fg(theme.fg_dim)),
        ]
    };

    let mut hints = Vec::new();
    hints.extend(key_hint("Tab", "Switch Tab"));
    hints.extend(key_hint("↑/↓", "Select"));

    match app.active_tab {
        Tab::Processes => {
            hints.extend(key_hint("v", "Views"));
            hints.extend(key_hint("/", "Search"));
            hints.extend(key_hint("k", "Kill"));
            hints.extend(key_hint("K", "Force Kill"));
            hints.extend(key_hint("Enter", "Details"));
            hints.extend(key_hint("m/c/p/n/o", "Sort"));
            hints.extend(key_hint("r", "Low⇄High"));
        }
        Tab::NetworkPorts => {
            hints.extend(key_hint("/", "Filter"));
            hints.extend(key_hint("L", "Listen Only"));
            hints.extend(key_hint("k", "Kill PID"));
            hints.extend(key_hint("o/p/s", "Sort"));
        }
        Tab::SystemDetails => {
            hints.extend(key_hint("Space", "Pause"));
        }
        Tab::Help => {
            hints.extend(key_hint("Esc", "Close"));
        }
    }

    hints.extend(key_hint("Space", if app.paused { "Resume" } else { "Pause" }));
    hints.extend(key_hint("?", "Help"));
    hints.extend(key_hint("q", "Quit"));

    let footer_paragraph = Paragraph::new(Line::from(hints)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );

    f.render_widget(footer_paragraph, area);
}
