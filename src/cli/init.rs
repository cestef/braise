use std::path::Path;

use paris_log::{debug, success, trace, warn};

use crate::{DEFAULT_CONFIG, Result, file::BraiseFile, utils::confirm_action};

// Handle initialization of a new Braise file

pub fn handle(path: &str) -> Result<()> {
    trace!("main: initializing");
    let mut name = "braise.toml".to_string();

    if let Ok(path) = BraiseFile::find_path() {
        warn!("The Braisefile already exists at <b>{path}</>");
        if !confirm_action("Do you want to overwrite it? [y/N]")? {
            debug!("Exiting...");
            return Ok(());
        }
        name = path;
    }

    let file_path = Path::new(path);
    let joined = file_path.join(name);
    std::fs::write(&joined, DEFAULT_CONFIG)?;
    success!("Initialized the Braisefile at <b>{joined:?}</>");
    trace!("main: exiting from init");
    Ok(())
}
