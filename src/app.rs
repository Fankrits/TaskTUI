use crate::system::{NetworkSocketItem, ProcessInfo, SystemMonitor};
use crate::theme::Theme;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Processes = 0,
    NetworkPorts = 1,
    SystemDetails = 2,
    Help = 3,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Processes => Tab::NetworkPorts,
            Tab::NetworkPorts => Tab::SystemDetails,
            Tab::SystemDetails => Tab::Help,
            Tab::Help => Tab::Processes,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Processes => Tab::Help,
            Tab::NetworkPorts => Tab::Processes,
            Tab::SystemDetails => Tab::NetworkPorts,
            Tab::Help => Tab::SystemDetails,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardView {
    Combined,
    TopRank,
    GraphsOnly,
}

impl DashboardView {
    pub fn next(&self) -> Self {
        match self {
            DashboardView::Combined => DashboardView::TopRank,
            DashboardView::TopRank => DashboardView::GraphsOnly,
            DashboardView::GraphsOnly => DashboardView::Combined,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Pid,
    Name,
    Cpu,
    Memory,
    Port,
    User,
    Status,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            SortDirection::Ascending => "Low → High (Ascending)",
            SortDirection::Descending => "High → Low (Descending)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkSortColumn {
    Port,
    Protocol,
    State,
    Pid,
    Name,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Modal {
    None,
    ConfirmKill { pid: u32, name: String, force: bool },
    ProcessDetails(u32),
    Help,
}

#[derive(Clone, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub timestamp: Instant,
    pub duration: Duration,
}

pub struct App {
    pub monitor: SystemMonitor,
    pub theme: Theme,
    pub active_tab: Tab,

    // Process Table State
    pub process_table_state: TableState,
    pub selected_proc_idx: usize,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub search_query: String,
    pub search_active: bool,
    pub filtered_processes: Vec<ProcessInfo>,

    // Network Table State
    pub network_table_state: TableState,
    pub selected_net_idx: usize,
    pub net_sort_column: NetworkSortColumn,
    pub net_sort_direction: SortDirection,
    pub net_search_query: String,
    pub net_search_active: bool,
    pub filtered_sockets: Vec<NetworkSocketItem>,
    pub listening_only: bool,

    // State & Modals
    pub active_modal: Modal,
    pub dashboard_view: DashboardView,
    pub toasts: Vec<Toast>,
    pub paused: bool,
    pub tick_rate: Duration,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let monitor = SystemMonitor::new();
        let mut app = Self {
            monitor,
            theme: Theme::default(),
            active_tab: Tab::Processes,

            process_table_state: TableState::default(),
            selected_proc_idx: 0,
            sort_column: SortColumn::Memory,
            sort_direction: SortDirection::Descending,
            search_query: String::new(),
            search_active: false,
            filtered_processes: Vec::new(),

            network_table_state: TableState::default(),
            selected_net_idx: 0,
            net_sort_column: NetworkSortColumn::Port,
            net_sort_direction: SortDirection::Ascending,
            net_search_query: String::new(),
            net_search_active: false,
            filtered_sockets: Vec::new(),
            listening_only: false,

            active_modal: Modal::None,
            dashboard_view: DashboardView::Combined,
            toasts: Vec::new(),
            paused: false,
            tick_rate: Duration::from_millis(1000),
            should_quit: false,
        };

        app.apply_process_filter_and_sort();
        app.apply_network_filter_and_sort();
        if !app.filtered_processes.is_empty() {
            app.process_table_state.select(Some(0));
        }
        if !app.filtered_sockets.is_empty() {
            app.network_table_state.select(Some(0));
        }
        app
    }

    pub fn switch_tab(&mut self, tab: Tab) {
        let previous = self.active_tab;
        self.active_tab = tab;
        if self.active_tab == Tab::NetworkPorts && previous != Tab::NetworkPorts {
            self.monitor.refresh_sockets();
            self.apply_network_filter_and_sort();
        }
    }

    pub fn on_tick(&mut self) {
        if !self.paused {
            let prev_socket_refresh = self.monitor.last_socket_refresh;
            self.monitor.refresh();
            self.apply_process_filter_and_sort();
            if self.monitor.last_socket_refresh != prev_socket_refresh {
                self.apply_network_filter_and_sort();
            }
        }

        // Clean up expired toasts (older than 4 seconds)
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.timestamp) < t.duration);
    }

    pub fn add_toast(&mut self, message: String, kind: ToastKind) {
        self.toasts.push(Toast {
            message,
            kind,
            timestamp: Instant::now(),
            duration: Duration::from_secs(4),
        });
    }

    pub fn apply_process_filter_and_sort(&mut self) {
        let query = self.search_query.to_lowercase().trim().to_string();

        let mut list: Vec<ProcessInfo> = self
            .monitor
            .processes
            .iter()
            .filter(|p| {
                if query.is_empty() {
                    return true;
                }
                p.name.to_lowercase().contains(&query)
                    || p.pid.to_string().contains(&query)
                    || p.user.to_lowercase().contains(&query)
                    || p.ports.iter().any(|port| port.to_string().contains(&query))
                    || p.cmd.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();

        // Sort
        let dir = self.sort_direction;
        match self.sort_column {
            SortColumn::Pid => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.pid.cmp(&b.pid),
                    SortDirection::Descending => b.pid.cmp(&a.pid),
                });
            }
            SortColumn::Name => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    SortDirection::Descending => b.name.to_lowercase().cmp(&a.name.to_lowercase()),
                });
            }
            SortColumn::Cpu => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
                    SortDirection::Descending => b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
                });
            }
            SortColumn::Memory => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.memory_bytes.cmp(&b.memory_bytes),
                    SortDirection::Descending => b.memory_bytes.cmp(&a.memory_bytes),
                });
            }
            SortColumn::Port => {
                list.sort_by(|a, b| {
                    let a_port = a.ports.first().copied().unwrap_or(0);
                    let b_port = b.ports.first().copied().unwrap_or(0);
                    match dir {
                        SortDirection::Ascending => a_port.cmp(&b_port),
                        SortDirection::Descending => b_port.cmp(&a_port),
                    }
                });
            }
            SortColumn::User => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
                    SortDirection::Descending => b.user.to_lowercase().cmp(&a.user.to_lowercase()),
                });
            }
            SortColumn::Status => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.status.cmp(&b.status),
                    SortDirection::Descending => b.status.cmp(&a.status),
                });
            }
        }

        self.filtered_processes = list;

        if self.filtered_processes.is_empty() {
            self.selected_proc_idx = 0;
            self.process_table_state.select(None);
        } else {
            if self.selected_proc_idx >= self.filtered_processes.len() {
                self.selected_proc_idx = self.filtered_processes.len().saturating_sub(1);
            }
            self.process_table_state.select(Some(self.selected_proc_idx));
        }
    }

    pub fn apply_network_filter_and_sort(&mut self) {
        let query = self.net_search_query.to_lowercase().trim().to_string();
        let listening_only = self.listening_only;

        let mut list: Vec<NetworkSocketItem> = self
            .monitor
            .sockets
            .iter()
            .filter(|s| {
                if listening_only && s.state != "LISTEN" {
                    return false;
                }
                if query.is_empty() {
                    return true;
                }
                s.local_port.to_string().contains(&query)
                    || s.protocol.to_lowercase().contains(&query)
                    || s.state.to_lowercase().contains(&query)
                    || s.local_addr.to_lowercase().contains(&query)
                    || s.remote_addr.to_lowercase().contains(&query)
                    || s.pids.iter().any(|p| p.to_string().contains(&query))
                    || s.process_names.iter().any(|n| n.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        let dir = self.net_sort_direction;
        match self.net_sort_column {
            NetworkSortColumn::Port => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.local_port.cmp(&b.local_port),
                    SortDirection::Descending => b.local_port.cmp(&a.local_port),
                });
            }
            NetworkSortColumn::Protocol => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.protocol.cmp(&b.protocol),
                    SortDirection::Descending => b.protocol.cmp(&a.protocol),
                });
            }
            NetworkSortColumn::State => {
                list.sort_by(|a, b| match dir {
                    SortDirection::Ascending => a.state.cmp(&b.state),
                    SortDirection::Descending => b.state.cmp(&a.state),
                });
            }
            NetworkSortColumn::Pid => {
                list.sort_by(|a, b| {
                    let a_pid = a.pids.first().copied().unwrap_or(0);
                    let b_pid = b.pids.first().copied().unwrap_or(0);
                    match dir {
                        SortDirection::Ascending => a_pid.cmp(&b_pid),
                        SortDirection::Descending => b_pid.cmp(&a_pid),
                    }
                });
            }
            NetworkSortColumn::Name => {
                list.sort_by(|a, b| {
                    let a_name = a.process_names.first().cloned().unwrap_or_default();
                    let b_name = b.process_names.first().cloned().unwrap_or_default();
                    match dir {
                        SortDirection::Ascending => a_name.cmp(&b_name),
                        SortDirection::Descending => b_name.cmp(&a_name),
                    }
                });
            }
        }

        self.filtered_sockets = list;

        if self.filtered_sockets.is_empty() {
            self.selected_net_idx = 0;
            self.network_table_state.select(None);
        } else {
            if self.selected_net_idx >= self.filtered_sockets.len() {
                self.selected_net_idx = self.filtered_sockets.len().saturating_sub(1);
            }
            self.network_table_state.select(Some(self.selected_net_idx));
        }
    }

    pub fn next_row(&mut self) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                let i = if self.selected_proc_idx + 1 < self.filtered_processes.len() {
                    self.selected_proc_idx + 1
                } else {
                    0
                };
                self.selected_proc_idx = i;
                self.process_table_state.select(Some(i));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                let i = if self.selected_net_idx + 1 < self.filtered_sockets.len() {
                    self.selected_net_idx + 1
                } else {
                    0
                };
                self.selected_net_idx = i;
                self.network_table_state.select(Some(i));
            }
            _ => {}
        }
    }

    pub fn prev_row(&mut self) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                let i = if self.selected_proc_idx > 0 {
                    self.selected_proc_idx - 1
                } else {
                    self.filtered_processes.len().saturating_sub(1)
                };
                self.selected_proc_idx = i;
                self.process_table_state.select(Some(i));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                let i = if self.selected_net_idx > 0 {
                    self.selected_net_idx - 1
                } else {
                    self.filtered_sockets.len().saturating_sub(1)
                };
                self.selected_net_idx = i;
                self.network_table_state.select(Some(i));
            }
            _ => {}
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                let i = (self.selected_proc_idx + page_size).min(self.filtered_processes.len() - 1);
                self.selected_proc_idx = i;
                self.process_table_state.select(Some(i));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                let i = (self.selected_net_idx + page_size).min(self.filtered_sockets.len() - 1);
                self.selected_net_idx = i;
                self.network_table_state.select(Some(i));
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                let i = self.selected_proc_idx.saturating_sub(page_size);
                self.selected_proc_idx = i;
                self.process_table_state.select(Some(i));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                let i = self.selected_net_idx.saturating_sub(page_size);
                self.selected_net_idx = i;
                self.network_table_state.select(Some(i));
            }
            _ => {}
        }
    }

    pub fn select_first(&mut self) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                self.selected_proc_idx = 0;
                self.process_table_state.select(Some(0));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                self.selected_net_idx = 0;
                self.network_table_state.select(Some(0));
            }
            _ => {}
        }
    }

    pub fn select_last(&mut self) {
        match self.active_tab {
            Tab::Processes if !self.filtered_processes.is_empty() => {
                let i = self.filtered_processes.len() - 1;
                self.selected_proc_idx = i;
                self.process_table_state.select(Some(i));
            }
            Tab::NetworkPorts if !self.filtered_sockets.is_empty() => {
                let i = self.filtered_sockets.len() - 1;
                self.selected_net_idx = i;
                self.network_table_state.select(Some(i));
            }
            _ => {}
        }
    }

    pub fn get_selected_process(&self) -> Option<&ProcessInfo> {
        self.filtered_processes.get(self.selected_proc_idx)
    }

    pub fn get_selected_socket(&self) -> Option<&NetworkSocketItem> {
        self.filtered_sockets.get(self.selected_net_idx)
    }

    pub fn prompt_kill_selected(&mut self, force: bool) {
        if self.active_tab == Tab::Processes {
            if let Some(proc) = self.get_selected_process() {
                self.active_modal = Modal::ConfirmKill {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    force,
                };
            }
        } else if self.active_tab == Tab::NetworkPorts
            && let Some(sock) = self.get_selected_socket()
        {
            match sock.pids.first() {
                Some(&pid) => {
                    let name = sock.process_names.first().cloned().unwrap_or_else(|| "Unknown".to_string());
                    self.active_modal = Modal::ConfirmKill {
                        pid,
                        name,
                        force,
                    };
                }
                None => {
                    self.add_toast("No PID associated with this port socket".to_string(), ToastKind::Error);
                }
            }
        }
    }

    pub fn execute_kill(&mut self, pid: u32, force: bool) {
        match self.monitor.kill_process(pid, force) {
            Ok(()) => {
                self.add_toast(
                    format!("Successfully terminated PID {} {}", pid, if force { "(Force)" } else { "" }),
                    ToastKind::Success,
                );
                self.apply_process_filter_and_sort();
                self.apply_network_filter_and_sort();
            }
            Err(e) => {
                self.add_toast(format!("Failed to kill PID {}: {}", pid, e), ToastKind::Error);
            }
        }
        self.active_modal = Modal::None;
    }

    pub fn get_sort_summary(&self) -> String {
        let col_name = match self.sort_column {
            SortColumn::Memory => "Memory",
            SortColumn::Cpu => "CPU %",
            SortColumn::Pid => "PID",
            SortColumn::Name => "Name",
            SortColumn::Port => "Port",
            SortColumn::User => "User",
            SortColumn::Status => "Status",
        };
        let dir_str = match self.sort_direction {
            SortDirection::Descending => "High → Low ▼",
            SortDirection::Ascending => "Low → High ▲",
        };
        format!("{} ({})", col_name, dir_str)
    }

    pub fn get_top_cpu_processes(&self, count: usize) -> Vec<&ProcessInfo> {
        let mut list: Vec<&ProcessInfo> = self.monitor.processes.iter().collect();
        if list.len() > count {
            list.select_nth_unstable_by(count, |a, b| {
                b.cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            list.truncate(count);
        }
        list.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    }

    pub fn get_top_memory_processes(&self, count: usize) -> Vec<&ProcessInfo> {
        let mut list: Vec<&ProcessInfo> = self.monitor.processes.iter().collect();
        if list.len() > count {
            list.select_nth_unstable_by(count, |a, b| b.memory_bytes.cmp(&a.memory_bytes));
            list.truncate(count);
        }
        list.sort_by_key(|b| std::cmp::Reverse(b.memory_bytes));
        list
    }

    pub fn set_sort(&mut self, col: SortColumn) {
        if self.sort_column == col {
            self.sort_direction = self.sort_direction.toggle();
        } else {
            self.sort_column = col;
            self.sort_direction = match col {
                SortColumn::Name | SortColumn::User | SortColumn::Status | SortColumn::Pid | SortColumn::Port => SortDirection::Ascending,
                SortColumn::Cpu | SortColumn::Memory => SortDirection::Descending,
            };
        }
        self.apply_process_filter_and_sort();
        let summary = self.get_sort_summary();
        self.add_toast(format!("Sorted by: {}", summary), ToastKind::Info);
    }

    pub fn toggle_sort_direction(&mut self) {
        self.sort_direction = self.sort_direction.toggle();
        self.apply_process_filter_and_sort();
        let summary = self.get_sort_summary();
        self.add_toast(format!("Sort direction: {}", summary), ToastKind::Info);
    }

    pub fn set_net_sort(&mut self, col: NetworkSortColumn) {
        if self.net_sort_column == col {
            self.net_sort_direction = self.net_sort_direction.toggle();
        } else {
            self.net_sort_column = col;
            self.net_sort_direction = SortDirection::Ascending;
        }
        self.apply_network_filter_and_sort();
        let col_name = match self.net_sort_column {
            NetworkSortColumn::Port => "Port",
            NetworkSortColumn::Protocol => "Protocol",
            NetworkSortColumn::State => "State",
            NetworkSortColumn::Pid => "PID",
            NetworkSortColumn::Name => "Name",
        };
        let dir_str = match self.net_sort_direction {
            SortDirection::Descending => "High → Low ▼",
            SortDirection::Ascending => "Low → High ▲",
        };
        self.add_toast(format!("Ports sorted by: {} ({})", col_name, dir_str), ToastKind::Info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_process(pid: u32, name: &str, cpu_usage: f32, memory_bytes: u64) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.to_string(),
            cpu_usage,
            memory_bytes,
            memory_percent: 0.0,
            virtual_memory_bytes: 0,
            status: "Running".to_string(),
            user: "test_user".to_string(),
            session_id: None,
            ports: Vec::new(),
            exe_path: String::new(),
            cmd: String::new(),
            start_time: 0,
            run_time_secs: 0,
            disk_read_bytes: 0,
            disk_written_bytes: 0,
        }
    }

    #[test]
    fn test_get_top_cpu_processes_ordering_and_limit() {
        let mut app = App::new();
        app.monitor.processes = vec![
            create_test_process(1, "proc1", 10.5, 1000),
            create_test_process(2, "proc2", 85.0, 2000),
            create_test_process(3, "proc3", 2.0, 3000),
            create_test_process(4, "proc4", 99.1, 4000),
            create_test_process(5, "proc5", 45.3, 5000),
            create_test_process(6, "proc6", 0.0, 6000),
        ];

        let top = app.get_top_cpu_processes(3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].pid, 4);
        assert_eq!(top[0].cpu_usage, 99.1);
        assert_eq!(top[1].pid, 2);
        assert_eq!(top[1].cpu_usage, 85.0);
        assert_eq!(top[2].pid, 5);
        assert_eq!(top[2].cpu_usage, 45.3);

        // Verify returning all when count > len
        let top_all = app.get_top_cpu_processes(10);
        assert_eq!(top_all.len(), 6);
        assert_eq!(top_all[0].pid, 4);
        assert_eq!(top_all[5].pid, 6);

        // Verify count = 0
        let top_none = app.get_top_cpu_processes(0);
        assert_eq!(top_none.len(), 0);
    }

    #[test]
    fn test_get_top_memory_processes_ordering_and_limit() {
        let mut app = App::new();
        app.monitor.processes = vec![
            create_test_process(1, "proc1", 10.0, 10_000_000),
            create_test_process(2, "proc2", 20.0, 85_000_000),
            create_test_process(3, "proc3", 30.0, 2_000_000),
            create_test_process(4, "proc4", 40.0, 99_000_000),
            create_test_process(5, "proc5", 50.0, 45_000_000),
            create_test_process(6, "proc6", 60.0, 500_000),
        ];

        let top = app.get_top_memory_processes(3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].pid, 4);
        assert_eq!(top[0].memory_bytes, 99_000_000);
        assert_eq!(top[1].pid, 2);
        assert_eq!(top[1].memory_bytes, 85_000_000);
        assert_eq!(top[2].pid, 5);
        assert_eq!(top[2].memory_bytes, 45_000_000);

        // Verify returning all when count > len
        let top_all = app.get_top_memory_processes(10);
        assert_eq!(top_all.len(), 6);
        assert_eq!(top_all[0].pid, 4);
        assert_eq!(top_all[5].pid, 6);

        // Verify count = 0
        let top_none = app.get_top_memory_processes(0);
        assert_eq!(top_none.len(), 0);
    }

    #[test]
    fn test_get_top_processes_empty_list() {
        let mut app = App::new();
        app.monitor.processes.clear();

        let top_cpu = app.get_top_cpu_processes(5);
        assert!(top_cpu.is_empty());

        let top_mem = app.get_top_memory_processes(5);
        assert!(top_mem.is_empty());
    }

    #[test]
    fn test_switch_tab_to_network_ports_triggers_socket_refresh() {
        let mut app = App::new();
        let initial_refresh = app.monitor.last_socket_refresh;

        std::thread::sleep(Duration::from_millis(10));
        app.switch_tab(Tab::NetworkPorts);

        assert_eq!(app.active_tab, Tab::NetworkPorts);
        assert!(app.monitor.last_socket_refresh > initial_refresh);
    }

    #[test]
    fn test_switch_tab_to_other_tabs_does_not_force_socket_refresh() {
        let mut app = App::new();
        app.switch_tab(Tab::SystemDetails);
        let refresh_after = app.monitor.last_socket_refresh;

        app.switch_tab(Tab::Help);
        assert_eq!(app.active_tab, Tab::Help);
        assert_eq!(app.monitor.last_socket_refresh, refresh_after);
    }

    #[test]
    fn test_on_tick_throttles_socket_refresh() {
        let mut app = App::new();
        let initial_refresh = app.monitor.last_socket_refresh;

        app.on_tick();
        assert_eq!(app.monitor.last_socket_refresh, initial_refresh);
    }
}
