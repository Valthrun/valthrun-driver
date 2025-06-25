use core::slice;
use std::{
    ffi::{
        c_char,
        CStr,
    },
    ops::{
        Deref,
        DerefMut,
    },
    ptr::NonNull,
};

use log::LevelFilter;
use vtd_libum::{
    self,
    DriverInterface,
    InterfaceError,
};

use crate::ffi_types::{
    FfiDirectoryTableType,
    FfiDriverFeature,
    FfiKeyboardState,
    FfiMouseState,
    FfiProcessId,
    FfiProcessInfo,
    FfiProcessModuleInfo,
    FfiVersionInfo,
};

mod ffi_types;

#[repr(C)]
pub enum VtumStatus {
    Success,

    GeneralFailure,
    Unimplemented,

    NoDriverFound,

    ProcessUnknown,
    MemoryAccessFailed,
    MemoryAccessPagedOut,
}

impl From<InterfaceError> for VtumStatus {
    fn from(value: InterfaceError) -> Self {
        match value {
            InterfaceError::NoDriverFound => VtumStatus::NoDriverFound,

            InterfaceError::ProcessUnknown => VtumStatus::ProcessUnknown,
            InterfaceError::MemoryAccessFailed => VtumStatus::MemoryAccessFailed,
            InterfaceError::MemoryAccessPagedOut => VtumStatus::MemoryAccessPagedOut,
            _ => VtumStatus::GeneralFailure,
        }
    }
}

impl From<Result<(), InterfaceError>> for VtumStatus {
    fn from(value: Result<(), InterfaceError>) -> Self {
        if let Err(value) = value {
            value.into()
        } else {
            VtumStatus::Success
        }
    }
}

/// An opaque handle representing an instance of the Valthrun driver interface.
struct InterfaceHandle(
    /// cbindgen:no-export
    DriverInterface,
);

impl Deref for InterfaceHandle {
    type Target = DriverInterface;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InterfaceHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Initializes the Valthrun Driver Interface C library.
///
/// This function must be called **once** before any other functions in the library are used.
/// It sets up any required global state or resources needed by the driver interface.
///
/// Calling any other function before `library_initialize` results in undefined behavior.
///
/// # Safety
/// The caller must ensure that:
/// - It must be called exactly once before any other use of the library.
/// - Reinitialization or concurrent calls from multiple threads may result in race conditions
///   or undefined behavior unless the implementation guarantees thread safety.
#[no_mangle]
unsafe extern "C" fn vtum_library_initialize() -> VtumStatus {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    VtumStatus::Success
}

/// Shutdown the Valthrun Driver Interface C library.
/// Calls to any other function are not allowed after a call to this.
#[no_mangle]
unsafe extern "C" fn vtum_library_finalize() {
    /* currently this is a no-op */
}

/// Returns the version of the library as a null-terminated C string.
///
/// # Safety
/// The caller must ensure that:
/// - The returned pointer must be treated as read-only.
/// - The pointer must not be deallocated or modified.
///
/// # Returns
/// A pointer to a null-terminated C string containing the version of the library.
#[no_mangle]
unsafe extern "C" fn vtum_library_version() -> *const c_char {
    static LIBRARY_VERSION: &'static str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    LIBRARY_VERSION.as_ptr() as *const c_char
}

/// Creates a new Valthrun driver interface instance with default parameters.
///
/// This function initializes a new `InterfaceHandle` and returns it through the provided
/// mutable reference. The resulting handle must later be destroyed using [`interface_destroy`]
/// to release associated resources.
///
/// # Safety
/// The caller must ensure that:
/// - The caller must provide a valid, writable reference to an uninitialized `InterfaceHandle`.
/// - The memory pointed to by `result` must be valid for writing.
///
/// # Parameters
/// - `result`: A mutable reference to an `InterfaceHandle` that will be initialized by this function.
#[no_mangle]
unsafe extern "C" fn vtum_interface_create(
    mut result: NonNull<*mut InterfaceHandle>,
) -> VtumStatus {
    let interface = match DriverInterface::create_from_env() {
        Ok(interface) => interface,
        Err(error) => {
            log::debug!("Driver interface creation failed: {error}");
            return error.into();
        }
    };

    *result.as_mut() = Box::into_raw(Box::new(InterfaceHandle(interface)));
    VtumStatus::Success
}

/// Destroys and frees all resources associated with an interface handle.
///
/// This function should be called when the interface is no longer needed.
/// It releases any memory, handles, or other system resources allocated during
/// the lifetime of the interface.
///
/// After calling this function, the pointer to the interface must not be used again.
///
/// # Safety
/// The caller must ensure that:
/// - The caller must ensure that `interface` is a valid, non-null pointer to an `InterfaceHandle`.
/// - The handle must not be used after destruction.
/// - Double destruction or passing an invalid pointer results in undefined behavior.
///
/// # Parameters
/// - `interface`: A pointer to a mutable `InterfaceHandle` to be destroyed.
#[no_mangle]
unsafe extern "C" fn vtum_interface_destroy(interface: NonNull<InterfaceHandle>) {
    let _ = Box::from_raw(interface.as_ptr());
}

/// Retrieves the version information of the driver associated with the interface.
///
/// This function fills the provided `VersionInfo` structure with version data
/// such as major, minor, and patch version numbers of the underlying driver.
///
/// # Safety
/// The caller must ensure that:
/// - `interface` points to a valid `InterfaceHandle`.
/// - `version_info` is a valid, mutable reference to an `VersionInfo` structure.
///
/// # Parameters
/// - `interface`: A pointer to a valid `InterfaceHandle`.
/// - `version_info`: A mutable reference to a `VersionInfo` structure that will be populated.
#[no_mangle]
unsafe extern "C" fn vtum_interface_driver_version(
    interface: NonNull<InterfaceHandle>,
    mut version_info: NonNull<FfiVersionInfo>,
) {
    let interface = interface.as_ref();
    *version_info.as_mut() = interface.driver_version().into();
}

/// Returns the feature bitmask supported by the driver.
///
/// This function returns a 64-bit bitmask indicating which optional features
/// are supported by the loaded Valthrun driver implementation.
///
/// Each bit in the returned value represents a specific feature. You can check
/// whether a particular feature is supported using bitwise AND operations with
/// defined driver feature constants.
///
/// # Safety
/// The caller must ensure that:
/// - `interface` must be a valid, non-null pointer to an initialized `InterfaceHandle`.
/// - `features` must be a valid mutable reference to an `DriverFeature` structure.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `features`: Mutable reference to an `DriverFeature` struct that will be filled
///   with the driver's supported features.
#[no_mangle]
unsafe extern "C" fn vtum_interface_driver_features(
    interface: NonNull<InterfaceHandle>,
    mut features: NonNull<FfiDriverFeature>,
) {
    let interface = interface.as_ref();
    *features.as_mut() = FfiDriverFeature::from_bits_retain(interface.driver_features().bits());
}

/// Reads data from the virtual memory of a target process.
///
/// This function allows reading a block of memory from the virtual address space of a
/// specified process into a caller-provided buffer. The read operation is performed in
/// the context of the given page directory table.
///
/// # Safety
/// This function is `unsafe` because it deals with raw pointers and cross-process memory access.
/// The caller must ensure:
/// - `interface` points to a valid `InterfaceHandle`.
/// - `directory_table_type` is a valid reference.
/// - `buffer` points to a writable memory region of at least `buffer_size` bytes.
///
/// # Parameters
/// - `interface`: A pointer to a valid `InterfaceHandle` structure.
/// - `process_id`: The ID of the target process to read memory from.
/// - `directory_table_type`: A reference to the paging context.
/// - `address`: The virtual memory address in the target process to begin reading from.
/// - `buffer`: A pointer to the buffer that will receive the read data.
/// - `buffer_size`: The number of bytes to read.
#[no_mangle]
unsafe extern "C" fn vtum_interface_memory_read(
    interface: NonNull<InterfaceHandle>,
    process_id: FfiProcessId,
    directory_table_type: NonNull<FfiDirectoryTableType>,
    address: u64,
    buffer: NonNull<u8>,
    buffer_size: usize,
) -> VtumStatus {
    let interface = interface.as_ref();
    let buffer = slice::from_raw_parts_mut(buffer.as_ptr(), buffer_size);
    interface
        .read_slice(
            process_id,
            (*directory_table_type.as_ref()).into(),
            address,
            buffer,
        )
        .into()
}

/// Writes data to the virtual memory of a target process.
///
/// This function provides an interface for writing a buffer into the virtual address
/// space of a specified process. The memory write is performed using the paging context
/// described by the given directory table type.
///
/// # Safety
/// The caller must ensure:
/// - `interface` points to a valid `InterfaceHandle`
/// - `directory_table_type` is a valid reference
/// - `buffer` points to a readable memory region of at least `buffer_size` bytes
///
/// # Parameters
/// - `interface`: A pointer to an initialized `InterfaceHandle` structure.
/// - `process_id`: The identifier of the target process whose memory will be written.
/// - `directory_table_type`: A reference to the paging context.
/// - `address`: The virtual memory address in the target process where data will be written.
/// - `buffer`: A pointer to the source data to be written.
/// - `buffer_size`: The size in bytes of the data to write.
#[no_mangle]
unsafe extern "C" fn vtum_interface_memory_write(
    interface: NonNull<InterfaceHandle>,
    process_id: FfiProcessId,
    directory_table_type: NonNull<FfiDirectoryTableType>,
    address: u64,
    buffer: NonNull<u8>,
    buffer_size: usize,
) -> VtumStatus {
    let interface = interface.as_ref();
    let buffer = slice::from_raw_parts(buffer.as_ptr(), buffer_size);
    interface
        .write_slice(
            process_id,
            (*directory_table_type.as_ref()).into(),
            address,
            buffer,
        )
        .into()
}

/// Adds a new metrics record to be submitted by the driver interface.
///
/// This function queues a metrics record identified by a type string along with
/// its associated payload. The record will be processed or submitted asynchronously
/// by the driver or associated subsystem.
///
/// # Safety
/// The caller must ensure:
/// - `interface` must be a valid pointer to an initialized `InterfaceHandle`.
/// - `record_type` and `record_payload` must be valid, null-terminated C strings.
/// - The caller must ensure that the pointers remain valid only for the duration of the call.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `record_type`: Null-terminated C string identifying the type or category of the metric.
/// - `record_payload`: Null-terminated C string containing the metric data or payload.
#[no_mangle]
unsafe extern "C" fn vtum_interface_metrics_add_record(
    interface: NonNull<InterfaceHandle>,
    record_type: NonNull<c_char>,
    record_payload: NonNull<c_char>,
) -> VtumStatus {
    let interface = interface.as_ref();
    let record_type = CStr::from_ptr(record_type.as_ptr());
    let record_payload = CStr::from_ptr(record_payload.as_ptr());

    interface
        .add_metrics_record(
            &*record_type.to_string_lossy(),
            &*record_payload.to_string_lossy(),
        )
        .into()
}

#[no_mangle]
unsafe extern "C" fn vtum_interface_process_toggle_protection(
    _interface: NonNull<InterfaceHandle>,
) -> VtumStatus {
    VtumStatus::Unimplemented
}

/// Lists all processes currently known to the driver interface.
///
/// This function enumerates all active processes and invokes the provided callback
/// for each process with a pointer to a `ProcessInfo` structure describing it.
///
/// The callback should return `true` to continue enumeration, or `false` to stop early.
///
/// # Safety
/// The caller must ensure:
/// - `interface` must be a valid pointer to an initialized `InterfaceHandle`.
/// - The callback function pointer must be valid and callable with a pointer to
///   a valid `ProcessInfo`.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `callback`: A C function pointer that receives a pointer to each `ProcessInfo`.
///   Returning `false` from the callback stops the enumeration early.
#[no_mangle]
unsafe extern "C" fn vtum_interface_process_list(
    interface: NonNull<InterfaceHandle>,
    callback: extern "C" fn(*const FfiProcessInfo) -> bool,
) -> VtumStatus {
    let interface = interface.as_ref();
    let processes = match interface.list_processes() {
        Ok(processes) => processes,
        Err(error) => return error.into(),
    };

    for process in &processes {
        if !callback(&process.into()) {
            break;
        }
    }

    VtumStatus::Success
}

/// Lists all modules loaded by a given process.
///
/// This function enumerates all modules (e.g., DLLs) loaded in the specified
/// process’s address space, using the provided paging context. The `callback` function
/// is invoked for each module with a pointer to an `ProcessModuleInfo` describing it.
///
/// The callback should return `true` to continue enumeration, or `false` to stop early.
///
/// # Safety
/// The caller must ensure:
/// - `interface` must be a valid pointer to an initialized `InterfaceHandle`.
/// - `directory_table_type` must be a valid reference to a paging context.
/// - The callback function pointer must be valid and callable with a pointer to
///   a valid `ProcessModuleInfo`.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `process_id`: The identifier of the target process whose modules will be listed.
/// - `directory_table_type`: Reference to the directory table type (paging context).
/// - `callback`: A C function pointer called with a pointer to each module’s info.
///   Returning `false` stops the enumeration early.
#[no_mangle]
unsafe extern "C" fn vtum_interface_process_module_list(
    interface: NonNull<InterfaceHandle>,
    process_id: FfiProcessId,
    directory_table_type: NonNull<FfiDirectoryTableType>,
    callback: extern "C" fn(*const FfiProcessModuleInfo) -> bool,
) -> VtumStatus {
    let interface = interface.as_ref();
    let modules = match interface.list_modules(process_id, (*directory_table_type.as_ref()).into())
    {
        Ok(processes) => processes,
        Err(error) => return error.into(),
    };

    for module in &modules {
        if !callback(&module.into()) {
            break;
        }
    }

    VtumStatus::Success
}

/// Sends keyboard input events.
///
/// This function injects an array of keyboard state records representing key presses,
/// releases, or other key events via the driver interface.
///
/// # Safety
/// The caller must ensure:
/// - `interface` must be a valid pointer to an initialized `InterfaceHandle`.
/// - `key_states` must point to a valid array of `KeyboardState` with at least `key_states_length` elements.
/// - The data pointed to by `key_states` remains valid for
///   the duration of the call.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `key_states`: Pointer to an array of keyboard state records.
/// - `key_states_length`: Number of elements in the `key_states` array.
#[no_mangle]
unsafe extern "C" fn vtum_interface_input_keyboard(
    interface: NonNull<InterfaceHandle>,
    key_states: NonNull<FfiKeyboardState>,
    key_states_length: usize,
) -> VtumStatus {
    let interface = interface.as_ref();
    let key_states = slice::from_raw_parts(key_states.as_ptr(), key_states_length);
    let key_states = key_states
        .iter()
        .map(|state| (*state).into())
        .collect::<Vec<_>>();

    interface.send_keyboard_state(&key_states).into()
}

/// Sends mouse input events to the target interface.
///
/// This function injects an array of mouse state records representing mouse movements,
/// button clicks, scrolls, or other mouse events to the driver interface.
///
/// # Safety
/// The caller must ensure:
/// - `interface` must be a valid pointer to an initialized `InterfaceHandle`.
/// - `mouse_states` must point to a valid array of `MouseState` with at least `mouse_states_length` elements.
/// - The data pointed to by `mouse_states` remains valid for the duration of the call.
///
/// # Parameters
/// - `interface`: Pointer to a valid `InterfaceHandle`.
/// - `mouse_states`: Pointer to an array of mouse state records.
/// - `mouse_states_length`: Number of elements in the `mouse_states` array.
#[no_mangle]
unsafe extern "C" fn vtum_interface_input_mouse(
    interface: NonNull<InterfaceHandle>,
    mouse_states: NonNull<FfiMouseState>,
    mouse_states_length: usize,
) -> VtumStatus {
    let interface = interface.as_ref();
    let mouse_states = slice::from_raw_parts(mouse_states.as_ptr(), mouse_states_length);
    let mouse_states = mouse_states
        .iter()
        .map(|state| (*state).into())
        .collect::<Vec<_>>();

    interface.send_mouse_state(&mouse_states).into()
}

/// Enables CR3 shenanigan mitigation using the specified strategy and flags.
///
/// This function activates workarounds against CR3-related
/// CPU or OS shenanigans (e.g., tricks involving context switches or paging).
///
/// For more information please visit our Discord server.
#[no_mangle]
unsafe extern "C" fn vtum_interface_cr3_shenanigan_mitigation_enable(
    interface: NonNull<InterfaceHandle>,
    strategy: u32,
    flags: u32,
    mut success: NonNull<bool>,
) -> VtumStatus {
    let interface = interface.as_ref();
    match interface.enable_cr3_shenanigan_mitigation(strategy, flags) {
        Ok(result) => {
            *success.as_mut() = result;
            VtumStatus::Success
        }
        Err(error) => error.into(),
    }
}

/// Disables CR3 shenanigan mitigation previously enabled.
///
/// This function deactivates any active mitigations against
/// CR3-related CPU or OS context-switching tricks.
#[no_mangle]
unsafe extern "C" fn vtum_interface_cr3_shenanigan_mitigation_disable(
    interface: NonNull<InterfaceHandle>,
) -> VtumStatus {
    let interface = interface.as_ref();
    interface.disable_cr3_shenanigan_mitigation().into()
}
