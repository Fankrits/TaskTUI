use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use sysinfo::{
    Disks, Networks, System, Users,
};
use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};

const HISTORY_LEN: usize = 60;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub virtual_memory_bytes: u64,
    pub status: String,
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
    pub cpu_history: VecDeque<f64>,
    pub mem_history: VecDeque<f64>,
    pub net_rx_history: VecDeque<f64>,
    pub net_tx_history: VecDeque<f64>,

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
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let users = Users::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let host_name = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let os_name = System::name().unwrap_or_else(|| "Unknown OS".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "".to_string());

        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "CPU".to_string());
        let cpu_core_count = sys.cpus().len();
        let total_memory = sys.total_memory();
        let total_swap = sys.total_swap();

        let mut monitor = Self {
            sys,
            users,
            disks,
            networks,
            last_refresh: Instant::now(),
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
            cpu_history: VecDeque::from(vec![0.0; HISTORY_LEN]),
            mem_history: VecDeque::from(vec![0.0; HISTORY_LEN]),
            net_rx_history: VecDeque::from(vec![0.0; HISTORY_LEN]),
            net_tx_history: VecDeque::from(vec![0.0; HISTORY_LEN]),
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

        self.sys.refresh_all();
        self.networks.refresh(true);
        self.disks.refresh(true);

        self.used_memory = self.sys.used_memory();
        self.used_swap = self.sys.used_swap();
        self.global_cpu = self.sys.global_cpu_usage();

        self.per_core_cpu = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        // Update history ring buffers
        let cpu_val = self.global_cpu.clamp(0.0, 100.0) as f64;
        let mem_val = if self.total_memory > 0 {
            (self.used_memory as f64 / self.total_memory as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        if self.cpu_history.len() >= HISTORY_LEN {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(cpu_val);

        if self.mem_history.len() >= HISTORY_LEN {
            self.mem_history.pop_front();
        }
        self.mem_history.push_back(mem_val);

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

        if self.net_rx_history.len() >= HISTORY_LEN {
            self.net_rx_history.pop_front();
        }
        self.net_rx_history.push_back(self.current_net_rx_rate / 1024.0); // KB/s

        if self.net_tx_history.len() >= HISTORY_LEN {
            self.net_tx_history.pop_front();
        }
        self.net_tx_history.push_back(self.current_net_tx_rate / 1024.0); // KB/s

        // Map listening / active ports to PIDs
        let (pid_to_ports, socket_list) = Self::collect_sockets();
        self.sockets = socket_list;

        // Process user mapping
        let mut pid_user_map: HashMap<u32, String> = HashMap::new();
        for (pid, process) in self.sys.processes() {
            if let Some(uid) = process.user_id() {
                if let Some(user) = self.users.get_user_by_id(uid) {
                    pid_user_map.insert(pid.as_u32(), user.name().to_string());
                }
            }
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

            let status_str = format!("{:?}", process.status());
            let user = pid_user_map
                .get(&pid_u32)
                .cloned()
                .unwrap_or_else(|| "-".to_string());
            let session_id = process.session_id().map(|s| s.as_u32());
            let ports = pid_to_ports.get(&pid_u32).cloned().unwrap_or_default();

            let exe_path = process
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let cmd = process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");

            let start_time = process.start_time();
            let run_time_secs = if system_uptime >= start_time {
                system_uptime - start_time
            } else {
                0
            };

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

        // Attach process names to sockets
        for sock in &mut self.sockets {
            let mut names = Vec::new();
            for p in &sock.pids {
                if let Some(proc) = proc_list.iter().find(|pr| pr.pid == *p) {
                    names.push(proc.name.clone());
                }
            }
            sock.process_names = names;
        }

        self.processes = proc_list;
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
