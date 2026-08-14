pub mod footer;
pub mod graphs;
pub mod header;
pub mod help_view;
pub mod modals;
pub mod network_table;
pub mod process_table;
pub mod system_info;

use crate::app::{App, Tab};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &mut App) {
    if app.active_tab == Tab::Help {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),   // Full-page Help & Guide Area
                Constraint::Length(3),  // Footer / Shortcuts
            ])
            .split(f.area());

        // Header
        header::render_header(f, app, chunks[0]);

        // Full-page Help View
        help_view::render_help_view(f, app, chunks[1]);

        // Footer
        footer::render_footer(f, app, chunks[2]);

        // Toasts / Modals (if any)
        modals::render_modals(f, app);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(8),  // Graphs / Metrics Dashboard
            Constraint::Min(10),   // Main Tab Content Area
            Constraint::Length(3),  // Footer / Shortcuts
        ])
        .split(f.area());

    // 1. Render Header
    header::render_header(f, app, chunks[0]);

    // 2. Render Usage Graphs
    graphs::render_graphs(f, app, chunks[1]);

    // 3. Render Main View depending on active tab
    match app.active_tab {
        Tab::Processes => {
            process_table::render_process_table(f, app, chunks[2]);
        }
        Tab::NetworkPorts => {
            network_table::render_network_table(f, app, chunks[2]);
        }
        Tab::SystemDetails => {
            system_info::render_system_info(f, app, chunks[2]);
        }
        Tab::Help => {
            help_view::render_help_view(f, app, chunks[2]);
        }
    }

    // 4. Render Footer
    footer::render_footer(f, app, chunks[3]);

    // 5. Render Any Active Popups / Modals / Toasts on top
    modals::render_modals(f, app);
}
