use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use log::{debug, trace};

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");

pub fn init_panic() -> Result<()> {
    trace!("init_panic: entering");
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    trace!("init_panic: installing eyre hook");
    eyre_hook.install()?;
    trace!("init_panic: installing panic hook");
    std::panic::set_hook(Box::new(move |panic_info| {
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            trace!("init_panic: human-panic hook");
            let meta = Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
                .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", "))
                .homepage(env!("CARGO_PKG_HOMEPAGE"));

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        println!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            trace!("init_panic: better-panic hook");
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(1);
    }));
    trace!("init_panic: hooks installed");
    trace!("init_panic: exiting");
    Ok(())
}

pub fn build_logger() -> pretty_env_logger::env_logger::Builder {
    trace!("build_logger: entering");
    let builder = pretty_env_logger::formatted_builder();
    trace!("build_logger: exiting");
    builder
}

pub fn version() -> String {
    trace!("version: entering");
    let author = clap::crate_authors!();
    debug!("version: raw_author: {}", author);
    let config_dir_path = dirs::home_dir()
        .map(|p| p.join(".config").join(clap::crate_name!()))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let author = author.replace(':', ", ");
    let hash = GIT_COMMIT_HASH.bold();
    trace!("version: exiting");
    format!(
        "\
{hash}

Authors: {}

Config directory: {}",
        author.dimmed().bold(),
        config_dir_path.dimmed().bold()
    )
}
