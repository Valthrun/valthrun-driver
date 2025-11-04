#![feature(inherent_str_constructors)]

use core::slice;

use lazy_static::lazy_static;
use registry::HandlerRegistry;

mod handle;
mod handler;
mod metrics;
mod registry;
mod util;

lazy_static! {
    static ref REQUEST_HANDLER: HandlerRegistry = init_request_handler();
}

fn init_request_handler() -> HandlerRegistry {
    let mut handler = HandlerRegistry::new();

    handler.register(&handler::init);
    handler.register(&handler::get_processes);
    handler.register(&handler::get_modules);
    handler.register(&handler::read);
    handler.register(&handler::mouse_move);
    handler.register(&handler::keyboard_state);
    handler.register(&handler::metrics_report_send);
    handler.register(&handler::metrics_flush);

    handler
}

#[no_mangle]
unsafe extern "C" fn startup() {
    env_logger::init();

    use crate::metrics;
    metrics::maybe_init();
}

#[no_mangle]
unsafe extern "C" fn teardown() {
    use crate::metrics;
    metrics::shutdown();
}

#[no_mangle]
#[allow(non_snake_case)]
extern "system" fn DllMain(_dll_module: *const (), _call_reason: u32, _: *mut ()) -> bool {
    true
}

#[no_mangle]
extern "C" fn execute_command(
    command_id: u32,

    payload: *mut u8,
    payload_length: usize,

    error_message: *mut u8,
    error_message_length: usize,
) -> u64 {
    let payload = unsafe { slice::from_raw_parts_mut(payload, payload_length) };
    let error_message = unsafe { slice::from_raw_parts_mut(error_message, error_message_length) };

    REQUEST_HANDLER
        .handle(command_id, payload, error_message)
        .bits()
}
