use std::io::BufRead;
use std::io::BufReader;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Result;
use crate::RuntimeError;

pub trait Executor: Send + Sync {
    fn run(&self, cmd: &str) -> Result<()>;

    fn output(&self) -> Option<String> {
        None
    }
}

pub struct DefaultExecutor {
    pub dry_run: bool,
}

impl DefaultExecutor {
    pub fn new(dry_run: bool) -> Self {
        DefaultExecutor { dry_run }
    }
}

impl Executor for DefaultExecutor {
    fn run(&self, input: &str) -> Result<()> {
        use std::process::Command;

        if self.dry_run {
            println!("Dry run: {}", input);
            return Ok(());
        }
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", input]);
            cmd
        } else {
            let mut cmd = Command::new("zsh");
            cmd.args(["-c", input]);
            cmd
        };
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| RuntimeError::Other(e.to_string()))?;

        if let Some(stdout) = child.stdout.take() {
            let stdout_reader = BufReader::new(stdout);
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| RuntimeError::Other(e.to_string()))?;

        if !output.status.success() {
            return Err(RuntimeError::CommandFailed {
                command: input.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            });
        }

        // Print any remaining stderr
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}

pub struct StringExecutor {
    pub dry_run: bool,
    pub output: Arc<Mutex<String>>,
}

impl Executor for StringExecutor {
    fn run(&self, input: &str) -> Result<()> {
        // pipe cmd outptu to a string
        if self.dry_run {
            let mut output = self.output.lock().unwrap();
            output.push_str(&format!("Dry run: {}\n", input));
            return Ok(());
        }
        use std::process::Command;

        let mut command = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", input]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", input]);
            cmd
        };
        let output = command
            .output()
            .map_err(|e| RuntimeError::Other(e.to_string()))?;
        if !output.status.success() {
            return Err(RuntimeError::CommandFailed {
                command: input.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            });
        }

        let mut out = self.output.lock().unwrap();
        out.push_str(&String::from_utf8_lossy(&output.stdout));
        out.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(())
    }

    fn output(&self) -> Option<String> {
        let output = self.output.lock().unwrap();
        if output.is_empty() {
            None
        } else {
            Some(output.clone())
        }
    }
}

impl StringExecutor {
    pub fn new(dry_run: bool) -> Self {
        StringExecutor {
            dry_run,
            output: Arc::new(Mutex::new(String::new())),
        }
    }
}
