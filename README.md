# Process Watcher

A small Windows system tray app, written in Rust, that watches for specific processes and terminates them as soon as they appear. Defualt is to kill the familysafety that opens randomly.

It runs quietly in the background with no console window, and can be paused, resumed or closed from the tray icon.

## Features

- Monitors running processes and kills any whose name matches a configurable list
- Process names are matched case-insensitively
- Checks every 500 ms by default
- Tray icon with a right-click menu:
  - **Expand**: show the main window
  - **Pause / Resume monitoring**
  - **Exit**
- Small main window with a status label, a pause/resume button and a **Hide to tray** button
- Closing the window only hides it; the app keeps running in the tray
- Uses a built-in Windows icon loaded from `imageres.dll`, so no extra files are needed

## Requirements

- Windows 10 or 11
- [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (the project uses the 2024 edition)

## Building

```powershell
git clone <your-repository-url>
cd process_watcher
cargo build --release
```

The executable is created at `target\release\process_watcher.exe`.

## Configuration

Settings are constants at the top of `src/main.rs`:

```rust
const TARGET_PROCESSES: &[&str] = &[
    "WpcUapApp.exe",
];

const CHECK_INTERVAL: u64 = 500; // milliseconds between checks
```

Add the executable names you want killed to `TARGET_PROCESSES`, then rebuild.

The tray and window icon is set by `ICON_DLL` and `ICON_ID`, which are the DLL name and icon resource ID (as in `imageres.dll,-1027`). Change them to use a different built-in icon.

## Running

Run `process_watcher.exe`. There is no console window, so look for the icon in the system tray (it may be under the `^` overflow arrow). Right-click it for the menu.

To stop the app completely, choose **Exit** from the tray menu. Closing the window only hides it.

## Start automatically with Task Scheduler

Instead of the Startup folder, this project is meant to be started at login with **Windows Task Scheduler**. A scheduled task can run the app with highest privileges without showing a UAC prompt every time you log in, which is needed if the process you want to kill is running elevated.

First, copy `process_watcher.exe` to a permanent location, for example `C:\Tools\process_watcher\`. Don't point the task at `target\release`, because `cargo clean` deletes it.

### Option A: PowerShell (recommended)

Open **PowerShell as Administrator** and run (adjust the path):

```powershell
$exe       = "C:\Tools\process_watcher\process_watcher.exe"
$action    = New-ScheduledTaskAction -Execute $exe
$trigger   = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest
$settings  = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit ([TimeSpan]::Zero) `
    -MultipleInstances IgnoreNew

Register-ScheduledTask -TaskName "ProcessWatcher" `
    -Action $action -Trigger $trigger `
    -Principal $principal -Settings $settings
```

This creates a task that:

- starts the app every time you **log on**
- runs with **highest privileges**
- keeps running on battery power
- is never stopped by the default "stop after 3 days" limit

To start it right away without logging out again:

```powershell
Start-ScheduledTask -TaskName "ProcessWatcher"
```

### Option B: Task Scheduler GUI

1. Open **Task Scheduler** (press `Win`, type "Task Scheduler").
2. Click **Create Task...** in the right-hand panel (not "Create Basic Task").
3. **General** tab:
   - Name: `ProcessWatcher`
   - Select **Run only when user is logged on**
   - Tick **Run with highest privileges**
4. **Triggers** tab: **New...**, set *Begin the task* to **At log on**, and choose your user.
5. **Actions** tab: **New...**, set *Action* to **Start a program**, and browse to `process_watcher.exe`.
6. **Conditions** tab: untick **Start the task only if the computer is on AC power** (important on laptops).
7. **Settings** tab:
   - Untick **Stop the task if it runs longer than 3 days**. This default would silently kill the watcher after three days.
   - Set *If the task is already running* to **Do not start a new instance**.
8. Click **OK**.

### Why "At log on" and not "At startup"?

The app has a tray icon and a window, so it needs your interactive desktop session. A task that runs at system startup would run in a session without a desktop and the tray icon would never appear.

### Notes

- The app has no built-in single-instance check. If the scheduled task is already running, exit it from the tray before launching the exe manually, otherwise you'll have two copies.
- Killing processes that belong to other users or to the system requires administrator rights, which is why the task uses highest privileges.
- To remove the autostart:

  ```powershell
  Unregister-ScheduledTask -TaskName "ProcessWatcher" -Confirm:$false
  ```

## How it works

A background thread refreshes the process list every `CHECK_INTERVAL` milliseconds using [`sysinfo`](https://crates.io/crates/sysinfo) and terminates any process whose name matches `TARGET_PROCESSES`. The tray icon and window are built with [`native-windows-gui`](https://crates.io/crates/native-windows-gui). Pausing flips a shared atomic flag that the monitor thread checks on every loop.

## Disclaimer

This tool forcibly terminates processes. Only add programs you are sure you want killed, and avoid critical system processes.
