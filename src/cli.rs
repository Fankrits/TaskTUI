use std::io::{self, Write};
use std::path::{Path, PathBuf};
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
    \x1b[1;32mupdate, upgrade\x1b[0m                Check for updates and automatically install the latest release
    \x1b[1;32muninstall, --uninstall [-y]\x1b[0m    Completely remove TaskTUI binaries from your system
    \x1b[1;32mversion, -v, --version\x1b[0m         Print version information
    \x1b[1;32mhelp, -h, --help\x1b[0m               Print this help message

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
            eprintln!(
                "\n\x1b[1;31m❌ Update failed with exit code: {:?}\x1b[0m",
                status.code()
            );
            eprintln!("You can manually update anytime with:");
            eprintln!(
                "  curl -fsSL https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.sh | sh"
            );
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
            eprintln!(
                "\n\x1b[1;31m❌ Update failed with exit code: {:?}\x1b[0m",
                status.code()
            );
            eprintln!("You can manually update anytime with:");
            eprintln!(
                "  irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.ps1 | iex"
            );
        }
    }

    Ok(())
}

pub fn run_uninstall(skip_confirm: bool) -> Result<(), anyhow::Error> {
    println!("\x1b[1;34m⚡ TaskTUI Uninstaller\x1b[0m");
    println!("Scanning system for TaskTUI installations...\n");

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Current executable
    if let Ok(exe_path) = std::env::current_exe() {
        candidates.push(exe_path);
    }

    // 2. Standard Unix / macOS install paths
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(PathBuf::from(&home).join(".local/bin/tasktui"));
            candidates.push(PathBuf::from(&home).join(".local/bin/task_tui"));
            candidates.push(PathBuf::from(&home).join(".cargo/bin/tasktui"));
            candidates.push(PathBuf::from(&home).join(".cargo/bin/task_tui"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/tasktui"));
        candidates.push(PathBuf::from("/usr/local/bin/task_tui"));
    }

    // 3. Standard Windows install paths
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local_app_data).join("Programs\\TaskTUI\\tasktui.exe"));
            candidates.push(PathBuf::from(&local_app_data).join("Programs\\TaskTUI\\task_tui.exe"));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            candidates.push(PathBuf::from(&user_profile).join(".local\\bin\\tasktui.exe"));
            candidates.push(PathBuf::from(&user_profile).join(".local\\bin\\task_tui.exe"));
            candidates.push(PathBuf::from(&user_profile).join(".cargo\\bin\\tasktui.exe"));
            candidates.push(PathBuf::from(&user_profile).join(".cargo\\bin\\task_tui.exe"));
        }
    }

    // Deduplicate and filter to existing files
    let mut targets_to_remove: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        if candidate.exists() && !targets_to_remove.contains(&candidate) {
            targets_to_remove.push(candidate);
        }
    }

    if targets_to_remove.is_empty() {
        println!(
            "\x1b[1;33mNotice:\x1b[0m No TaskTUI binaries found in standard installation paths."
        );
        return Ok(());
    }

    println!("The following items will be removed:");
    for path in &targets_to_remove {
        println!("  \x1b[1;31m•\x1b[0m {}", path.display());
    }
    println!();

    if !skip_confirm {
        print!("Are you sure you want to uninstall TaskTUI? [\x1b[1my\x1b[0m/\x1b[1mN\x1b[0m]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            println!("\nUninstall canceled.");
            return Ok(());
        }
        println!();
    }

    let mut removed_count = 0;
    for path in &targets_to_remove {
        match remove_target_file(path) {
            Ok(true) => {
                println!("  \x1b[1;32m✔\x1b[0m Removed {}", path.display());
                removed_count += 1;
            }
            Ok(false) => {
                println!(
                    "  \x1b[1;33m-\x1b[0m Skipped (already removed) {}",
                    path.display()
                );
            }
            Err(e) => {
                eprintln!(
                    "  \x1b[1;31m✖\x1b[0m Failed to remove {}: {e}",
                    path.display()
                );
            }
        }
    }

    // Clean up empty directory on Windows if present
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let app_dir = PathBuf::from(&local_app_data).join("Programs\\TaskTUI");
            if app_dir.exists() {
                let _ = std::fs::remove_dir(&app_dir);
            }
        }
    }

    if removed_count > 0 {
        println!("\n\x1b[1;32m🎉 TaskTUI was successfully uninstalled from your system.\x1b[0m");
    } else {
        println!("\n\x1b[1;33mNo files were removed.\x1b[0m");
    }

    Ok(())
}

fn remove_target_file(path: &Path) -> Result<bool, anyhow::Error> {
    if !path.exists() {
        return Ok(false);
    }

    if let Err(err) = std::fs::remove_file(path) {
        #[cfg(not(target_os = "windows"))]
        {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                println!(
                    "  \x1b[33mElevated permission needed for {}, trying sudo...\x1b[0m",
                    path.display()
                );
                let status = Command::new("sudo")
                    .arg("rm")
                    .arg("-f")
                    .arg(path)
                    .status()?;
                if status.success() {
                    return Ok(true);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, if running binary is locked, schedule removal right after process exits
            let escaped_path = path.display().to_string();
            let _ = Command::new("powershell")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(format!("Start-Sleep -Milliseconds 800; Remove-Item -Force '{escaped_path}' -ErrorAction SilentlyContinue"))
                .spawn();
            return Ok(true);
        }

        return Err(err.into());
    }

    Ok(true)
}
