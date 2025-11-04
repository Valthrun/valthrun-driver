#[cfg(not(windows))]
mod unix;

#[cfg(not(windows))]
pub use unix::resolve_info;

#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub use win32::resolve_info;

#[cfg(windows)]
pub fn get_tick_count64() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount64;

    unsafe { GetTickCount64() }
}

#[cfg(unix)]
pub fn get_tick_count64() -> u64 {
    use libc::{
        CLOCK_MONOTONIC,
        clock_gettime,
        timespec,
    };

    unsafe {
        let mut ts: timespec = std::mem::zeroed();
        if clock_gettime(CLOCK_MONOTONIC, &mut ts) == 0 {
            (ts.tv_sec as u64 * 1000) + (ts.tv_nsec as u64 / 1_000_000)
        } else {
            0
        }
    }
}
