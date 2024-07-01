use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use paris::error;

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");

pub fn init_panic() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};

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
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(1);
    }));
    Ok(())
}

pub fn version() -> String {
    let author = clap::crate_authors!();

    let config_dir_path = dirs::home_dir()
        .map(|p| p.join(".config").join(clap::crate_name!()))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let author = author.replace(':', ", ");
    let hash = GIT_COMMIT_HASH.bold();
    format!(
        "\
{hash}

Authors: {}

Config directory: {}",
        author.dimmed().bold(),
        config_dir_path.dimmed().bold()
    )
}
