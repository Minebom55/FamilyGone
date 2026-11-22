#![windows_subsystem = "windows"]
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use sysinfo::{ProcessesToUpdate, System};

use native_windows_gui as nwg;

use std::rc::Rc;

const TARGET_PROCESSES: &[&str] = &[
    "WpcUapApp.exe",
];

const CHECK_INTERVAL: u64 = 500;

fn start_process_monitor(
    paused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut system = System::new()
            .expect("Failed to initialize sysinfo");

        println!("Process monitor started.");

        while !stop.load(Ordering::SeqCst) {
            if !paused.load(Ordering::SeqCst) {
                system.refresh_processes(
                    ProcessesToUpdate::All,
                    true,
                );

                for (pid, process) in system.processes() {
                    let name = process
                        .name()
                        .to_string_lossy();

                    let matches = TARGET_PROCESSES
                        .iter()
                        .any(|target| name.eq_ignore_ascii_case(target));

                    if matches {
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
            }

            thread::sleep(Duration::from_millis(CHECK_INTERVAL));
        }

        println!("Process monitor stopped.");
    })
}

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::{FreeLibrary, LoadLibraryExW, LOAD_LIBRARY_AS_DATAFILE};
use winapi::um::sysinfoapi::GetSystemDirectoryW;
use winapi::um::winnt::HANDLE;
use winapi::um::winuser::{
    GetSystemMetrics, LoadImageW, IMAGE_ICON, LR_DEFAULTCOLOR, SM_CXICON, SM_CXSMICON,
    SM_CYICON, SM_CYSMICON,
};

const ICON_DLL: &str = "imageres.dll";
const ICON_ID: u16 = 1027; // the number from "imageres.dll,-1027"

fn system_dll_path(dll_name: &str) -> Vec<u16> {
    let mut buf = [0u16; 260];
    let len = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    let mut path: Vec<u16> = buf[..len as usize].to_vec();
    path.push('\\' as u16);
    for c in OsStr::new(dll_name).encode_wide() {
        path.push(c);
    }
    path.push(0);
    path
}

/// Loads icon `resource_id` from a system DLL at an exact pixel size.
fn load_icon_from_dll(dll_name: &str, resource_id: u16, width: i32, height: i32) -> nwg::Icon {
    let path = system_dll_path(dll_name);

    let hicon: HANDLE = unsafe {
        let module = LoadLibraryExW(path.as_ptr(), ptr::null_mut(), LOAD_LIBRARY_AS_DATAFILE);
        if module.is_null() {
            ptr::null_mut()
        } else {
            let handle = LoadImageW(
                module,
                resource_id as usize as *const u16, // MAKEINTRESOURCE
                IMAGE_ICON,
                width,
                height,
                LR_DEFAULTCOLOR,
            );
            FreeLibrary(module); // the icon is an independent copy, safe to unload
            handle
        }
    };

    if hicon.is_null() {
        // Visible fallback (blue "i"), since eprintln! is invisible in a windows-subsystem app
        return nwg::Icon::from_system(nwg::OemIcon::Information);
    }

    let mut icon = nwg::Icon::default();
    icon.handle = hicon;
    icon
}

fn toggle_monitoring(
    paused: &AtomicBool,
    button: &nwg::Button,
    status: &nwg::Label,
) {
    let was_paused = paused.fetch_xor(true, Ordering::SeqCst);
    let now_paused = !was_paused;

    if now_paused {
        button.set_text("Resume monitoring");
        status.set_text("Monitoring is paused");
    } else {
        button.set_text("Pause monitoring");
        status.set_text("Monitoring is running");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared monitoring state
    let paused = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    // Start background process monitoring
    let monitor = start_process_monitor(
        Arc::clone(&paused),
        Arc::clone(&stop),
    );

    // Initialize Windows GUI
    nwg::init()?;

    nwg::Font::set_global_family("Segoe UI")?;
    
    // Tray icon
    let (small_w, small_h, big_w, big_h) = unsafe {
    (
        GetSystemMetrics(SM_CXSMICON),
        GetSystemMetrics(SM_CYSMICON),
        GetSystemMetrics(SM_CXICON),
        GetSystemMetrics(SM_CYICON),
    )
};
let tray_icon = load_icon_from_dll(ICON_DLL, ICON_ID, small_w, small_h);
let window_icon = load_icon_from_dll(ICON_DLL, ICON_ID, big_w, big_h);

    

    let mut message_window = nwg::MessageWindow::default();

    nwg::MessageWindow::builder()
        .build(&mut message_window)?;

    let mut tray = nwg::TrayNotification::default();

    nwg::TrayNotification::builder()
     .parent(&message_window)
     .icon(Some(&tray_icon))
     .tip(Some("Process Watcher"))
     .build(&mut tray)?;

    // Tray context menu
    let mut tray_menu = nwg::Menu::default();

    nwg::Menu::builder()
        .popup(true)
        .parent(&message_window)
        .build(&mut tray_menu)?;

    let mut expand_item = nwg::MenuItem::default();
    let mut toggle_item = nwg::MenuItem::default();
    let mut exit_item = nwg::MenuItem::default();

    nwg::MenuItem::builder()
        .text("Expand")
        .parent(&tray_menu)
        .build(&mut expand_item)?;

    nwg::MenuItem::builder()
        .text("Pause / Resume monitoring")
        .parent(&tray_menu)
        .build(&mut toggle_item)?;

    nwg::MenuItem::builder()
        .text("Exit")
        .parent(&tray_menu)
        .build(&mut exit_item)?;

    // Main application window (initially hidden)
    let mut window = nwg::Window::default();

    nwg::Window::builder()
     .flags(nwg::WindowFlags::WINDOW)
     .size((360, 190))
     .position((500, 300))
     .title("Process Watcher")
     .icon(Some(&window_icon))
     .build(&mut window)?;

    let mut status = nwg::Label::default();

    nwg::Label::builder()
        .text("Monitoring is running")
        .position((20, 20))
        .size((320, 25))
        .parent(&window)
        .build(&mut status)?;

    let mut description = nwg::Label::default();

    nwg::Label::builder()
        .text("Watching configured process names.")
        .position((20, 55))
        .size((320, 25))
        .parent(&window)
        .build(&mut description)?;

    let mut pause_button = nwg::Button::default();

    nwg::Button::builder()
        .text("Pause monitoring")
        .position((20, 100))
        .size((150, 35))
        .parent(&window)
        .build(&mut pause_button)?;

    let mut minimize_button = nwg::Button::default();

    nwg::Button::builder()
        .text("Hide to tray")
        .position((185, 100))
        .size((150, 35))
        .parent(&window)
        .build(&mut minimize_button)?;

    // ControlHandle is Copy. Grab the handles now so the event closures can
    // compare against them without borrowing (or moving) the controls.
    // The controls themselves stay alive in this function until shutdown.
    let window_handle = window.handle;
    let pause_handle = pause_button.handle;
    let hide_handle = minimize_button.handle;
    let tray_handle = tray.handle;
    let expand_handle = expand_item.handle;
    let toggle_handle = toggle_item.handle;
    let exit_handle = exit_item.handle;

    // Shared UI references
    let window = Rc::new(window);
    let status = Rc::new(status);
    let pause_button = Rc::new(pause_button);

    // Tray event handler
    let tray_events = {
        let window = Rc::clone(&window);
        let status = Rc::clone(&status);
        let pause_button = Rc::clone(&pause_button);

        let paused = Arc::clone(&paused);
        let stop = Arc::clone(&stop);

        nwg::full_bind_event_handler(
            &message_window.handle,
            move |evt, _evt_data, handle| {
                match evt {
                    nwg::Event::OnContextMenu => {
                        if handle == tray_handle {
                            let (x, y) = nwg::GlobalCursor::position();
                            tray_menu.popup(x, y);
                        }
                    }

                    nwg::Event::OnMenuItemSelected => {
                        if handle == expand_handle {
                            window.set_visible(true);
                            window.set_focus();
                        }

                        if handle == toggle_handle {
                            toggle_monitoring(
                                &paused,
                                &pause_button,
                                &status,
                            );
                        }

                        if handle == exit_handle {
                            stop.store(true, Ordering::SeqCst);
                            nwg::stop_thread_dispatch();
                        }
                    }

                    _ => {}
                }
            },
        )
    };

    // Window event handler: close (hides instead), pause button, hide button
    let window_events = {
        let window = Rc::clone(&window);
        let status = Rc::clone(&status);
        let pause_button = Rc::clone(&pause_button);
        let paused = Arc::clone(&paused);

        nwg::full_bind_event_handler(
            &window_handle,
            move |evt, evt_data, handle| {
                match evt {
                    nwg::Event::OnWindowClose => {
                        if handle == window_handle {
                            // Cancel the close and just hide the window
                            if let nwg::EventData::OnWindowClose(close_data) = &evt_data {
                                close_data.close(false);
                            }

                            window.set_visible(false);
                        }
                    }

                    nwg::Event::OnButtonClick => {
                        if handle == pause_handle {
                            toggle_monitoring(
                                &paused,
                                &pause_button,
                                &status,
                            );
                        } else if handle == hide_handle {
                            window.set_visible(false);
                        }
                    }

                    _ => {}
                }
            },
        )
    };

    println!("Process Watcher is running in the system tray.");

    // Start Windows message loop
    nwg::dispatch_thread_events();

    // Shutdown
    stop.store(true, Ordering::SeqCst);

    let _ = monitor.join();

    nwg::unbind_event_handler(&tray_events);
    nwg::unbind_event_handler(&window_events);

    Ok(())
}