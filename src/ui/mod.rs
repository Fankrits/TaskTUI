pub mod footer;
pub mod graphs;
pub mod header;
pub mod help_view;
pub mod modals;
pub mod network_table;
pub mod process_table;
pub mod system_info;

use crate::app::{App, Tab};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

pub fn render(f: &mut Frame, app: &mut App) {
    if app.active_tab == Tab::Help {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Full-page Help & Guide Area
                Constraint::Length(3), // Footer / Shortcuts
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
            Constraint::Length(3), // Header
            Constraint::Length(8), // Graphs / Metrics Dashboard
            Constraint::Min(10),   // Main Tab Content Area
            Constraint::Length(3), // Footer / Shortcuts
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DashboardView, Modal, ToastKind};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_all_tabs_various_sizes() {
        let sizes = [(80, 24), (120, 40), (200, 60), (60, 20)];
        let tabs = [
            Tab::Processes,
            Tab::NetworkPorts,
            Tab::SystemDetails,
            Tab::Help,
        ];

        for &(w, h) in &sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            for &tab in &tabs {
                let mut app = App::new();
                app.switch_tab(tab);

                terminal.draw(|f| render(f, &mut app)).unwrap();
            }
        }
    }

    #[test]
    fn test_render_all_dashboard_views() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let views = [
            DashboardView::Combined,
            DashboardView::TopRank,
            DashboardView::GraphsOnly,
        ];

        for &view in &views {
            let mut app = App::new();
            app.dashboard_view = view;
            terminal.draw(|f| render(f, &mut app)).unwrap();
        }
    }

    #[test]
    fn test_render_modals_and_toasts() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let modals = [
            Modal::None,
            Modal::ConfirmKill {
                pid: 1234,
                name: "test_process".to_string(),
                force: false,
            },
            Modal::ConfirmKill {
                pid: 1234,
                name: "test_process".to_string(),
                force: true,
            },
            Modal::ProcessDetails(1234),
            Modal::Help,
        ];

        for modal in modals {
            let mut app = App::new();
            app.active_modal = modal;
            app.add_toast("Test Toast Success".to_string(), ToastKind::Success);
            app.add_toast("Test Toast Error".to_string(), ToastKind::Error);
            app.add_toast("Test Toast Info".to_string(), ToastKind::Info);

            terminal.draw(|f| render(f, &mut app)).unwrap();
        }
    }

    #[test]
    fn test_render_populated_tables_and_search_mode() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        // Processes tab with search active
        app.switch_tab(Tab::Processes);
        app.search_active = true;
        app.search_query = "cargo".to_string();
        app.apply_process_filter_and_sort();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Network tab with search and listen filter active
        app.switch_tab(Tab::NetworkPorts);
        app.net_search_active = true;
        app.net_search_query = "80".to_string();
        app.listening_only = true;
        app.apply_network_filter_and_sort();
        terminal.draw(|f| render(f, &mut app)).unwrap();

        // Top rankings with mock data
        app.update_top_rankings();
        app.dashboard_view = DashboardView::TopRank;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
}
