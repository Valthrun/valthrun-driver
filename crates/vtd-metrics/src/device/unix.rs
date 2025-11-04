use std::{
    fs,
    process::Command,
};

use uuid::Uuid;

use crate::{
    data::{
        DeviceInfo,
        UnixDeviceInfo,
    },
    error::MetricsResult,
};

fn is_wsl() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok()
}

fn wsl_get_bios_unique_id() -> Option<String> {
    let output = Command::new("powershell.exe")
        .args([
            "-Command",
            "(Get-CimInstance Win32_ComputerSystemProduct).UUID",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    Uuid::parse_str(trimmed)
        .ok()
        .map(|uuid| hex::encode(&uuid.to_bytes_le()))
}

fn dmi_get_product_unique_id() -> Option<String> {
    let path = "/sys/class/dmi/id/product_uuid";
    let contents = fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    Uuid::parse_str(trimmed)
        .ok()
        .map(|uuid| hex::encode(&uuid.to_bytes_le()))
}

fn get_bios_unique_id() -> Option<String> {
    if self::is_wsl() {
        None.or_else(self::wsl_get_bios_unique_id)
            .or_else(self::dmi_get_product_unique_id)
    } else {
        None.or_else(self::dmi_get_product_unique_id)
    }
}

fn uname() -> Option<String> {
    let output = Command::new("uname").args(["-a"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.len() > 128 {
        Some(stdout[0..128].to_string())
    } else {
        Some(stdout.to_string())
    }
}

pub fn resolve_info() -> MetricsResult<DeviceInfo> {
    Ok(DeviceInfo::Unix(UnixDeviceInfo {
        bios_uuid: self::get_bios_unique_id(),
        uname: self::uname(),
    }))
}
