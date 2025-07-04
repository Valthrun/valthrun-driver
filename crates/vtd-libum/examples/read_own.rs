use std::mem;

use log::LevelFilter;
use rand::Rng;
use vtd_libum::DriverInterface;
use vtd_protocol::types::DirectoryTableType;

fn read_heap_buffer(interface: &DriverInterface) -> anyhow::Result<()> {
    let mut buffer = Vec::with_capacity(10_000);
    buffer.resize(10_000, 0);

    for (index, entry) in buffer.iter_mut().enumerate() {
        *entry = index;
    }

    let mut read_buffer = Vec::new();
    read_buffer.resize(buffer.len(), 0usize);
    interface.read_slice::<usize>(
        std::process::id(),
        DirectoryTableType::Default,
        buffer.as_ptr() as u64,
        &mut read_buffer,
    )?;

    if buffer == read_buffer {
        log::info!("Read head buffer successfull");
    } else {
        assert_eq!(buffer, read_buffer);
        log::info!("Full heap buffer read failed!");
    }

    Ok(())
}

pub fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let interface = DriverInterface::create_from_env()?;

    let target_value = 0x42u64;
    let read_value = interface.read::<u64>(
        std::process::id(),
        DirectoryTableType::Default,
        &target_value as *const _ as u64,
    );

    log::info!("Read result: {:X?}", read_value);
    read_heap_buffer(&interface)?;

    let own_process_id = std::process::id();
    let mut rng = rand::thread_rng();
    for _ in 0..10_000 {
        let mut src_buffer = Vec::new();
        src_buffer.resize_with(rng.gen_range(1..128_000) as usize, || rng.gen::<u8>());

        let mut dst_buffer = Vec::<u8>::new();
        dst_buffer.resize(src_buffer.len(), 0);
        let result = interface.read_slice(
            own_process_id,
            DirectoryTableType::Default,
            src_buffer.as_ptr() as u64,
            dst_buffer.as_mut_slice(),
        );

        if dst_buffer != src_buffer || result.is_err() {
            log::info!("Result: {:#?}", result);
            log::info!(
                "Read failed :( Buffer size: {}, src = {:X}, dst = {:X}",
                src_buffer.len(),
                src_buffer.as_ptr() as u64,
                dst_buffer.as_ptr() as u64
            );
        }
        mem::forget(dst_buffer);
    }
    log::info!("Done :)");
    Ok(())
}
