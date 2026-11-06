#[cfg(not(all(target_os = "macos", feature = "macos")))]
fn main() {
    eprintln!("table-demo requires --features macos on macOS");
}

#[cfg(all(target_os = "macos", feature = "macos"))]
fn main() {
    eprintln!("table-demo: Phase A in progress — not yet runnable");
}
