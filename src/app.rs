use crate::system::{NetworkSocketItem, ProcessInfo, SystemMonitor};
use crate::theme::Theme;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

/// Case-insensitive substring test that allocates nothing.
///
/// `needle` must already be lowercase. Matching is ASCII-case-insensitive,
/// which is what the previous `to_lowercase().contains()` did for the ASCII
/// text (ports, PIDs, addresses, process names) this is used on.
pub fn contains_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return false;
    }
    h.windows(n.len())
        .any(|w| w.iter().zip(n).all(|(a, b)| a.to_ascii_lowercase() == *b))
}

/// Substring test against a number's decimal form without allocating a String.
pub fn number_contains(value: u64, needle: &str) -> bool {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let mut v = value;
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    // Digits are ASCII, so a plain substring search is enough.
    let s = std::str::from_utf8(&buf[i..]).unwrap_or("");
    s.contains(needle)
}

/// Compute the visible slice of a scrolling list, keeping `selected` on screen.
///
/// Returns the row range to actually build widgets for, so a 600-row table only
/// materialises the ~40 rows the terminal can display.
pub fn visible_window(
    total: usize,
    selected: usize,
    visible: usize,
    offset: &mut usize,
) -> std::ops::Range<usize> {
    if total == 0 || visible == 0 {
        *offset = 0;
        return 0..0;
    }
    let max_offset = total.saturating_sub(visible);
    if selected < *offset {
        *offset = selected;
    } else if selected >= *offset + visible {
        *offset = selected + 1 - visible;
    }
    if *offset > max_offset {
        *offset = max_offset;
    }
    let start = *offset;
    start..(start + visible).min(total)
}

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
    /// First visible row of the process table; owned here so both the renderer
    /// and mouse hit-testing agree on what is on screen.
    pub proc_view_offset: usize,

    // Network Table State
    pub network_table_state: TableState,
    pub selected_net_idx: usize,
    pub net_sort_column: NetworkSortColumn,
    pub net_sort_direction: SortDirection,
    pub net_search_query: String,
    pub net_search_active: bool,
    pub filtered_sockets: Vec<usize>,
    pub listening_only: bool,
    /// First visible row of the network table.
    pub net_view_offset: usize,

    // State & Modals
    pub active_modal: Modal,
    pub dashboard_view: DashboardView,
    pub toasts: Vec<Toast>,
    pub paused: bool,
    pub tick_rate: Duration,
    pub should_quit: bool,

    /// Reusable scratch buffers so the per-tick hot paths allocate nothing.
    rank_scratch: Vec<usize>,
    query_scratch: String,
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
            proc_view_offset: 0,

            network_table_state: TableState::default(),
            selected_net_idx: 0,
            net_sort_column: NetworkSortColumn::Port,
            net_sort_direction: SortDirection::Ascending,
            net_search_query: String::new(),
            net_search_active: false,
            filtered_sockets: Vec::new(),
            listening_only: false,
            net_view_offset: 0,

            active_modal: Modal::None,
            dashboard_view: DashboardView::Combined,
            toasts: Vec::new(),
            paused: false,
            tick_rate: Duration::from_millis(1000),
            should_quit: false,
            rank_scratch: Vec::new(),
            query_scratch: String::new(),
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
        if previous == tab {
            return;
        }
        self.active_tab = tab;

        // The Help tab shows no live data, so ticks skip refreshing entirely
        // while it is open. Catch up immediately on the way out.
        if previous == Tab::Help && !self.paused {
            self.monitor.refresh();
            self.update_top_rankings();
        }

        match tab {
            Tab::NetworkPorts => {
                self.monitor.refresh_sockets();
                self.apply_network_filter_and_sort();
            }
            // Filtering/sorting is skipped on ticks while another tab is up,
            // so rebuild the view before showing the table again.
            Tab::Processes => self.apply_process_filter_and_sort(),
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        // Nothing on the Help tab is driven by live data, so skip the whole
        // sampling pass while it is open.
        if !self.paused && self.active_tab != Tab::Help {
            let prev_socket_refresh = self.monitor.last_socket_refresh;
            self.monitor.refresh();

            // Only the visible table needs re-filtering; the dashboard
            // rankings are drawn on every non-Help tab.
            if self.active_tab == Tab::Processes {
                self.apply_process_filter_and_sort();
            }
            self.update_top_rankings();

            if self.monitor.last_socket_refresh != prev_socket_refresh
                && self.active_tab == Tab::NetworkPorts
            {
                self.apply_network_filter_and_sort();
            }

            // Disk I/O is sampled only for the process on screen in the
            // inspector, not for every process on the system.
            if let Modal::ProcessDetails(pid) = self.active_modal {
                self.monitor.refresh_process_disk_usage(pid);
            }
        }

        // Clean up expired toasts (older than 4 seconds)
        let now = Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.timestamp) < t.duration);
    }

    /// Lowercase the given query into the reusable scratch buffer, returning it
    /// so callers can borrow `self` immutably while matching.
    fn take_lowercased_query(&mut self, source_is_net: bool) -> String {
        let mut buf = std::mem::take(&mut self.query_scratch);
        buf.clear();
        let src = if source_is_net {
            &self.net_search_query
        } else {
            &self.search_query
        };
        for c in src.trim().chars() {
            for lc in c.to_lowercase() {
                buf.push(lc);
            }
        }
        buf
    }

    /// Open the process inspector, priming the metrics that are only sampled
    /// while a process is actually being looked at.
    pub fn open_process_details(&mut self, pid: u32) {
        self.monitor.refresh_process_disk_usage(pid);
        self.active_modal = Modal::ProcessDetails(pid);
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
        let query = self.take_lowercased_query(false);
        self.filtered_processes.clear();

        for (idx, p) in self.monitor.processes.iter().enumerate() {
            if query.is_empty()
                || p.name_lower.contains(&query)
                || number_contains(p.pid as u64, &query)
                || p.user_lower.contains(&query)
                || p.ports
                    .iter()
                    .any(|port| number_contains(*port as u64, &query))
                || p.cmd_lower.contains(&query)
            {
                self.filtered_processes.push(idx);
            }
        }
        self.query_scratch = query;

        let procs = &self.monitor.processes;
        let dir = self.sort_direction;
        match self.sort_column {
            SortColumn::Pid => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].pid.cmp(&procs[b].pid),
                SortDirection::Descending => procs[b].pid.cmp(&procs[a].pid),
            }),
            // Compares the pre-lowercased cache, so sorting by name no longer
            // allocates two Strings per comparison.
            SortColumn::Name => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a].name_lower.cmp(&procs[b].name_lower),
                SortDirection::Descending => procs[b].name_lower.cmp(&procs[a].name_lower),
            }),
            SortColumn::Cpu => self.filtered_processes.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => procs[a]
                    .cpu_usage
                    .partial_cmp(&procs[b].cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortDirection::Descending => procs[b]
                    .cpu_usage
                    .partial_cmp(&procs[a].cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
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
                SortDirection::Ascending => procs[a].user_lower.cmp(&procs[b].user_lower),
                SortDirection::Descending => procs[b].user_lower.cmp(&procs[a].user_lower),
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
            self.process_table_state
                .select(Some(self.selected_proc_idx));
        }
    }

    pub fn apply_network_filter_and_sort(&mut self) {
        let query = self.take_lowercased_query(true);
        let listening_only = self.listening_only;
        self.filtered_sockets.clear();

        for (idx, s) in self.monitor.sockets.iter().enumerate() {
            if listening_only && s.state != "LISTEN" {
                continue;
            }
            if query.is_empty()
                || number_contains(s.local_port as u64, &query)
                || contains_ci(s.protocol, &query)
                || contains_ci(s.state, &query)
                || contains_ci(&s.local_addr, &query)
                || contains_ci(&s.remote_addr, &query)
                || s.pids.iter().any(|p| number_contains(*p as u64, &query))
                || s.process_names.iter().any(|n| contains_ci(n, &query))
            {
                self.filtered_sockets.push(idx);
            }
        }
        self.query_scratch = query;

        let socks = &self.monitor.sockets;
        let dir = self.net_sort_direction;
        match self.net_sort_column {
            NetworkSortColumn::Port => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].local_port.cmp(&socks[b].local_port),
                SortDirection::Descending => socks[b].local_port.cmp(&socks[a].local_port),
            }),
            NetworkSortColumn::Protocol => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].protocol.cmp(socks[b].protocol),
                SortDirection::Descending => socks[b].protocol.cmp(socks[a].protocol),
            }),
            NetworkSortColumn::State => self.filtered_sockets.sort_by(|&a, &b| match dir {
                SortDirection::Ascending => socks[a].state.cmp(socks[b].state),
                SortDirection::Descending => socks[b].state.cmp(socks[a].state),
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
                let a_name = socks[a]
                    .process_names
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let b_name = socks[b]
                    .process_names
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("");
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

    /// Recompute the top-5 CPU and memory rankings.
    ///
    /// Uses a single scratch buffer that is reused across ticks, so this
    /// allocates nothing once the process count has settled.
    pub fn update_top_rankings(&mut self) {
        let procs = &self.monitor.processes;
        let scratch = &mut self.rank_scratch;
        let len = procs.len();

        scratch.clear();
        scratch.extend(0..len);
        if len > 5 {
            scratch.select_nth_unstable_by(5, |&a, &b| {
                procs[b]
                    .cpu_usage
                    .partial_cmp(&procs[a].cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            scratch.truncate(5);
        }
        scratch.sort_by(|&a, &b| {
            procs[b]
                .cpu_usage
                .partial_cmp(&procs[a].cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.top_cpu_indices.clear();
        self.top_cpu_indices.extend_from_slice(scratch);

        scratch.clear();
        scratch.extend(0..len);
        if len > 5 {
            scratch.select_nth_unstable_by(5, |&a, &b| {
                procs[b].memory_bytes.cmp(&procs[a].memory_bytes)
            });
            scratch.truncate(5);
        }
        scratch.sort_by_key(|&b| std::cmp::Reverse(procs[b].memory_bytes));
        self.top_mem_indices.clear();
        self.top_mem_indices.extend_from_slice(scratch);
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
        self.filtered_processes
            .get(self.selected_proc_idx)
            .and_then(|&idx| self.monitor.processes.get(idx))
    }

    pub fn get_selected_socket(&self) -> Option<&NetworkSocketItem> {
        self.filtered_sockets
            .get(self.selected_net_idx)
            .and_then(|&idx| self.monitor.sockets.get(idx))
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
                    let name = sock
                        .process_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string());
                    self.active_modal = Modal::ConfirmKill { pid, name, force };
                }
                None => {
                    self.add_toast(
                        "No PID associated with this port socket".to_string(),
                        ToastKind::Error,
                    );
                }
            }
        }
    }

    pub fn execute_kill(&mut self, pid: u32, force: bool) {
        match self.monitor.kill_process(pid, force) {
            Ok(()) => {
                self.add_toast(
                    format!(
                        "Successfully terminated PID {} {}",
                        pid,
                        if force { "(Force)" } else { "" }
                    ),
                    ToastKind::Success,
                );
                self.apply_process_filter_and_sort();
                self.update_top_rankings();
                self.apply_network_filter_and_sort();
            }
            Err(e) => {
                self.add_toast(
                    format!("Failed to kill PID {}: {}", pid, e),
                    ToastKind::Error,
                );
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
                SortColumn::Name
                | SortColumn::User
                | SortColumn::Status
                | SortColumn::Pid
                | SortColumn::Port => SortDirection::Ascending,
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
        self.add_toast(
            format!("Ports sorted by: {} ({})", col_name, dir_str),
            ToastKind::Info,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_process(pid: u32, name: &str, cpu_usage: f32, memory_bytes: u64) -> ProcessInfo {
        let mut p = ProcessInfo::new(
            pid,
            name.to_string(),
            "test_user".to_string(),
            String::new(),
        );
        p.cpu_usage = cpu_usage;
        p.memory_bytes = memory_bytes;
        p.status = "Run";
        p
    }

    #[test]
    fn test_visible_window_tracks_selection() {
        // Fresh view starts at the top.
        let mut offset = 0;
        assert_eq!(visible_window(100, 0, 10, &mut offset), 0..10);
        assert_eq!(offset, 0);

        // Moving below the window scrolls it down just far enough.
        assert_eq!(visible_window(100, 15, 10, &mut offset), 6..16);
        assert_eq!(offset, 6);

        // Moving above the window scrolls back up.
        assert_eq!(visible_window(100, 3, 10, &mut offset), 3..13);
        assert_eq!(offset, 3);

        // Selecting the final row clamps to the end of the list.
        assert_eq!(visible_window(100, 99, 10, &mut offset), 90..100);

        // A list shorter than the viewport is shown in full.
        let mut short = 5;
        assert_eq!(visible_window(4, 0, 10, &mut short), 0..4);
        assert_eq!(short, 0);

        // Degenerate cases stay in range.
        let mut empty = 7;
        assert_eq!(visible_window(0, 0, 10, &mut empty), 0..0);
        assert_eq!(empty, 0);
        let mut no_room = 3;
        assert_eq!(visible_window(50, 10, 0, &mut no_room), 0..0);
    }

    #[test]
    fn test_number_contains_matches_string_form() {
        assert!(number_contains(1234, "23"));
        assert!(number_contains(1234, "1234"));
        assert!(number_contains(0, "0"));
        assert!(number_contains(8080, "80"));
        assert!(!number_contains(1234, "56"));
        assert!(number_contains(42, ""));
        // Matches what `value.to_string().contains(..)` would have returned.
        for v in [0u64, 7, 99, 1234, u32::MAX as u64] {
            for needle in ["0", "1", "9", "23", "456"] {
                assert_eq!(
                    number_contains(v, needle),
                    v.to_string().contains(needle),
                    "mismatch for {v} / {needle}"
                );
            }
        }
    }

    #[test]
    fn test_contains_ci_is_case_insensitive() {
        assert!(contains_ci("LISTEN", "listen"));
        assert!(contains_ci("ESTABLISHED", "stab"));
        assert!(contains_ci("TCP", "tcp"));
        assert!(!contains_ci("UDP", "tcp"));
        assert!(contains_ci("anything", ""));
        assert!(!contains_ci("ab", "abc"));
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
                protocol: "TCP",
                local_addr: "127.0.0.1".to_string(),
                local_port: 8080,
                remote_addr: "0.0.0.0".to_string(),
                state: "LISTEN",
                pids: vec![101],
                process_names: vec!["web".to_string()],
            },
            NetworkSocketItem {
                protocol: "TCP",
                local_addr: "127.0.0.1".to_string(),
                local_port: 3000,
                remote_addr: "0.0.0.0".to_string(),
                state: "LISTEN",
                pids: vec![102],
                process_names: vec!["node".to_string()],
            },
            NetworkSocketItem {
                protocol: "UDP",
                local_addr: "0.0.0.0".to_string(),
                local_port: 53,
                remote_addr: "0.0.0.0".to_string(),
                state: "NONE",
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
