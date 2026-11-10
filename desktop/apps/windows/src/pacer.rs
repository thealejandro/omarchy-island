//! Frames in step with the screen. While the island moves, a thread waits
//! for each composition (`DwmFlush`, once per refresh) and asks the window
//! for a frame; a USER timer fires on a coarse 15.6 ms tick that drifts
//! against the refresh, so frames came uneven. When the island is still,
//! the thread sleeps on a condition variable: no wake-ups at rest.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

struct Shared {
    running: Mutex<bool>,
    wake: Condvar,
    /// A frame asked for and not drawn yet: never more than one queued.
    pending: AtomicBool,
}

pub struct Pacer {
    shared: Arc<Shared>,
}

impl Pacer {
    /// Starts the (sleeping) thread; it posts `msg` to `hwnd` per frame.
    pub fn new(hwnd: HWND, msg: u32) -> Self {
        let shared = Arc::new(Shared {
            running: Mutex::new(false),
            wake: Condvar::new(),
            pending: AtomicBool::new(false),
        });
        let worker = shared.clone();
        let hwnd = hwnd.0 as isize;
        let _ = std::thread::Builder::new()
            .name("frames".into())
            .spawn(move || run(&worker, hwnd, msg));
        Self { shared }
    }

    pub fn start(&self) {
        if let Ok(mut on) = self.shared.running.lock() {
            *on = true;
            self.shared.wake.notify_one();
        }
    }

    pub fn stop(&self) {
        if let Ok(mut on) = self.shared.running.lock() {
            *on = false;
        }
    }

    /// The frame asked for was drawn; the next one may be asked.
    pub fn drawn(&self) {
        self.shared.pending.store(false, Ordering::Release);
    }
}

fn run(s: &Shared, hwnd: isize, msg: u32) {
    loop {
        {
            let Ok(mut on) = s.running.lock() else { return };
            while !*on {
                on = match s.wake.wait(on) {
                    Ok(g) => g,
                    Err(_) => return,
                };
            }
        }
        // Without composition (it can't be off on Windows 10 and 11, but
        // still): about one refresh.
        if unsafe { DwmFlush() }.is_err() {
            std::thread::sleep(Duration::from_millis(16));
        }
        if !s.pending.swap(true, Ordering::AcqRel) {
            let posted =
                unsafe { PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0)) };
            if posted.is_err() {
                // The window is gone.
                return;
            }
        }
    }
}
