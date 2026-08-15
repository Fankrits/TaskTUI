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
    pub filtered_processes: Vec<usize>,
    pub top_cpu_indices: Vec<usize>,
    pub top_mem_indices: Vec<usize>,

    // Network Table State
    pub network_table_state: TableState,
    pub selected_net_idx: usize,
    pub net_sort_column: NetworkSortColumn,
    pub net_sort_direction: SortDirection,
    pub net_search_query: String,
    pub net_search_active: bool,
    pub filtered_sockets: Vec<usize>,
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
            top_cpu_indices: Vec::new(),
            top_mem_indices: Vec::new(),

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
        app.update_top_rankings();
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
            self.update_top_rankings();
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
        self.filtered_processes.clear();

        for (idx, p) in self.monitor.processes.iter().enumerate() {
            if query.is_empty()
                || p.name.to_lowercase().contains(&query)
                || p.pid.to_string().contains(&query)
                || p.user.to_lowercase().contains(&query)
                || p.ports.iter().any(|port| port.to_string().contains(&query))
                || p.cmd.to_lowercase().contains(&query)
            {
                self.filtered_processes.push(idx);
            }
        }

        let procs = &self.monitor.processes;
        let dir = self.sort_direction;
        match self.sort_column {
            SortColumn::Pid => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].pid.cmp(&procs[b].pid),
                SortDirection::Descending => procs[b].pid.cmp(&procs[a].pid),
            }),
            SortColumn::Name => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].name.to_lowercase().cmp(&procs[b].name.to_lowercase()),
                SortDirection::Descending => procs[b].name.to_lowercase().cmp(&procs[a].name.to_lowercase()),
            }),
            SortColumn::Cpu => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].cpu_usage.partial_cmp(&procs[b].cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
                SortDirection::Descending => procs[b].cpu_usage.partial_cmp(&procs[a].cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
            }),
            SortColumn::Memory => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].memory_bytes.cmp(&procs[b].memory_bytes),
                SortDirection::Descending => procs[b].memory_bytes.cmp(&procs[a].memory_bytes),
            }),
            SortColumn::Port => self.filtered_processes.sort_by(|&a, &b| {
                let a_port = procs[a].ports.first().copied().unwrap_or(0);
                let b_port = procs[b].ports.first().copied().unwrap_or(0);
                match dir {
                    SortDirection::Ascending => a_port.cmp(&b_port),
                    SortDirection::Descending => b_port.cmp(&a_port),
                }
            }),
            SortColumn::User => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].user.to_lowercase().cmp(&procs[b].user.to_lowercase()),
                SortDirection::Descending => procs[b].user.to_lowercase().cmp(&procs[a].user.to_lowercase()),
            }),
            SortColumn::Status => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].status.cmp(procs[b].status),
                SortDirection::Descending => procs[b].status.cmp(procs[a].status),
            }),
        }

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
        self.filtered_sockets.clear();

        for (idx, s) in self.monitor.sockets.iter().enumerate() {
            if listening_only && s.state != "LISTEN" {
                continue;
            }
            if query.is_empty()
                || s.local_port.to_string().contains(&query)
                || s.protocol.to_lowercase().contains(&query)
                || s.state.to_lowercase().contains(&query)
                || s.local_addr.to_lowercase().contains(&query)
                || s.remote_addr.to_lowercase().contains(&query)
                || s.pids.iter().any(|p| p.to_string().contains(&query))
                || s.process_names.iter().any(|n| n.to_lowercase().contains(&query))
            {
                self.filtered_sockets.push(idx);
            }
        }

        let socks = &self.monitor.sockets;
        let dir = self.net_sort_direction;
        match self.net_sort_column {
            NetworkSortColumn::Port => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].local_port.cmp(&socks[b].local_port),
                SortDirection::Descending => socks[b].local_port.cmp(&socks[a].local_port),
            }),
            NetworkSortColumn::Protocol => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].protocol.cmp(&socks[b].protocol),
                SortDirection::Descending => socks[b].protocol.cmp(&socks[a].protocol),
            }),
            NetworkSortColumn::State => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].state.cmp(&socks[b].state),
                SortDirection::Descending => socks[b].state.cmp(&socks[a].state),
            }),
            NetworkSortColumn::Pid => self.filtered_sockets.sort_by(|&a, &b| {
                let a_pid = socks[a].pids.first().copied().unwrap_or(0);
                let b_pid = socks[b].pids.first().copied().unwrap_or(0);
                match dir {
                    SortDirection::Ascending => a_pid.cmp(&b_pid),
                    SortDirection::Descending => b_pid.cmp(&a_pid),
                }
            }),
            NetworkSortColumn::Name => self.filtered_sockets.sort_by(|&a, &b| {
                let a_name = socks[a].process_names.first().map(|s| s.as_str()).unwrap_or("");
                let b_name = socks[b].process_names.first().map(|s| s.as_str()).unwrap_or("");
                match dir {
                    SortDirection::Ascending => a_name.cmp(b_name),
                    SortDirection::Descending => b_name.cmp(a_name),
                }
            }),
        }

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

    pub fn update_top_rankings(&mut self) {
        let mut cpu_indices: Vec<usize> = (0..self.monitor.processes.len()).collect();
        if cpu_indices.len() > 5 {
            cpu_indices.select_nth_unstable_by(5, |&a, &b| {
                self.monitor.processes[b].cpu_usage
                    .partial_cmp(&self.monitor.processes[a].cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            cpu_indices.truncate(5);
        }
        cpu_indices.sort_by(|&a, &b| {
            self.monitor.processes[b].cpu_usage
                .partial_cmp(&self.monitor.processes[a].cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.top_cpu_indices = cpu_indices;

        let mut mem_indices: Vec<usize> = (0..self.monitor.processes.len()).collect();
        if mem_indices.len() > 5 {
            mem_indices.select_nth_unstable_by(5, |&a, &b| {
                self.monitor.processes[b].memory_bytes.cmp(&self.monitor.processes[a].memory_bytes)
            });
            mem_indices.truncate(5);
        }
        mem_indices.sort_by_key(|&b| std::cmp::Reverse(self.monitor.processes[b].memory_bytes));
        self.top_mem_indices = mem_indices;
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
        self.filtered_processes.get(self.selected_proc_idx).and_then(|&idx| self.monitor.processes.get(idx))
    }

    pub fn get_selected_socket(&self) -> Option<&NetworkSocketItem> {
        self.filtered_sockets.get(self.selected_net_idx).and_then(|&idx| self.monitor.sockets.get(idx))
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
                self.update_top_rankings();
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
            status: "Run",
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

    #[test]
    fn test_index_based_filter_and_sort() {
        let mut app = App::new();
        app.monitor.processes = vec![
            create_test_process(1, "alpha", 10.0, 500),
            create_test_process(2, "beta", 50.0, 100),
            create_test_process(3, "gamma", 20.0, 900),
        ];
        app.search_query = "beta".to_string();
        app.apply_process_filter_and_sort();

        assert_eq!(app.filtered_processes.len(), 1);
        assert_eq!(app.get_selected_process().unwrap().name, "beta");
    }

    #[test]
    fn test_update_top_rankings() {
        let mut app = App::new();
        app.monitor.processes = vec![
            create_test_process(1, "p1", 10.0, 500),
            create_test_process(2, "p2", 80.0, 100),
            create_test_process(3, "p3", 30.0, 900),
            create_test_process(4, "p4", 95.0, 200),
            create_test_process(5, "p5", 5.0, 1000),
            create_test_process(6, "p6", 50.0, 50),
        ];
        app.update_top_rankings();

        assert_eq!(app.top_cpu_indices.len(), 5);
        // top cpu: p4 (95.0, idx 3), p2 (80.0, idx 1), p6 (50.0, idx 5), p3 (30.0, idx 2), p1 (10.0, idx 0)
        assert_eq!(app.top_cpu_indices[0], 3);
        assert_eq!(app.top_cpu_indices[1], 1);
        assert_eq!(app.top_cpu_indices[2], 5);
        assert_eq!(app.top_cpu_indices[3], 2);
        assert_eq!(app.top_cpu_indices[4], 0);

        assert_eq!(app.top_mem_indices.len(), 5);
        // top mem: p5 (1000, idx 4), p3 (900, idx 2), p1 (500, idx 0), p4 (200, idx 3), p2 (100, idx 1)
        assert_eq!(app.top_mem_indices[0], 4);
        assert_eq!(app.top_mem_indices[1], 2);
        assert_eq!(app.top_mem_indices[2], 0);
        assert_eq!(app.top_mem_indices[3], 3);
        assert_eq!(app.top_mem_indices[4], 1);
    }

    #[test]
    fn test_index_based_network_filter_and_sort() {
        let mut app = App::new();
        app.monitor.sockets = vec![
            NetworkSocketItem {
                protocol: "TCP".to_string(),
                local_addr: "127.0.0.1".to_string(),
                local_port: 8080,
                remote_addr: "0.0.0.0".to_string(),
                state: "LISTEN".to_string(),
                pids: vec![101],
                process_names: vec!["web".to_string()],
            },
            NetworkSocketItem {
                protocol: "TCP".to_string(),
                local_addr: "127.0.0.1".to_string(),
                local_port: 3000,
                remote_addr: "0.0.0.0".to_string(),
                state: "LISTEN".to_string(),
                pids: vec![102],
                process_names: vec!["node".to_string()],
            },
            NetworkSocketItem {
                protocol: "UDP".to_string(),
                local_addr: "0.0.0.0".to_string(),
                local_port: 53,
                remote_addr: "0.0.0.0".to_string(),
                state: "NONE".to_string(),
                pids: vec![103],
                process_names: vec!["dns".to_string()],
            },
        ];

        app.net_search_query = "node".to_string();
        app.apply_network_filter_and_sort();
        assert_eq!(app.filtered_sockets.len(), 1);
        assert_eq!(app.get_selected_socket().unwrap().local_port, 3000);

        app.net_search_query = String::new();
        app.listening_only = true;
        app.apply_network_filter_and_sort();
        assert_eq!(app.filtered_sockets.len(), 2);
    }
}
