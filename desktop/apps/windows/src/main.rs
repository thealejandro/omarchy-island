// No console window in release builds; debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The window, media and lyrics come in the next phases (#64).
    #[cfg(debug_assertions)]
    println!("island {}", island_core::VERSION);
}
