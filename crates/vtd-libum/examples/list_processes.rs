use log::LevelFilter;
use vtd_libum::DriverInterface;

pub fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let interface = DriverInterface::create_from_env()?;
    let processes = interface.list_processes()?;

    log::info!("Process count: {}", processes.len());
    for process in processes {
        log::info!(
            " - {: >5} {} (directory table base = {:X})",
            process.process_id,
            process.get_image_base_name().unwrap_or("<invalid>"),
            process.directory_table_base
        );
    }

    Ok(())
}
