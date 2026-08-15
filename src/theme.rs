use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color,
    pub card_bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub info: Color,
    pub border: Color,
    pub border_focused: Color,
    pub table_header_bg: Color,
    pub table_header_fg: Color,
    pub table_selected_bg: Color,
    pub table_selected_fg: Color,
    pub sparkline_cpu: Color,
    pub sparkline_mem: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            card_bg: Color::Rgb(15, 23, 42),            // Slate 900
            fg: Color::Rgb(241, 245, 249),              // Slate 100
            fg_dim: Color::Rgb(148, 163, 184),          // Slate 400
            primary: Color::Rgb(56, 189, 248),          // Cyan 400
            secondary: Color::Rgb(129, 140, 248),       // Indigo 400
            accent: Color::Rgb(168, 85, 247),           // Purple 500
            success: Color::Rgb(52, 211, 153),          // Emerald 400
            warning: Color::Rgb(251, 191, 36),          // Amber 400
            danger: Color::Rgb(248, 113, 113),          // Rose 400
            info: Color::Rgb(96, 165, 250),             // Blue 400
            border: Color::Rgb(51, 65, 85),             // Slate 700
            border_focused: Color::Rgb(56, 189, 248),   // Cyan 400
            table_header_bg: Color::Rgb(30, 41, 59),    // Slate 800
            table_header_fg: Color::Rgb(125, 211, 252), // Sky 300
            table_selected_bg: Color::Rgb(30, 58, 138), // Blue 900
            table_selected_fg: Color::Rgb(255, 255, 255),
            sparkline_cpu: Color::Rgb(56, 189, 248),
            sparkline_mem: Color::Rgb(168, 85, 247),
        }
    }
}

impl Theme {
    pub fn block<'a>(&self, title: &'a str, focused: bool) -> Block<'a> {
        let border_color = if focused {
            self.border_focused
        } else {
            self.border
        };

        let title_style = if focused {
            Style::default()
                .fg(self.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.fg_dim)
                .add_modifier(Modifier::BOLD)
        };

        Block::default()
            .title(format!(" {} ", title))
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
    }

    pub fn status_color(&self, status: &str) -> Color {
        match status.to_lowercase().as_str() {
            "running" | "run" => self.success,
            "idle" | "sleeping" | "sleep" => self.fg_dim,
            "dead" | "zombie" | "stopped" => self.danger,
            _ => self.info,
        }
    }

    pub fn cpu_color(&self, percent: f32) -> Color {
        if percent > 60.0 {
            self.danger
        } else if percent > 25.0 {
            self.warning
        } else {
            self.success
        }
    }

    pub fn memory_color(&self, percent: f32) -> Color {
        if percent > 85.0 {
            self.danger
        } else if percent > 60.0 {
            self.warning
        } else {
            self.primary
        }
    }
}
