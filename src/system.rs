use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use std::collections::HashMap;
use std::time::Instant;
use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind, ProcessesToUpdate,
    RefreshKind, System, UpdateKind, Users,
};

pub const HISTORY_LEN: usize = 60;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub virtual_memory_bytes: u64,
    pub status: &'static str,
    pub user: String,
    pub session_id: Option<u32>,
    pub ports: Vec<u16>,
    pub exe_path: String,
    pub cmd: String,
    pub start_time: u64,
    pub run_time_secs: u64,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct NetworkSocketItem {
    pub protocol: String,
    pub local_port: u16,
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
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
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(
                ProcessRefreshKind::nothing()
                    .with_cpu()
                    .with_memory()
                    .with_disk_usage()
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
        };

        monitor.refresh();
        monitor
    }

    pub fn refresh(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refresh).as_secs_f64().max(0.1);
        self.last_refresh = now;

        // Targeted CPU & Memory Refresh
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage()
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
        }

        self.used_memory = self.sys.used_memory();
        self.used_swap = self.sys.used_swap();
        self.global_cpu = self.sys.global_cpu_usage();

        self.per_core_cpu = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

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
        if self.last_socket_refresh.elapsed() >= std::time::Duration::from_secs(3)
            || self.sockets.is_empty()
        {
            let (pid_to_ports, socket_list) = Self::collect_sockets();
            self.cached_pid_ports = pid_to_ports;
            self.sockets = socket_list;
            self.last_socket_refresh = now;
        }

        let system_uptime = System::uptime();

        let mut proc_list: Vec<ProcessInfo> = Vec::with_capacity(self.sys.processes().len());
        for (pid, process) in self.sys.processes() {
            let pid_u32 = pid.as_u32();
            let name = process.name().to_string_lossy().to_string();
            let cpu_usage = process.cpu_usage();
            let memory_bytes = process.memory();
            let virtual_memory_bytes = process.virtual_memory();
            let memory_percent = if self.total_memory > 0 {
                (memory_bytes as f32 / self.total_memory as f32) * 100.0
            } else {
                0.0
            };

            let status_str = Self::map_status(process.status());
            let user = match process.user_id() {
                Some(uid) => {
                    if let Some(cached_user) = self.user_cache.get(uid) {
                        cached_user.clone()
                    } else {
                        let user_name = self
                            .users
                            .get_user_by_id(uid)
                            .map(|u| u.name().to_string())
                            .unwrap_or_else(|| "-".to_string());
                        self.user_cache.insert(uid.clone(), user_name.clone());
                        user_name
                    }
                }
                None => "-".to_string(),
            };

            let session_id = process.session_id().map(|s| s.as_u32());
            let ports = self
                .cached_pid_ports
                .get(&pid_u32)
                .cloned()
                .unwrap_or_default();

            let exe_path = process
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let cmd = if process.cmd().is_empty() {
                String::new()
            } else {
                process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            };

            let start_time = process.start_time();
            let run_time_secs = system_uptime.saturating_sub(start_time);

            let disk_usage = process.disk_usage();
            let disk_read_bytes = disk_usage.read_bytes;
            let disk_written_bytes = disk_usage.written_bytes;

            proc_list.push(ProcessInfo {
                pid: pid_u32,
                name,
                cpu_usage,
                memory_bytes,
                memory_percent,
                virtual_memory_bytes,
                status: status_str,
                user,
                session_id,
                ports,
                exe_path,
                cmd,
                start_time,
                run_time_secs,
                disk_read_bytes,
                disk_written_bytes,
            });
        }

        // Build O(1) PID -> Name lookup
        let pid_to_name: HashMap<u32, &str> =
            proc_list.iter().map(|p| (p.pid, p.name.as_str())).collect();

        // Attach process names to sockets in O(S * P) instead of O(S * P * N)
        for sock in &mut self.sockets {
            let mut names = Vec::with_capacity(sock.pids.len());
            for p in &sock.pids {
                if let Some(&name) = pid_to_name.get(p) {
                    names.push(name.to_string());
                }
            }
            sock.process_names = names;
        }

        self.processes = proc_list;
    }

    pub fn refresh_sockets(&mut self) {
        let (pid_to_ports, socket_list) = Self::collect_sockets();
        self.cached_pid_ports = pid_to_ports;
        self.sockets = socket_list;
        self.last_socket_refresh = Instant::now();

        // Update process names for sockets using existing process list
        let pid_to_name: HashMap<u32, &str> = self
            .processes
            .iter()
            .map(|p| (p.pid, p.name.as_str()))
            .collect();
        for sock in &mut self.sockets {
            let mut names = Vec::with_capacity(sock.pids.len());
            for p in &sock.pids {
                if let Some(&name) = pid_to_name.get(p) {
                    names.push(name.to_string());
                }
            }
            sock.process_names = names;
        }

        // Update ports on existing processes
        for proc in &mut self.processes {
            if let Some(ports) = self.cached_pid_ports.get(&proc.pid) {
                proc.ports = ports.clone();
            } else {
                proc.ports.clear();
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
                        }
                        .to_string();

                        for &p in &pids {
                            let entry = pid_to_ports.entry(p).or_default();
                            if !entry.contains(&local_port) {
                                entry.push(local_port);
                            }
                        }

                        socket_items.push(NetworkSocketItem {
                            protocol: "TCP".to_string(),
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
                        let state = "UDP".to_string();

                        for &p in &pids {
                            let entry = pid_to_ports.entry(p).or_default();
                            if !entry.contains(&local_port) {
                                entry.push(local_port);
                            }
                        }

                        socket_items.push(NetworkSocketItem {
                            protocol: "UDP".to_string(),
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

    pub fn get_disks_info(&self) -> Vec<DiskInfo> {
        self.disks
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
                file_system: d.file_system().to_string_lossy().to_string(),
            })
            .collect()
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
        for disk in &disks {
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
