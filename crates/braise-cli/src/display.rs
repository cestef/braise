pub fn init_log(debug: bool) {
    env_logger::Builder::new()
        .filter_level(if debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Off
        })
        .format_timestamp(None)
        .format_target(false)
        .format_module_path(false)
        .init();
}
