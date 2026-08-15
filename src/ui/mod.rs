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

    /// Flatten a rendered frame into text so tests can assert on what is on
    /// screen.
    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    /// The process table only builds widgets for the rows that fit on screen.
    /// This guards the scroll maths behind that: the selected row must actually
    /// be drawn, and rows outside the window must not be.
    #[test]
    fn test_process_table_virtualized_window_follows_selection() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        app.monitor.processes = (0..500)
            .map(|i| {
                let mut p = crate::system::ProcessInfo::new(
                    i,
                    format!("proc{i:03}"),
                    "tester".to_string(),
                    String::new(),
                );
                p.status = "Run";
                p
            })
            .collect();
        app.sort_column = crate::app::SortColumn::Pid;
        app.sort_direction = crate::app::SortDirection::Ascending;
        app.apply_process_filter_and_sort();

        // Top of the list.
        app.select_first();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let top = buffer_text(&terminal);
        assert!(top.contains("proc000"), "first row should be visible");
        assert!(
            !top.contains("proc499"),
            "last row must not be drawn while scrolled to the top"
        );
        assert_eq!(app.proc_view_offset, 0);

        // Jump to the end: the window must scroll to bring it into view.
        app.select_last();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let bottom = buffer_text(&terminal);
        assert!(
            bottom.contains("proc499"),
            "selected last row should be drawn after scrolling"
        );
        assert!(
            !bottom.contains("proc000"),
            "rows scrolled off the top must not be drawn"
        );
        assert!(
            app.proc_view_offset > 0,
            "view should have scrolled, got offset {}",
            app.proc_view_offset
        );

        // And back to the top again.
        app.select_first();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        assert!(buffer_text(&terminal).contains("proc000"));
        assert_eq!(app.proc_view_offset, 0);
    }

    /// A stale filter index must never panic the renderer, since filtering is
    /// skipped on ticks while another tab is in front.
    #[test]
    fn test_render_survives_stale_filter_indices() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        app.filtered_processes = vec![0, 1, 999_999];
        app.selected_proc_idx = 2;
        terminal.draw(|f| render(f, &mut app)).unwrap();

        app.filtered_sockets = vec![0, 424_242];
        app.selected_net_idx = 1;
        app.active_tab = Tab::NetworkPorts;
        terminal.draw(|f| render(f, &mut app)).unwrap();
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
