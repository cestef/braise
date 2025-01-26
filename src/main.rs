use std::{
    ffi::OsString,
    path::Path,
    sync::Arc,
    thread::{spawn, JoinHandle},
};

use braise::{
    cli,
    constants::{DEFAULT_CONFIG, TASKS_SEPARATOR},
    env,
    error::BraiseError,
    file::BraiseFile,
    task,
    utils::{build_logger, confirm_action, init_panic},
};

use color_eyre::{
    eyre::{bail, eyre, Result},
    owo_colors::OwoColorize,
};

use log::{debug, trace};

// Handle initialization of a new Braise file
fn handle_init(path: &str) -> Result<()> {
    trace!("main: initializing");
    let mut name = "braise.toml".to_string();

    if let Ok(path) = BraiseFile::find_path() {
        println!("The Braisefile already exists at {}", path.bold());
        if !confirm_action("Do you want to overwrite it? [y/N]")? {
            println!("Exiting...");
            return Ok(());
        }
        name = path;
    }

    let file_path = Path::new(path);
    let joined = file_path.join(name);
    std::fs::write(&joined, DEFAULT_CONFIG)?;
    println!("Initialized the Braisefile at {}", joined.display().bold());
    trace!("main: exiting from init");
    Ok(())
}

// Parse and load Braise file
fn load_braise_file(path: String) -> Result<Arc<BraiseFile>> {
    debug!("Found file at: {}", path);
    let value = toml::from_str::<toml::Value>(&std::fs::read_to_string(path)?)?;
    debug!("Parsed file: {:#?}", value);

    let file = Arc::new(BraiseFile::from_value(value)?);
    debug!("Parsed braisé file: {:#?}", file);
    Ok(file)
}

// Parse task input and arguments
fn parse_task_input(
    matches: &clap::ArgMatches,
    file: &BraiseFile,
) -> Result<(String, Vec<String>)> {
    if let Some((input, matches)) = matches.subcommand() {
        Ok((
            input.to_string(),
            matches
                .get_many::<OsString>("")
                .ok_or(eyre!("Couldn't parse external args"))?
                .collect::<Vec<_>>()
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect(),
        ))
    } else if let Some(ref default) = file.default {
        Ok((default.to_string(), vec![]))
    } else {
        bail!(BraiseError::NoTask);
    }
}

fn main() -> Result<()> {
    // Initialize logger
    let mut logger = build_logger();
    let matches = cli::create().get_matches();

    // Configure logging level
    let debug_level = matches.get_count("debug");
    let quiet_level = matches.get_count("quiet");
    if debug_level == 1 {
        logger.filter_level(log::LevelFilter::Debug);
    } else if debug_level > 1 {
        logger.filter_level(log::LevelFilter::Trace);
    }

    logger.try_init()?;
    trace!("main: starting");
    init_panic()?;
    debug!("Matches: {:#?}", matches);

    // Handle initialization if requested
    if let Some(path) = matches.get_one::<String>("init") {
        return handle_init(path);
    }

    // Load and parse Braise file
    let path = BraiseFile::find_path()?;
    let file = load_braise_file(path.clone())?;

    // Handle task listing if requested
    if matches.get_flag("list") {
        trace!("main: listing tasks");
        file.display(&path);
        trace!("main: exiting from list");
        return Ok(());
    }

    // Parse task input and prepare arguments
    let (input, args) = parse_task_input(&matches, &file)?;
    let args = Arc::new(args);

    // Split tasks and determine if they should run in parallel
    let inputs: Vec<_> = input.split(TASKS_SEPARATOR).map(|e| e.to_owned()).collect();
    let parallel = matches.get_flag("parallel") || file.parallel.unwrap_or(false);

    // Load environment variables
    let env_vars = Arc::new(env::load(&file)?);
    debug!("Env vars: {:#?}", env_vars);

    // Execute tasks
    let mut handles: Vec<JoinHandle<Result<()>>> = vec![];
    for task_name in inputs {
        let file = file.clone();
        let args = args.clone();
        let env_vars = env_vars.clone();

        if !parallel {
            // Wait for previous task to complete if not running in parallel
            for handle in handles.drain(..) {
                handle.join().map_err(|e| {
                    debug!("Error joining thread: {:#?}", e);
                    BraiseError::ThreadError
                })??;
            }
        }

        handles.push(spawn(move || {
            task::run(
                quiet_level,
                file.find_task(&task_name)?,
                &args,
                &file,
                &env_vars,
                &task_name,
                vec![],
            )
        }));
    }

    // Wait for all remaining tasks to complete
    for handle in handles {
        handle.join().map_err(|e| {
            debug!("Error joining thread: {:#?}", e);
            BraiseError::ThreadError
        })??;
    }

    trace!("main: exiting");
    Ok(())
}
