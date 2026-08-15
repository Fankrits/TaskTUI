use crate::app::{App, DashboardView, Modal, NetworkSortColumn, SortColumn, Tab, ToastKind};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::Duration;

pub fn handle_events(app: &mut App) -> Result<bool, anyhow::Error> {
    let mut handled = false;
    while event::poll(Duration::from_millis(0))? {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key);
                    handled = true;
                }
            }
            Event::Mouse(mouse) => {
                if handle_mouse_event(app, mouse) {
                    handled = true;
                }
            }
            Event::Resize(_, _) => {
                handled = true;
            }
            _ => {}
        }
    }
    Ok(handled)
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    // 1. Handle Active Modals
    match &app.active_modal {
        Modal::ConfirmKill { pid, force, .. } => {
            let target_pid = *pid;
            let was_force = *force;
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    app.execute_kill(target_pid, was_force);
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.execute_kill(target_pid, true);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.active_modal = Modal::None;
                }
                _ => {}
            }
            return;
        }
        Modal::ProcessDetails(pid) => {
            let target_pid = *pid;
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    app.active_modal = Modal::None;
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    let force = key.code == KeyCode::Char('K') || key.modifiers.contains(KeyModifiers::SHIFT);
                    if let Some(proc) = app.monitor.processes.iter().find(|p| p.pid == target_pid) {
                        app.active_modal = Modal::ConfirmKill {
                            pid: target_pid,
                            name: proc.name.clone(),
                            force,
                        };
                    }
                }
                _ => {}
            }
            return;
        }
        Modal::Help => {
            match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q') => {
                    app.active_modal = Modal::None;
                }
                _ => {}
            }
            return;
        }
        Modal::None => {}
    }

    // 2. Handle Search Input Mode
    if app.search_active {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.search_active = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.apply_process_filter_and_sort();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.apply_process_filter_and_sort();
            }
            _ => {}
        }
        return;
    }

    if app.net_search_active {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.net_search_active = false;
            }
            KeyCode::Backspace => {
                app.net_search_query.pop();
                app.apply_network_filter_and_sort();
            }
            KeyCode::Char(c) => {
                app.net_search_query.push(c);
                app.apply_network_filter_and_sort();
            }
            _ => {}
        }
        return;
    }

    // 3. Normal Mode Global Shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }

        // Help
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.active_modal = Modal::Help;
        }

        // Tab Navigation
        KeyCode::Tab => {
            app.switch_tab(app.active_tab.next());
        }
        KeyCode::BackTab => {
            app.switch_tab(app.active_tab.prev());
        }
        KeyCode::Char('1') => app.switch_tab(Tab::Processes),
        KeyCode::Char('2') => app.switch_tab(Tab::NetworkPorts),
        KeyCode::Char('3') => app.switch_tab(Tab::SystemDetails),
        KeyCode::Char('4') => app.switch_tab(Tab::Help),

        // Vertical row navigation
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_row();
        }
        KeyCode::Up | KeyCode::Char('k') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            app.prev_row();
        }
        KeyCode::PageDown => {
            app.page_down(15);
        }
        KeyCode::PageUp => {
            app.page_up(15);
        }
        KeyCode::Home => {
            app.select_first();
        }
        KeyCode::End => {
            app.select_last();
        }

        // Process Killing
        KeyCode::Char('k') | KeyCode::Delete => {
            app.prompt_kill_selected(false);
        }
        KeyCode::Char('K') => {
            app.prompt_kill_selected(true);
        }

        // Process Details / Inspector
        KeyCode::Enter | KeyCode::Char('d') => {
            if app.active_tab == Tab::Processes {
                if let Some(proc) = app.get_selected_process() {
                    app.active_modal = Modal::ProcessDetails(proc.pid);
                }
            } else if app.active_tab == Tab::NetworkPorts
                && let Some(&pid) = app.get_selected_socket().and_then(|s| s.pids.first())
            {
                app.active_modal = Modal::ProcessDetails(pid);
            }
        }

        // Search & Filter
        KeyCode::Char('/') => {
            if app.active_tab == Tab::Processes {
                app.search_active = true;
            } else if app.active_tab == Tab::NetworkPorts {
                app.net_search_active = true;
            }
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.apply_process_filter_and_sort();
                app.add_toast("Search filter cleared".to_string(), ToastKind::Info);
            }
            if !app.net_search_query.is_empty() {
                app.net_search_query.clear();
                app.apply_network_filter_and_sort();
            }
        }

        // Cycle Dashboard View (Combined / Top Rankings / Graphs)
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.dashboard_view = app.dashboard_view.next();
            let view_name = match app.dashboard_view {
                DashboardView::Combined => "Dashboard: Combined (Graphs + Top Rank)",
                DashboardView::TopRank => "Dashboard: Top Process Rankings",
                DashboardView::GraphsOnly => "Dashboard: System Usage Graphs",
            };
            app.add_toast(view_name.to_string(), ToastKind::Info);
        }

        // Sorting (Processes Tab)
        KeyCode::Char('m') if app.active_tab == Tab::Processes => {
            app.set_sort(SortColumn::Memory);
        }
        KeyCode::Char('c') if app.active_tab == Tab::Processes => {
            app.set_sort(SortColumn::Cpu);
        }
        KeyCode::Char('p') if app.active_tab == Tab::Processes => {
            app.set_sort(SortColumn::Pid);
        }
        KeyCode::Char('n') if app.active_tab == Tab::Processes => {
            app.set_sort(SortColumn::Name);
        }
        KeyCode::Char('o') if app.active_tab == Tab::Processes => {
            app.set_sort(SortColumn::Port);
        }
        KeyCode::Char('r') if app.active_tab == Tab::Processes => {
            app.toggle_sort_direction();
        }

        // Network Tab Controls
        KeyCode::Char('L') | KeyCode::Char('l') if app.active_tab == Tab::NetworkPorts => {
            app.listening_only = !app.listening_only;
            app.apply_network_filter_and_sort();
            let msg = if app.listening_only {
                "Filter: Listening ports only"
            } else {
                "Filter: All sockets"
            };
            app.add_toast(msg.to_string(), ToastKind::Info);
        }
        KeyCode::Char('o') if app.active_tab == Tab::NetworkPorts => {
            app.set_net_sort(NetworkSortColumn::Port);
        }
        KeyCode::Char('s') if app.active_tab == Tab::NetworkPorts => {
            app.set_net_sort(NetworkSortColumn::State);
        }

        // Pause / Resume
        KeyCode::Char(' ') => {
            app.paused = !app.paused;
            let msg = if app.paused { "Paused Live Refresh" } else { "Resumed Live Refresh" };
            app.add_toast(msg.to_string(), ToastKind::Info);
        }

        // Manual Refresh
        KeyCode::F(5) => {
            app.monitor.refresh();
            app.monitor.refresh_sockets();
            app.apply_process_filter_and_sort();
            app.apply_network_filter_and_sort();
            app.add_toast("Refreshed data".to_string(), ToastKind::Info);
        }

        _ => {}
    }
}

fn get_modal_bounds(percent_x: u16, percent_y: u16, width: u16, height: u16) -> (u16, u16, u16, u16) {
    let pad_x = (width.saturating_sub(width * percent_x / 100)) / 2;
    let w = width * percent_x / 100;
    let pad_y = (height.saturating_sub(height * percent_y / 100)) / 2;
    let h = height * percent_y / 100;
    (pad_x, pad_y, w, h)
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> bool {
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((100, 30));
    let x = mouse.column;
    let y = mouse.row;

    // 1. Mouse Scroll Wheel
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.prev_row();
            return true;
        }
        MouseEventKind::ScrollDown => {
            app.next_row();
            return true;
        }
        _ => {}
    }

    // 2. Right Click -> Trigger Kill Confirmation on clicked row
    if mouse.kind == MouseEventKind::Down(MouseButton::Right) {
        if y >= 16 && y < term_height.saturating_sub(4) {
            let offset = app.process_table_state.offset();
            let row_idx = offset + (y - 16) as usize;
            if app.active_tab == Tab::Processes && row_idx < app.filtered_processes.len() {
                app.selected_proc_idx = row_idx;
                app.process_table_state.select(Some(row_idx));
                app.prompt_kill_selected(false);
                return true;
            }
        }
        return false;
    }

    // 3. Left Click Handling
    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        // Modal Hit Testing
        match &app.active_modal {
            Modal::ConfirmKill { pid, .. } => {
                let target_pid = *pid;
                let (mx, my, mw, mh) = get_modal_bounds(50, 35, term_width, term_height);

                // Click outside modal -> dismiss
                if x < mx || x >= mx + mw || y < my || y >= my + mh {
                    app.active_modal = Modal::None;
                    return true;
                }

                // Click on button line (near bottom of modal)
                if y >= my + mh.saturating_sub(3) && y < my + mh {
                    let rel_x = x.saturating_sub(mx);
                    let slot = rel_x * 3 / mw.max(1);
                    match slot {
                        0 => app.execute_kill(target_pid, false), // [Y] Terminate
                        1 => app.execute_kill(target_pid, true),  // [F] Force Kill
                        _ => app.active_modal = Modal::None,     // [Esc] Cancel
                    }
                    return true;
                }
                return false;
            }
            Modal::ProcessDetails(pid) => {
                let target_pid = *pid;
                let (mx, my, mw, mh) = get_modal_bounds(65, 60, term_width, term_height);

                // Click outside modal -> dismiss
                if x < mx || x >= mx + mw || y < my || y >= my + mh {
                    app.active_modal = Modal::None;
                    return true;
                }

                // Click on bottom action buttons
                if y >= my + mh.saturating_sub(3) && y < my + mh {
                    let rel_x = x.saturating_sub(mx);
                    if rel_x < mw / 2 {
                        // [k] Kill Process
                        if let Some(proc) = app.monitor.processes.iter().find(|p| p.pid == target_pid) {
                            app.active_modal = Modal::ConfirmKill {
                                pid: target_pid,
                                name: proc.name.clone(),
                                force: false,
                            };
                        }
                    } else {
                        // [Close]
                        app.active_modal = Modal::None;
                    }
                    return true;
                }
                return false;
            }
            Modal::Help => {
                let (mx, my, mw, mh) = get_modal_bounds(65, 70, term_width, term_height);
                if x < mx || x >= mx + mw || y < my || y >= my + mh || y >= my + mh.saturating_sub(3) {
                    app.active_modal = Modal::None;
                    return true;
                }
                return false;
            }
            Modal::None => {}
        }

        // Header Clicks (y in 0..3)
        if y < 3 {
            let brand_width = 24;
            let btn_width = 17;
            let host_width = 28;
            let btn_start = term_width.saturating_sub(btn_width + host_width);
            let host_start = term_width.saturating_sub(host_width);

            if x >= brand_width && x < btn_start {
                let tab_area_width = btn_start.saturating_sub(brand_width);
                let rel_x = x - brand_width;
                let tab_idx = (rel_x * 4 / tab_area_width.max(1)).min(3);
                match tab_idx {
                    0 => app.switch_tab(Tab::Processes),
                    1 => app.switch_tab(Tab::NetworkPorts),
                    2 => app.switch_tab(Tab::SystemDetails),
                    3 => app.switch_tab(Tab::Help),
                    _ => {}
                }
                return true;
            } else if x >= btn_start && x < host_start {
                // Clicked Play / Pause Button
                app.paused = !app.paused;
                let msg = if app.paused { "⏸ Paused Live Refresh" } else { "▶ Resumed Live Refresh" };
                app.add_toast(msg.to_string(), ToastKind::Info);
                return true;
            } else if x >= host_start {
                // Clicked Host block
                app.paused = !app.paused;
                let msg = if app.paused { "⏸ Paused Live Refresh" } else { "▶ Resumed Live Refresh" };
                app.add_toast(msg.to_string(), ToastKind::Info);
                return true;
            }
            return false;
        }

        // Dashboard Graphs / Ranking Area Clicks (y in 3..11)
        if (3..11).contains(&y) {
            // Click anywhere in dashboard to cycle view modes
            app.dashboard_view = app.dashboard_view.next();
            let view_name = match app.dashboard_view {
                DashboardView::Combined => "Dashboard: Combined (Graphs + Top Rank)",
                DashboardView::TopRank => "Dashboard: Top Process Rankings",
                DashboardView::GraphsOnly => "Dashboard: System Usage Graphs",
            };
            app.add_toast(view_name.to_string(), ToastKind::Info);
            return true;
        }

        // Search & Filter Bar Clicks (y in 11..14)
        if (11..14).contains(&y) {
            if app.active_tab == Tab::Processes {
                app.search_active = true;
                return true;
            } else if app.active_tab == Tab::NetworkPorts {
                app.net_search_active = true;
                return true;
            }
            return false;
        }

        // Table Header Clicks (y == 15) -> Instant Column Sorting
        if y == 15 {
            if app.active_tab == Tab::Processes {
                // Constraints: PID:8, Name:22, Cpu:9, Mem:11, Mem%:8, Ports:16, Status:9, User:18
                if x < 11 {
                    app.set_sort(SortColumn::Pid);
                } else if x < 33 {
                    app.set_sort(SortColumn::Name);
                } else if x < 42 {
                    app.set_sort(SortColumn::Cpu);
                } else if x < 61 {
                    app.set_sort(SortColumn::Memory);
                } else if x < 77 {
                    app.set_sort(SortColumn::Port);
                } else if x < 86 {
                    app.set_sort(SortColumn::Status);
                } else {
                    app.set_sort(SortColumn::User);
                }
                return true;
            } else if app.active_tab == Tab::NetworkPorts {
                // Constraints: Port:10, Proto:8, State:14, LocalAddr:24, RemoteAddr:24, PID:12, Name:20
                if x < 13 {
                    app.set_net_sort(NetworkSortColumn::Port);
                } else if x < 21 {
                    app.set_net_sort(NetworkSortColumn::Protocol);
                } else if x < 35 {
                    app.set_net_sort(NetworkSortColumn::State);
                } else if (83..95).contains(&x) {
                    app.set_net_sort(NetworkSortColumn::Pid);
                } else if x >= 95 {
                    app.set_net_sort(NetworkSortColumn::Name);
                }
                return true;
            }
            return false;
        }

        // Table Row Clicks (y in 16..(height - 4))
        if y >= 16 && y < term_height.saturating_sub(4) {
            let row_offset = (y - 16) as usize;
            match app.active_tab {
                Tab::Processes => {
                    let target_idx = app.process_table_state.offset() + row_offset;
                    if target_idx < app.filtered_processes.len() {
                        if app.selected_proc_idx == target_idx {
                            // Clicked already selected process -> open details inspector!
                            if let Some(proc) = app.filtered_processes.get(target_idx).and_then(|&idx| app.monitor.processes.get(idx)) {
                                app.active_modal = Modal::ProcessDetails(proc.pid);
                            }
                        } else {
                            app.selected_proc_idx = target_idx;
                            app.process_table_state.select(Some(target_idx));
                        }
                        return true;
                    }
                }
                Tab::NetworkPorts => {
                    let target_idx = app.network_table_state.offset() + row_offset;
                    if target_idx < app.filtered_sockets.len() {
                        if app.selected_net_idx == target_idx {
                            if let Some(&pid) = app.filtered_sockets.get(target_idx).and_then(|&idx| app.monitor.sockets.get(idx)).and_then(|s| s.pids.first()) {
                                app.active_modal = Modal::ProcessDetails(pid);
                            }
                        } else {
                            app.selected_net_idx = target_idx;
                            app.network_table_state.select(Some(target_idx));
                        }
                        return true;
                    }
                }
                _ => {}
            }
            return false;
        }

        // Footer Clicks (y >= height - 3)
        if y >= term_height.saturating_sub(3) {
            if x < 12 {
                app.switch_tab(app.active_tab.next());
                return true;
            } else if x >= term_width.saturating_sub(12) {
                app.should_quit = true;
                return true;
            } else if x >= term_width.saturating_sub(24) {
                app.active_modal = Modal::Help;
                return true;
            }
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

    #[test]
    fn test_mouse_scroll_triggers_redraw() {
        let mut app = App::new();
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(handle_mouse_event(&mut app, scroll_down));

        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(handle_mouse_event(&mut app, scroll_up));
    }

    #[test]
    fn test_mouse_move_ignored() {
        let mut app = App::new();
        let mouse_move = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!handle_mouse_event(&mut app, mouse_move));
    }

    #[test]
    fn test_mouse_click_header_and_dashboard() {
        let mut app = App::new();
        // Click dashboard area (y=5) -> should cycle view and return true
        let dash_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 20,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert!(handle_mouse_event(&mut app, dash_click));
        assert_eq!(app.dashboard_view, DashboardView::TopRank);
    }

    #[test]
    fn test_key_tab_navigation() {
        let mut app = App::new();
        assert_eq!(app.active_tab, Tab::Processes);
        handle_key_event(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.active_tab, Tab::NetworkPorts);
    }

    #[test]
    fn test_key_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        handle_key_event(&mut app, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);
    }
}

