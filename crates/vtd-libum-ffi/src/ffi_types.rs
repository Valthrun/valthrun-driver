use bitflags::bitflags;
use vtd_libum::protocol::{
    command::{
        KeyboardState,
        MouseState,
        VersionInfo,
    },
    types::{
        DirectoryTableType,
        ProcessInfo,
        ProcessModuleInfo,
    },
};

#[repr(C)]
pub struct FfiVersionInfo {
    pub application_name: [u8; 0x20],
    pub version_major: u32,
    pub version_minor: u32,
    pub version_patch: u32,
}

impl From<&VersionInfo> for FfiVersionInfo {
    fn from(value: &VersionInfo) -> Self {
        Self {
            application_name: value.application_name,
            version_major: value.version_major,
            version_minor: value.version_minor,
            version_patch: value.version_patch,
        }
    }
}

bitflags! {
    #[repr(C)]
    pub struct FfiDriverFeature : u64 {
        const PROCESS_LIST               = 0x00_00_00_01;
        const PROCESS_MODULES            = 0x00_00_00_02;
        const PROCESS_PROTECTION_KERNEL   = 0x00_00_00_04;
        const PROCESS_PROTECTION_ZENITH   = 0x00_00_00_08;

        const MEMORY_READ                = 0x00_00_01_00;
        const MEMORY_WRITE               = 0x00_00_02_00;

        const INPUT_KEYBOARD             = 0x00_01_00_00;
        const INPUT_MOUSE                = 0x00_02_00_00;

        const METRICS                   = 0x01_00_00_00;
        const DTT_EXPLICIT               = 0x02_00_10_00;
        const CR3_SSHENANIGANS           = 0x04_00_00_00;
    }
}

#[derive(Clone, Copy)]
#[allow(unused)]
#[repr(C)]
pub enum FfiDirectoryTableType {
    /// Use the process directory table base specified by the system
    Default,

    /// Manually specify the directory table base for the target process
    Explicit { directory_table_base: u64 },

    /// Try to mitigate CR3 shenanigans and do not use the directory table base known to the system
    Cr3Shenanigans,
}

impl Into<DirectoryTableType> for FfiDirectoryTableType {
    fn into(self) -> DirectoryTableType {
        match self {
            Self::Default => DirectoryTableType::Default,
            Self::Explicit {
                directory_table_base,
            } => DirectoryTableType::Explicit {
                directory_table_base,
            },
            Self::Cr3Shenanigans => DirectoryTableType::Cr3Shenanigans,
        }
    }
}

pub type FfiProcessId = u32;

#[repr(C)]
pub struct FfiProcessInfo {
    pub process_id: FfiProcessId,
    pub image_base_name: [u8; 0x0F],
    pub directory_table_base: u64,
}

impl From<&ProcessInfo> for FfiProcessInfo {
    fn from(value: &ProcessInfo) -> Self {
        Self {
            process_id: value.process_id,
            image_base_name: value.image_base_name,
            directory_table_base: value.directory_table_base,
        }
    }
}

#[repr(C)]
pub struct FfiProcessModuleInfo {
    pub base_dll_name: [u8; 0x100],
    pub base_address: u64,
    pub module_size: u64,
}

impl From<&ProcessModuleInfo> for FfiProcessModuleInfo {
    fn from(value: &ProcessModuleInfo) -> Self {
        Self {
            base_dll_name: value.base_dll_name,
            base_address: value.base_address,
            module_size: value.module_size,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FfiKeyboardState {
    pub scane_code: u16,
    pub down: bool,
}

impl Into<KeyboardState> for FfiKeyboardState {
    fn into(self) -> KeyboardState {
        KeyboardState {
            scane_code: self.scane_code,
            down: self.down,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FfiMouseState {
    pub buttons: u8,
    pub buttons_mask: u8,

    pub last_x: i32,
    pub last_y: i32,

    pub hwheel: bool,
    pub wheel: bool,
}

impl Into<MouseState> for FfiMouseState {
    fn into(self) -> MouseState {
        let mut buttons = [None; 5];
        for index in 0..buttons.len() {
            if self.buttons_mask & (1 << index) == 0 {
                continue;
            }

            buttons[index] = Some(self.buttons_mask & (1 << index) > 0);
        }

        MouseState {
            buttons,
            hwheel: self.hwheel,
            wheel: self.wheel,
            last_x: self.last_x,
            last_y: self.last_y,
        }
    }
}
