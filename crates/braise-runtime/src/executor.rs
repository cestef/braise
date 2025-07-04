use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Result;
use crate::RuntimeError;

pub trait Executor: Send + Sync {
    fn run(&self, cmd: &str, shell: Option<&String>) -> Result<()>;

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
    fn run(&self, input: &str, shell: Option<&String>) -> Result<()> {
        if self.dry_run {
            println!("Dry run: {input}");
            return Ok(());
        }
        let mut cmd = resolve_shell(shell);
        let mut child = cmd
            .arg(input)
            .spawn()
            .map_err(|e| RuntimeError::Other(e.to_string()))?;
        let status = child
            .wait()
            .map_err(|e| RuntimeError::Other(e.to_string()))?;
        if !status.success() {
            return Err(RuntimeError::CommandFailed {
                command: input.to_string(),
                exit_code: status.code().unwrap_or(-1),
            });
        }

        Ok(())
    }
}

fn resolve_shell(shell: Option<&String>) -> Command {
    if let Some(shell) = shell {
        let (shell, args) = shell.split_once(' ').unwrap_or((shell, ""));
        let mut cmd = Command::new(shell);
        let args = args.split_whitespace();
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    } else if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        cmd
    }
}

pub struct StringExecutor {
    pub dry_run: bool,
    pub output: Arc<Mutex<String>>,
}

impl Executor for StringExecutor {
    fn run(&self, input: &str, shell: Option<&String>) -> Result<()> {
        // pipe cmd outptu to a string
        if self.dry_run {
            let mut output = self.output.lock().unwrap();
            output.push_str(&format!("Dry run: {input}\n"));
            return Ok(());
        }

        let mut command = resolve_shell(shell);
        let output = command
            .arg(input)
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
