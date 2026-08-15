use std::process::Command;

pub fn print_version() {
    println!("tasktui {}", env!("CARGO_PKG_VERSION"));
}

pub fn print_help() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "\x1b[1;34m⚡ TaskTUI\x1b[0m v{version}
A blazing-fast, lightweight, and modern terminal task manager & system monitor.

\x1b[1;33mUSAGE:\x1b[0m
    tasktui [COMMAND] [OPTIONS]

\x1b[1;33mCOMMANDS:\x1b[0m
    \x1b[1;32mupdate, upgrade\x1b[0m        Check for updates and automatically install the latest release
    \x1b[1;32mversion, -v, --version\x1b[0m Print version information
    \x1b[1;32mhelp, -h, --help\x1b[0m       Print this help message

\x1b[1;33mKEYBOARD SHORTCUTS IN TUI:\x1b[0m
    \x1b[1mTab / 1-4\x1b[0m          Switch between Tasks, Ports, Specs, and Help tabs
    \x1b[1mEnter / d\x1b[0m          Open deep Process Details inspector
    \x1b[1mk / Delete\x1b[0m         Graceful process termination modal (SIGTERM)
    \x1b[1mK (Shift+k)\x1b[0m        Force kill process modal (SIGKILL)
    \x1b[1m/\x1b[0m                  Instant live search filter (Esc to clear)
    \x1b[1mm / c / p / n / o\x1b[0m  Sort by Memory, CPU, PID, Name, or Port
    \x1b[1mr\x1b[0m                  Invert sort direction (High \u{21c4} Low)
    \x1b[1mv\x1b[0m                  Cycle dashboard layout (Combined \u{21c4} Top Rank \u{21c4} Graphs)
    \x1b[1mSpace\x1b[0m              Pause / Resume live metrics updates
    \x1b[1mF5\x1b[0m                 Force refresh all system metrics
    \x1b[1mq / Ctrl+C\x1b[0m         Quit

\x1b[1;33mMOUSE CONTROLS:\x1b[0m
    \x1b[1mClick Tab\x1b[0m          Switch active view
    \x1b[1mClick Column\x1b[0m       Sort table by clicked column header
    \x1b[1mClick Row\x1b[0m          Select process row / open inspector
    \x1b[1mRight-Click Row\x1b[0m    Open terminate process confirmation modal
    \x1b[1mScroll Wheel\x1b[0m       Scroll lists and table views

For documentation, bugs, or feature requests:
\x1b[4mhttps://github.com/Fankrits/TaskTUI\x1b[0m"
    );
}

pub fn run_update() -> Result<(), anyhow::Error> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("\x1b[1;34m⚡ TaskTUI Auto-Updater\x1b[0m");
    println!("Current version: \x1b[1mv{current_version}\x1b[0m");
    println!("Fetching and installing the latest release from GitHub...\n");

    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.sh | sh")
            .status()?;

        if status.success() {
            println!("\n\x1b[1;32m✅ TaskTUI has been successfully updated!\x1b[0m");
        } else {
            eprintln!("\n\x1b[1;31m❌ Update failed with exit code: {:?}\x1b[0m", status.code());
            eprintln!("You can manually update anytime with:");
            eprintln!("  curl -fsSL https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.sh | sh");
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg("irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.ps1 | iex")
            .status()?;

        if status.success() {
            println!("\n\x1b[1;32m✅ TaskTUI has been successfully updated!\x1b[0m");
        } else {
            eprintln!("\n\x1b[1;31m❌ Update failed with exit code: {:?}\x1b[0m", status.code());
            eprintln!("You can manually update anytime with:");
            eprintln!("  irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.ps1 | iex");
        }
    }

    Ok(())
}
