use log::LevelFilter;
use vtd_libum::DriverInterface;
use vtd_protocol::types::DirectoryTableType;

pub fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let interface = DriverInterface::create_from_env()?;

    let target_value = 0x01u64;
    interface.write::<u64>(
        std::process::id(),
        DirectoryTableType::Default,
        &target_value as *const _ as u64,
        &0x42,
    )?;

    log::info!("Target value: {:X}", target_value);
    Ok(())
}
