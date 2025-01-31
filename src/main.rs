use std::sync::Arc;

use braise::{
    cli, env, file::BraiseFile, panic, utils::build_logger, BraiseError, Result, TASKS_SEPARATOR,
};
use paris_log::{debug, trace};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    panic::init()?;

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

    logger.try_init().map_err(BraiseError::from)?;

    trace!("main: starting");

    // Handle initialization if requested
    if let Some(path) = matches.get_one::<String>("init") {
        return Ok(cli::init::handle(path)?);
    }

    // Load and parse Braise file
    let file = BraiseFile::new()?;

    // Handle task listing if requested
    if matches.get_flag("list") {
        trace!("main: listing tasks");
        file.display();
        trace!("main: exiting from list");
        return Ok(());
    }

    let (input, args) = file.parse_matches(&matches)?;

    let inputs: Vec<_> = input.split(TASKS_SEPARATOR).map(|e| e.to_owned()).collect();
    // Resolve glob patterns in inputs
    let inputs = file.resolve_globs(inputs)?;

    debug!("main: inputs: {:?}", inputs);

    let parallel = matches.get_flag("parallel") || file.parallel.unwrap_or(false);

    let env_vars = Arc::new(env::load(&file)?);
    let args = Arc::new(args);

    // Build the dependency graph
    let graph = file.build_graph(&inputs)?;

    // Handle task execution
    let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

    let task_map = Arc::new(file.tasks.clone());

    for node in &graph {
        let env_vars = env_vars.clone();
        let args = args.clone();
        let node = node.clone();
        let quiet = file.quiet_settings(quiet_level, node.task.quiet.clone());
        let task_map = task_map.clone();

        if !parallel {
            debug!("main: running task sequentially");
            for handle in handles.drain(..) {
                println!("Waiting for task to finish");
                handle.await.map_err(BraiseError::from)??;
            }
        }

        let handle = tokio::spawn(async move { node.run(env_vars, args, quiet, task_map).await });
        handles.push(handle);
    }

    // Wait for all tasks to finish
    for handle in handles {
        handle.await.map_err(BraiseError::from)??;
    }

    Ok(())
}
