# ⚡ TaskTUI

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Edition](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](#-platform-support)
[![Memory](https://img.shields.io/badge/Memory-~15%20MB%20RSS-blueviolet.svg)](#-performance)

**A blazing-fast, lightweight, and modern terminal task manager and system monitor built with Rust and Ratatui.**

</div>

---

## 🌟 Core Features

### ⚙️ 1. Real-Time Process & Task Manager
* **Live System Metrics**: Monitor process PID, Name, CPU %, Memory (RSS & Virtual), Bound Ports, Status, User/Session, Start Time, and Runtime.
* **Deep Inspector Dialog (`Enter` / `d`)**: Inspect full executable paths, complete CLI arguments, disk read/written I/O bytes, and associated network ports in a focused modal.
* **Safe Termination (`k` / `K`)**: Prompt confirmation before killing processes with graceful (`SIGTERM` / `taskkill`) and forceful (`SIGKILL` / `taskkill /F`) options.

### 🌐 2. Network Port & Socket Inspector
* **Live Socket Resolution**: Automatically associates active TCP/UDP ports with corresponding process names and PIDs.
* **Connection State Tracking**: Displays socket states (`LISTEN`, `ESTABLISHED`, `TIME_WAIT`, `CLOSE_WAIT`, `CLOSED`) with color-coded badges.
* **Listening Filter (`L`)**: Instantly toggle between all active network connections and listening ports only.

### 🎛️ 3. Multi-Mode Live Dashboard
* **Combined View**: Side-by-side CPU/Memory history sparklines and Top Process rankings.
* **Top Rankings View**: High-density leaderboard highlighting the highest CPU (`🔥`) and RAM (`🧠`) consumer processes.
* **Graphs-Only View**: Triple-gauge real-time charts for CPU load, Memory usage, and Network RX/TX bandwidth rates.
* **Cycle Views (`v`)**: Switch between dashboard modes with a single keypress or click.

### 🔍 4. Instant Search & Multi-Column Sorting
* **Instant Filter (`/`)**: Live incremental search by process name, PID, user, port number, or CLI arguments.
* **One-Key Column Sorting**:
  * `m`: Sort by Memory (RAM)
  * `c`: Sort by CPU %
  * `p`: Sort by PID
  * `n`: Sort by Name
  * `o`: Sort by Port
  * `r`: Invert sort direction (**High → Low ▼** ⇄ **Low → High ▲**)

### 📊 5. Hardware Specs & Storage Partitions
* **Hardware Overview**: Hostname, OS distribution, kernel version, CPU model, core count, total RAM, and Swap.
* **Storage Disks & Mounts**: All mounted drives, filesystem types, used/free/total disk capacities, and visual usage bars.
* **Network Interfaces**: Live packet counters and total data transferred/received per interface.

### 🖱️ 6. Full Mouse & Keyboard Interaction
* **Mouse Navigation**: Click tabs to switch views, click column headers to sort, click rows to select/inspect, right-click to trigger kill prompt, and scroll wheel to scroll tables.
* **Vim & Standard Navigation**: Supports both arrow keys and Vim keybindings (`j` / `k`, `Home` / `End`, `PgUp` / `PgDn`).

---

## ⚡ Quick Feature & Shortcut Reference

| Category | Shortcut | Action |
| :--- | :---: | :--- |
| **Tabs** | `Tab` / `1-4` | Switch between **Tasks**, **Ports**, **Specs**, and **Help** |
| **Inspect** | `Enter` / `d` | Open deep Process Details Inspector |
| **Terminate** | `k` / `Delete` | Graceful process termination modal (`SIGTERM`) |
| **Force Kill**| `K` (`Shift+k`) | Force kill process modal (`SIGKILL`) |
| **Search** | `/` | Open live filter query (Press `Esc` to clear/exit) |
| **Sort** | `m` / `c` / `p` / `n` / `o` | Sort by Memory, CPU, PID, Name, or Port |
| **Invert** | `r` | Invert sort order (High ⇄ Low) |
| **Dashboard**| `v` | Cycle dashboard layout (Combined ⇄ Top Rank ⇄ Graphs) |
| **Pause** | `Space` | Pause / resume live metric updates |
| **Refresh** | `F5` | Manually force-refresh all metrics |
| **Quit** | `q` / `Ctrl+C` | Clean exit and terminal restoration |

---

## 🚀 Getting Started

### Installation
```bash
# Clone the repository
git clone https://github.com/Fankrits/TaskTUI.git
cd TaskTUI

# Build and run
cargo run --release
```

### Install Globally
```bash
cargo install --path .
task_tui
```

---

## 💻 Platform Support

* **macOS**: Apple Silicon (M1/M2/M3/M4) & Intel (x86_64)
* **Linux**: Ubuntu, Debian, Arch, Fedora, Alpine (musl), Void, NixOS
* **Windows**: Windows 10/11 (Windows Terminal, PowerShell, CMD)
* **Terminals**: iTerm2, Alacritty, Kitty, WezTerm, VS Code Terminal, GNOME Terminal, Tmux

---

## 📄 License

This project is licensed under the **[MIT License](LICENSE)**. See [LICENSE](LICENSE) for full details.
