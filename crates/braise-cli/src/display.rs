pub fn init_log(debug: bool) {
    let mut binding = env_logger::Builder::from_env("BRAISE_LOG");
    let mut builder = binding
        .format_timestamp(None)
        .format_target(false)
        .format_module_path(false)
        .filter_module("sled", log::LevelFilter::Warn);
    if debug {
        builder = builder.filter_level(log::LevelFilter::Debug);
    }
    builder.init();
}
