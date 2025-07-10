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
            .map_err(|e| RuntimeError::other(e.to_string()))?;
        let status = child
            .wait()
            .map_err(|e| RuntimeError::other(e.to_string()))?;
        if !status.success() {
            return Err(RuntimeError::command_failed(
                input.to_string(),
                status.code().unwrap_or(-1),
            )
            .boxed());
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
        if self.dry_run {
            let mut output = self.output.lock().unwrap();
            output.push_str(&format!("Dry run: {input}\n"));
            return Ok(());
        }

        let mut command = resolve_shell(shell);
        let output = command
            .arg(input)
            .output()
            .map_err(|e| RuntimeError::other(e.to_string()))?;
        if !output.status.success() {
            return Err(RuntimeError::command_failed(
                input.to_string(),
                output.status.code().unwrap_or(-1),
            )
            .boxed());
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

pub struct CachingExecutor {
    inner: Box<dyn Executor>,
    cache_manager: braise_cache::CacheManager,
}

impl CachingExecutor {
    pub fn new(inner: Box<dyn Executor>, cache_manager: braise_cache::CacheManager) -> Self {
        Self {
            inner,
            cache_manager,
        }
    }
}

impl Executor for CachingExecutor {
    fn run(&self, input: &str, shell: Option<&String>) -> Result<()> {
        use braise_cache::CommandOutput;
        use std::collections::HashMap;
        use std::env;

        // Generate cache key for the command
        let working_dir = env::current_dir().unwrap_or_default();
        let env_vars: HashMap<String, String> = env::vars().collect();

        // Check cache first
        if let Ok(Some(cached_output)) =
            self.cache_manager
                .get_command_output(input, &working_dir, &env_vars)
        {
            use owo_colors::OwoColorize;
            println!(
                "    {} {} ({})",
                "→".cyan(),
                "Using cached command output".dimmed(),
                input.dimmed()
            );

            if cached_output.exit_code == 0 {
                // Print cached output for consistency
                if !cached_output.stdout.is_empty() {
                    print!("{}", cached_output.stdout);
                }
                if !cached_output.stderr.is_empty() {
                    eprint!("{}", cached_output.stderr);
                }
                return Ok(());
            } else {
                return Err(RuntimeError::command_failed(
                    input.to_string(),
                    cached_output.exit_code,
                )
                .boxed());
            }
        }

        // Execute the command and capture output if possible
        let result = self.execute_and_capture(input, shell);

        // Cache the result
        if let Ok(output) = &result {
            let command_output = CommandOutput {
                stdout: output.clone().unwrap_or_default(),
                stderr: String::new(),
                exit_code: 0,
            };

            let dependencies = vec![]; // Could add file dependencies here

            let _ = self.cache_manager.set_command_output(
                input,
                &working_dir,
                &env_vars,
                command_output,
                dependencies,
            );
        } else if let Err(err) = result.as_ref()
            && let RuntimeError::CommandFailed {
                command: _,
                exit_code,
                ..
            } = err.as_ref()
        {
            let command_output = CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: *exit_code,
            };

            let dependencies = vec![];

            let _ = self.cache_manager.set_command_output(
                input,
                &working_dir,
                &env_vars,
                command_output,
                dependencies,
            );
        }

        result.map(|_| ())
    }

    fn output(&self) -> Option<String> {
        self.inner.output()
    }
}

impl CachingExecutor {
    fn execute_and_capture(&self, input: &str, shell: Option<&String>) -> Result<Option<String>> {
        // For now, just delegate to the inner executor
        // In a more sophisticated implementation, we could capture stdout/stderr
        self.inner.run(input, shell)?;
        Ok(self.inner.output())
    }
}
