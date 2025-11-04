use std::mem;

use windows::Win32::{
    Foundation::GetLastError,
    System::SystemInformation::{
        GetSystemFirmwareTable,
        GetVersionExA,
        OSVERSIONINFOEXA,
        RSMB,
    },
};

use crate::{
    data::{
        DeviceInfo,
        Win32DeviceInfo,
    },
    error::MetricsResult,
};

fn get_bios_unique_id() -> Option<String> {
    let table_size = unsafe {
        let result = GetSystemFirmwareTable(RSMB, 0, None);
        if result == 0 {
            log::warn!("Failed to get RSMB table length: 0x{:X}", GetLastError().0);
            return None;
        }

        result as usize
    };

    let mut buffer = Vec::<u8>::new();
    buffer.resize(table_size, 0);
    let table_size = unsafe {
        let result = GetSystemFirmwareTable(RSMB, 0, Some(&mut buffer));
        if result == 0 {
            log::warn!("Failed to get RSMB table: 0x{:X}", GetLastError().0);
            return None;
        }

        result as usize
    };

    let mut offset = 0x08; // 0x08 = sizeof(RawSMBIOSData)
    while offset + 4 < table_size {
        let table_type = buffer[offset];
        let table_length = buffer[offset + 1];
        if table_length < 4 {
            break;
        }

        if table_type != 0x01 || table_length < 0x19 {
            offset += table_length as usize;

            /* skip over unformatted area */
            while offset + 2 < table_size {
                if u16::from_be_bytes(buffer[offset..offset + 2].try_into().unwrap()) == 0 {
                    /* marker found */
                    break;
                }

                offset += 1;
            }
            offset += 2;
            continue;
        }

        /* bios uuid found */
        offset += 0x08; // UUID offset

        /*
         * Note:
         * As off version 2.6 of the SMBIOS specification, the first 3 fields of the UUID are supposed to be encoded on little-endian. (para 7.2.1)
         * We ignore this here, asd it's still unique, just not in a proper uuid format.
         */
        return Some(hex::encode(&buffer[offset..offset + 16]));
    }

    None
}

fn get_ubr() -> Option<u32> {
    None
}

pub fn resolve_info() -> MetricsResult<DeviceInfo> {
    let mut winver = OSVERSIONINFOEXA::default();
    winver.dwOSVersionInfoSize = mem::size_of_val(&winver) as u32;
    let _ = unsafe { GetVersionExA(&mut winver as *mut _ as *mut _) };

    Ok(DeviceInfo::Win32(Win32DeviceInfo {
        bios_uuid: self::get_bios_unique_id(),

        win_major_version: winver.dwMajorVersion,
        win_minor_version: winver.dwMinorVersion,
        win_build_no: winver.dwBuildNumber,
        win_platform_id: winver.dwPlatformId,
        win_unique_build_number: self::get_ubr().unwrap_or_default(),

        win_service_pack_major: winver.wServicePackMajor,
        win_service_pack_minor: winver.wServicePackMinor,

        win_suite_mask: winver.wSuiteMask,
        win_product_type: winver.wProductType,
    }))
}
