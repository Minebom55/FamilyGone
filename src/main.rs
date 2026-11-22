
use std::{thread, time::Duration};
use sysinfo::{ProcessesToUpdate, System};

const TARGET_PROCESSES: &[&str] = &[
    "WpcUapApp.exe",
];

const CHECK_INTERVAL: u64 = 500;

fn main() {
    let mut system = System::new().expect("Failed to initialize system");

    println!("Process watcher started.");

    loop {
        system.refresh_processes(ProcessesToUpdate::All, true);

        for (pid, process) in system.processes() {
            let name = process.name().to_string_lossy().to_lowercase();

            if TARGET_PROCESSES.iter().any(|target| {
                name == target.to_lowercase()
            }) {
                println!(
                    "Found {} (PID: {}). Terminating...",
                    name,
                    pid
                );

                if process.kill() {
                    println!("Process terminated.");
                } else {
                    println!("Failed to terminate process.");
                }
            }
        }

        thread::sleep(Duration::from_millis(CHECK_INTERVAL));
    }
}
