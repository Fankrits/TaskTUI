use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use std::collections::HashMap;
use std::time::Instant;
use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind, ProcessesToUpdate,
    RefreshKind, System, UpdateKind, Users,
};

pub const HISTORY_LEN: usize = 60;

/// A process row.
///
/// Fields are split into two classes for performance. *Identity* fields
/// (`name`, `user`, `cmd`, `exe_path`, `session_id`, `start_time`) never change
/// for the lifetime of a PID, so they are built once when the process is first
/// seen and then left alone. *Volatile* fields (cpu, memory, status, ports) are
/// overwritten in place on every refresh. This keeps a steady-state refresh
/// free of heap allocation — see `SystemMonitor::refresh`.
///
/// The `*_lower` fields are pre-computed ASCII-lowercase copies used by the
/// search filter and the name/user sort comparators, so neither has to
/// allocate a temporary `String` per comparison.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub name_lower: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub virtual_memory_bytes: u64,
    pub status: &'static str,
    pub user: String,
    pub user_lower: String,
    pub session_id: Option<u32>,
    pub ports: Vec<u16>,
    pub exe_path: String,
    pub cmd: String,
    pub cmd_lower: String,
    pub start_time: u64,
    pub run_time_secs: u64,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
}

impl ProcessInfo {
    /// Build a row with the lowercase search caches kept consistent with the
    /// identity fields. Prefer this over a struct literal so the two can never
    /// drift apart.
    pub fn new(pid: u32, name: String, user: String, cmd: String) -> Self {
        Self {
            pid,
            name_lower: name.to_lowercase(),
            name,
            cpu_usage: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            virtual_memory_bytes: 0,
            status: "Other",
            user_lower: user.to_lowercase(),
            user,
            session_id: None,
            ports: Vec::new(),
            exe_path: String::new(),
            cmd_lower: cmd.to_lowercase(),
            cmd,
            start_time: 0,
            run_time_secs: 0,
            disk_read_bytes: 0,
            disk_written_bytes: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetworkSocketItem {
    /// Static label ("TCP"/"UDP") — no per-socket allocation.
    pub protocol: &'static str,
    pub local_port: u16,
    pub local_addr: String,
    pub remote_addr: String,
    /// Static label ("LISTEN", "ESTABLISHED", ...) — no per-socket allocation.
    pub state: &'static str,
    pub pids: Vec<u32>,
    pub process_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub file_system: String,
}

#[allow(dead_code)]
pub struct SystemMonitor {
    pub sys: System,
    pub users: Users,
    pub disks: Disks,
    pub networks: Networks,
    pub last_refresh: Instant,
    pub last_disk_refresh: Instant,
    pub last_socket_refresh: Instant,
    pub cached_pid_ports: HashMap<u32, Vec<u16>>,
    pub user_cache: HashMap<sysinfo::Uid, String>,

    // System static info
    pub host_name: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_brand: String,
    pub cpu_core_count: usize,
    pub total_memory: u64,
    pub total_swap: u64,

    // Live Metrics
    pub used_memory: u64,
    pub used_swap: u64,
    pub global_cpu: f32,
    pub per_core_cpu: Vec<f32>,

    // Historical Ring Buffers for charts
    pub cpu_history: [u64; HISTORY_LEN],
    pub mem_history: [u64; HISTORY_LEN],
    pub net_rx_history: [u64; HISTORY_LEN],
    pub net_tx_history: [u64; HISTORY_LEN],

    // Network / Disk live rate counters
    pub last_net_rx: u64,
    pub last_net_tx: u64,
    pub current_net_rx_rate: f64,
    pub current_net_tx_rate: f64,

    // Cached Process & Socket Data
    pub processes: Vec<ProcessInfo>,
    pub sockets: Vec<NetworkSocketItem>,

    /// Cached disk rows, rebuilt only when `disks` is actually refreshed
    /// (every 15s) instead of once per rendered frame.
    pub disks_info: Vec<DiskInfo>,

    /// PID -> slot in `processes`, so a refresh can update rows in place
    /// instead of rebuilding the whole vector.
    pid_index: HashMap<u32, usize>,
    /// Liveness marker, parallel to `processes`, reused across refreshes.
    proc_seen: Vec<bool>,
}

impl SystemMonitor {
    pub fn map_status(status: sysinfo::ProcessStatus) -> &'static str {
        match status {
            sysinfo::ProcessStatus::Run => "Run",
            sysinfo::ProcessStatus::Sleep => "Sleep",
            sysinfo::ProcessStatus::Idle => "Idle",
            sysinfo::ProcessStatus::Zombie => "Zombie",
            sysinfo::ProcessStatus::Dead => "Dead",
            sysinfo::ProcessStatus::Stop => "Stop",
            _ => "Other",
        }
    }

    pub fn new() -> Self {
        // Only CPU *usage* is ever displayed. `CpuRefreshKind::everything()`
        // would additionally sample per-core frequency, which costs an extra
        // sysfs/proc read per core on every tick for data nothing renders.
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(
                ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory()
                    .with_exe(UpdateKind::OnlyIfNotSet)
                    .with_cmd(UpdateKind::OnlyIfNotSet)
                    .with_user(UpdateKind::OnlyIfNotSet)
                    .without_tasks()
                    .without_environ(),
            );
        let sys = System::new_with_specifics(refresh_kind);

        let users = Users::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let host_name = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let os_name = System::name().unwrap_or_else(|| "Unknown OS".to_string());
        let os_version = System::os_version().unwrap_or_default();
        let kernel_version = System::kernel_version().unwrap_or_default();

        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "CPU".to_string());
        let cpu_core_count = sys.cpus().len();
        let total_memory = sys.total_memory();
        let total_swap = sys.total_swap();

        let now = Instant::now();

        let mut monitor = Self {
            sys,
            users,
            disks,
            networks,
            last_refresh: now,
            last_disk_refresh: now,
            last_socket_refresh: now
                .checked_sub(std::time::Duration::from_secs(5))
                .unwrap_or(now),
            cached_pid_ports: HashMap::new(),
            user_cache: HashMap::new(),
            host_name,
            os_name,
            os_version,
            kernel_version,
            cpu_brand,
            cpu_core_count,
            total_memory,
            total_swap,
            used_memory: 0,
            used_swap: 0,
            global_cpu: 0.0,
            per_core_cpu: Vec::new(),
            cpu_history: [0; HISTORY_LEN],
            mem_history: [0; HISTORY_LEN],
            net_rx_history: [0; HISTORY_LEN],
            net_tx_history: [0; HISTORY_LEN],
            last_net_rx: 0,
            last_net_tx: 0,
            current_net_rx_rate: 0.0,
            current_net_tx_rate: 0.0,
            processes: Vec::new(),
            sockets: Vec::new(),
            disks_info: Vec::new(),
            pid_index: HashMap::new(),
            proc_seen: Vec::new(),
        };

        monitor.rebuild_disks_info();
        monitor.refresh();
        monitor
    }

    fn rebuild_disks_info(&mut self) {
        self.disks_info.clear();
        self.disks_info.extend(self.disks.iter().map(|d| DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            file_system: d.file_system().to_string_lossy().to_string(),
        }));
    }

    pub fn refresh(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refresh).as_secs_f64().max(0.1);
        self.last_refresh = now;

        // Targeted CPU & Memory Refresh
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_user(UpdateKind::OnlyIfNotSet)
                .without_tasks()
                .without_environ(),
        );
        self.networks.refresh(true);

        // Only refresh disks every 15 seconds or when needed
        if self.last_disk_refresh.elapsed() >= std::time::Duration::from_secs(15) {
            self.disks.refresh(true);
            self.last_disk_refresh = now;
            self.rebuild_disks_info();
        }

        self.used_memory = self.sys.used_memory();
        self.used_swap = self.sys.used_swap();
        self.global_cpu = self.sys.global_cpu_usage();

        // Reuse the existing buffer rather than collecting a fresh Vec each tick.
        self.per_core_cpu.clear();
        self.per_core_cpu
            .extend(self.sys.cpus().iter().map(|c| c.cpu_usage()));

        // Shift and push to fixed arrays
        let cpu_val = (self.global_cpu.clamp(0.0, 100.0)).round() as u64;
        let mem_val = if self.total_memory > 0 {
            ((self.used_memory as f64 / self.total_memory as f64 * 100.0).clamp(0.0, 100.0)).round()
                as u64
        } else {
            0
        };

        self.cpu_history.rotate_left(1);
        self.cpu_history[HISTORY_LEN - 1] = cpu_val;

        self.mem_history.rotate_left(1);
        self.mem_history[HISTORY_LEN - 1] = mem_val;

        // Network rates
        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;
        for (_interface_name, data) in &self.networks {
            total_rx += data.total_received();
            total_tx += data.total_transmitted();
        }

        if self.last_net_rx > 0 && total_rx >= self.last_net_rx {
            self.current_net_rx_rate = (total_rx - self.last_net_rx) as f64 / elapsed;
        }
        if self.last_net_tx > 0 && total_tx >= self.last_net_tx {
            self.current_net_tx_rate = (total_tx - self.last_net_tx) as f64 / elapsed;
        }
        self.last_net_rx = total_rx;
        self.last_net_tx = total_tx;

        let rx_kb = (self.current_net_rx_rate / 1024.0).round() as u64;
        let tx_kb = (self.current_net_tx_rate / 1024.0).round() as u64;

        self.net_rx_history.rotate_left(1);
        self.net_rx_history[HISTORY_LEN - 1] = rx_kb;

        self.net_tx_history.rotate_left(1);
        self.net_tx_history[HISTORY_LEN - 1] = tx_kb;

        // Map listening / active ports to PIDs - throttled to every 3 seconds
        let sockets_changed = self.last_socket_refresh.elapsed()
            >= std::time::Duration::from_secs(3)
            || self.sockets.is_empty();
        if sockets_changed {
            let (pid_to_ports, socket_list) = Self::collect_sockets();
            self.cached_pid_ports = pid_to_ports;
            self.sockets = socket_list;
            self.last_socket_refresh = now;
        }

        let system_uptime = System::uptime();

        // Update the process table in place. Rows that already exist keep their
        // identity strings (name/user/cmd/exe) and only have their volatile
        // metrics overwritten, so a steady-state tick allocates nothing.
        //
        // Fields are borrowed individually so the immutable borrow of `sys`
        // can coexist with the mutable borrows of the caches below.
        let sys = &self.sys;
        let users = &self.users;
        let user_cache = &mut self.user_cache;
        let processes = &mut self.processes;
        let pid_index = &mut self.pid_index;
        let proc_seen = &mut self.proc_seen;
        let cached_pid_ports = &self.cached_pid_ports;
        let total_memory = self.total_memory;

        proc_seen.clear();
        proc_seen.resize(processes.len(), false);

        let mut alive = 0usize;
        for (pid, process) in sys.processes() {
            let pid_u32 = pid.as_u32();

            let slot = match pid_index.get(&pid_u32) {
                Some(&slot) => slot,
                None => {
                    // Newly observed PID: build its identity strings once.
                    let name = process.name().to_string_lossy().to_string();
                    let user = match process.user_id() {
                        Some(uid) => user_cache
                            .entry(uid.clone())
                            .or_insert_with(|| {
                                users
                                    .get_user_by_id(uid)
                                    .map(|u| u.name().to_string())
                                    .unwrap_or_else(|| "-".to_string())
                            })
                            .clone(),
                        None => "-".to_string(),
                    };
                    let cmd = if process.cmd().is_empty() {
                        String::new()
                    } else {
                        let mut buf = String::new();
                        for (i, part) in process.cmd().iter().enumerate() {
                            if i > 0 {
                                buf.push(' ');
                            }
                            buf.push_str(&part.to_string_lossy());
                        }
                        buf
                    };

                    let mut info = ProcessInfo::new(pid_u32, name, user, cmd);
                    info.session_id = process.session_id().map(|s| s.as_u32());
                    info.start_time = process.start_time();
                    info.exe_path = process
                        .exe()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let slot = processes.len();
                    processes.push(info);
                    proc_seen.push(false);
                    pid_index.insert(pid_u32, slot);
                    slot
                }
            };

            proc_seen[slot] = true;
            alive += 1;

            // Volatile metrics — overwritten every tick, never reallocated.
            let p = &mut processes[slot];
            p.cpu_usage = process.cpu_usage();
            p.memory_bytes = process.memory();
            p.virtual_memory_bytes = process.virtual_memory();
            p.memory_percent = if total_memory > 0 {
                (p.memory_bytes as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            };
            p.status = Self::map_status(process.status());
            p.run_time_secs = system_uptime.saturating_sub(p.start_time);

            let disk_usage = process.disk_usage();
            p.disk_read_bytes = disk_usage.read_bytes;
            p.disk_written_bytes = disk_usage.written_bytes;

            p.ports.clear();
            if let Some(ports) = cached_pid_ports.get(&pid_u32) {
                p.ports.extend_from_slice(ports);
            }
        }

        // Drop rows for processes that have exited, then re-index. Only runs on
        // the ticks where something actually died.
        if alive < processes.len() {
            let mut slot = 0usize;
            processes.retain(|_| {
                let keep = proc_seen[slot];
                slot += 1;
                keep
            });
            pid_index.clear();
            for (i, p) in processes.iter().enumerate() {
                pid_index.insert(p.pid, i);
            }
        }

        // Socket -> process-name association only needs redoing when the socket
        // list itself was re-collected (every 3s), not on every tick.
        if sockets_changed {
            self.attach_socket_process_names();
        }
    }

    /// Sample disk I/O counters for a single process.
    ///
    /// Disk usage is the only per-process metric that costs an extra `/proc`
    /// read per PID, and it is only ever displayed in the details modal. Rather
    /// than pay for it across every process on every tick, the app calls this
    /// for the one process currently being inspected.
    pub fn refresh_process_disk_usage(&mut self, pid: u32) {
        let target = sysinfo::Pid::from_u32(pid);
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[target]),
            false,
            ProcessRefreshKind::nothing().with_disk_usage(),
        );

        let Some(process) = self.sys.process(target) else {
            return;
        };
        let usage = process.disk_usage();
        if let Some(&slot) = self.pid_index.get(&pid) {
            self.processes[slot].disk_read_bytes = usage.read_bytes;
            self.processes[slot].disk_written_bytes = usage.written_bytes;
        }
    }

    /// Fill in `process_names` for each socket via an O(1) PID lookup.
    fn attach_socket_process_names(&mut self) {
        let processes = &self.processes;
        let pid_index = &self.pid_index;
        for sock in &mut self.sockets {
            sock.process_names.clear();
            for pid in &sock.pids {
                if let Some(&slot) = pid_index.get(pid) {
                    sock.process_names.push(processes[slot].name.clone());
                }
            }
        }
    }

    pub fn refresh_sockets(&mut self) {
        let (pid_to_ports, socket_list) = Self::collect_sockets();
        self.cached_pid_ports = pid_to_ports;
        self.sockets = socket_list;
        self.last_socket_refresh = Instant::now();

        self.attach_socket_process_names();

        // Update ports on existing processes, reusing each row's buffer.
        for proc in &mut self.processes {
            proc.ports.clear();
            if let Some(ports) = self.cached_pid_ports.get(&proc.pid) {
                proc.ports.extend_from_slice(ports);
            }
        }
    }

    fn collect_sockets() -> (HashMap<u32, Vec<u16>>, Vec<NetworkSocketItem>) {
        let mut pid_to_ports: HashMap<u32, Vec<u16>> = HashMap::new();
        let mut socket_items = Vec::new();

        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

        if let Ok(sockets) = get_sockets_info(af_flags, proto_flags) {
            for s in sockets {
                let pids = s.associated_pids;
                match s.protocol_socket_info {
                    ProtocolSocketInfo::Tcp(tcp) => {
                        let local_port = tcp.local_port;
                        let local_addr = format!("{}:{}", tcp.local_addr, tcp.local_port);
                        let remote_addr = if tcp.remote_port > 0 {
                            format!("{}:{}", tcp.remote_addr, tcp.remote_port)
                        } else {
                            "*:*".to_string()
                        };
                        let state = match tcp.state {
                            TcpState::Listen => "LISTEN",
                            TcpState::Established => "ESTABLISHED",
                            TcpState::SynSent => "SYN_SENT",
                            TcpState::SynReceived => "SYN_RECV",
                            TcpState::FinWait1 => "FIN_WAIT_1",
                            TcpState::FinWait2 => "FIN_WAIT_2",
                            TcpState::TimeWait => "TIME_WAIT",
                            TcpState::Closed => "CLOSED",
                            TcpState::CloseWait => "CLOSE_WAIT",
                            TcpState::LastAck => "LAST_ACK",
                            TcpState::Closing => "CLOSING",
                            _ => "UNKNOWN",
                        };

                        for &p in &pids {
                            let entry = pid_to_ports.entry(p).or_default();
                            if !entry.contains(&local_port) {
                                entry.push(local_port);
                            }
                        }

                        socket_items.push(NetworkSocketItem {
                            protocol: "TCP",
                            local_port,
                            local_addr,
                            remote_addr,
                            state,
                            pids,
                            process_names: Vec::new(),
                        });
                    }
                    ProtocolSocketInfo::Udp(udp) => {
                        let local_port = udp.local_port;
                        let local_addr = format!("{}:{}", udp.local_addr, udp.local_port);
                        let remote_addr = "*:*".to_string();
                        let state = "UDP";

                        for &p in &pids {
                            let entry = pid_to_ports.entry(p).or_default();
                            if !entry.contains(&local_port) {
                                entry.push(local_port);
                            }
                        }

                        socket_items.push(NetworkSocketItem {
                            protocol: "UDP",
                            local_port,
                            local_addr,
                            remote_addr,
                            state,
                            pids,
                            process_names: Vec::new(),
                        });
                    }
                }
            }
        }

        // Sort ports for each PID
        for ports in pid_to_ports.values_mut() {
            ports.sort_unstable();
        }

        (pid_to_ports, socket_items)
    }

    /// Borrow the cached disk rows. Rebuilt on the 15s disk refresh, so the
    /// render path never pays for it.
    pub fn get_disks_info(&self) -> &[DiskInfo] {
        &self.disks_info
    }

    pub fn kill_process(&mut self, pid: u32, force: bool) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = std::process::Command::new("taskkill");
            if force {
                cmd.args(["/F", "/PID", &pid.to_string()]);
            } else {
                cmd.args(["/PID", &pid.to_string()]);
            }

            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        self.refresh();
                        Ok(())
                    } else {
                        let err_msg = String::from_utf8_lossy(&output.stderr);
                        let stdout_msg = String::from_utf8_lossy(&output.stdout);
                        let msg = if !err_msg.trim().is_empty() {
                            err_msg.trim().to_string()
                        } else if !stdout_msg.trim().is_empty() {
                            stdout_msg.trim().to_string()
                        } else {
                            format!("Failed to kill PID {}", pid)
                        };
                        Err(msg)
                    }
                }
                Err(e) => Err(format!("Command execution error: {}", e)),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let target_pid = sysinfo::Pid::from_u32(pid);
            if let Some(process) = self.sys.process(target_pid) {
                let killed = if force {
                    process.kill_with(sysinfo::Signal::Kill).unwrap_or(false)
                } else {
                    process.kill_with(sysinfo::Signal::Term).unwrap_or(false) || process.kill()
                };

                if killed {
                    self.refresh();
                    Ok(())
                } else {
                    Err(format!("Could not send kill signal to PID {}", pid))
                }
            } else {
                Err(format!("Process PID {} not found", pid))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_array_history_buffers() {
        let mut monitor = SystemMonitor::new();
        assert_eq!(monitor.cpu_history.len(), HISTORY_LEN);
        assert_eq!(monitor.mem_history.len(), HISTORY_LEN);
        assert_eq!(monitor.net_rx_history.len(), HISTORY_LEN);
        assert_eq!(monitor.net_tx_history.len(), HISTORY_LEN);

        monitor.refresh();
        assert!(monitor.cpu_history[HISTORY_LEN - 1] <= 100);
        assert!(monitor.mem_history[HISTORY_LEN - 1] <= 100);
    }

    #[test]
    fn test_system_monitor_initialization() {
        let monitor = SystemMonitor::new();
        assert!(!monitor.cpu_brand.is_empty() || monitor.cpu_core_count > 0);
        assert_eq!(monitor.cpu_history.len(), HISTORY_LEN);
        assert_eq!(monitor.mem_history.len(), HISTORY_LEN);
        assert_eq!(monitor.net_rx_history.len(), HISTORY_LEN);
        assert_eq!(monitor.net_tx_history.len(), HISTORY_LEN);
    }

    #[test]
    fn test_system_monitor_refresh_updates_metrics() {
        let mut monitor = SystemMonitor::new();
        let initial_last_refresh = monitor.last_refresh;

        std::thread::sleep(Duration::from_millis(10));
        monitor.refresh();

        assert!(monitor.last_refresh >= initial_last_refresh);
        assert_eq!(monitor.cpu_history.len(), HISTORY_LEN);
        assert_eq!(monitor.mem_history.len(), HISTORY_LEN);
    }

    #[test]
    fn test_user_cache_populated_and_persisted() {
        let mut monitor = SystemMonitor::new();
        monitor.refresh();

        let initial_cache_count = monitor.user_cache.len();
        monitor.refresh();
        assert!(monitor.user_cache.len() >= initial_cache_count);
    }

    #[test]
    fn test_disk_refresh_throttling() {
        let mut monitor = SystemMonitor::new();

        let first_disk_refresh = monitor.last_disk_refresh;

        monitor.refresh();
        assert_eq!(monitor.last_disk_refresh, first_disk_refresh);

        // Manually simulate 16 seconds elapsed
        monitor.last_disk_refresh = Instant::now() - Duration::from_secs(16);
        let past_disk_refresh = monitor.last_disk_refresh;

        monitor.refresh();
        assert!(monitor.last_disk_refresh > past_disk_refresh);
    }

    #[test]
    fn test_socket_refresh_throttling() {
        let mut monitor = SystemMonitor::new();

        let first_socket_refresh = monitor.last_socket_refresh;

        // Calling refresh immediately should not re-query sockets
        monitor.refresh();
        assert_eq!(monitor.last_socket_refresh, first_socket_refresh);

        // Manually simulate 4 seconds elapsed
        monitor.last_socket_refresh = Instant::now() - Duration::from_secs(4);
        let past_socket_refresh = monitor.last_socket_refresh;

        monitor.refresh();
        assert!(monitor.last_socket_refresh > past_socket_refresh);
    }

    #[test]
    fn test_force_refresh_sockets() {
        let mut monitor = SystemMonitor::new();
        let initial_refresh = monitor.last_socket_refresh;

        std::thread::sleep(Duration::from_millis(10));
        monitor.refresh_sockets();

        assert!(monitor.last_socket_refresh > initial_refresh);
    }

    #[test]
    fn test_get_disks_info() {
        let monitor = SystemMonitor::new();
        let disks = monitor.get_disks_info();
        for disk in disks {
            assert!(!disk.mount_point.is_empty() || !disk.name.is_empty());
        }
    }

    #[test]
    fn test_status_mapping_static_str() {
        assert_eq!(
            SystemMonitor::map_status(sysinfo::ProcessStatus::Run),
            "Run"
        );
        assert_eq!(
            SystemMonitor::map_status(sysinfo::ProcessStatus::Sleep),
            "Sleep"
        );
        assert_eq!(
            SystemMonitor::map_status(sysinfo::ProcessStatus::Idle),
            "Idle"
        );
        assert_eq!(
            SystemMonitor::map_status(sysinfo::ProcessStatus::Zombie),
            "Zombie"
        );
        assert_eq!(
            SystemMonitor::map_status(sysinfo::ProcessStatus::Dead),
            "Dead"
        );
    }

    #[test]
    fn test_socket_process_name_association() {
        let mut monitor = SystemMonitor::new();
        monitor.refresh_sockets();
        for sock in &monitor.sockets {
            for name in &sock.process_names {
                assert!(!name.is_empty());
            }
        }
    }
}
