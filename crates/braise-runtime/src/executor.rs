use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;

use crate::Result;
use crate::RuntimeError;

#[derive(Debug, Clone, Default)]
pub enum ShellMode {
    /// Each command runs in a separate process (original behavior)
    Isolated,
    /// Commands run in a persistent shell session (new behavior)
    #[default]
    Persistent,
}

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub mode: ShellMode,
    pub shell: Option<String>,
    pub dry_run: bool,
    pub quiet: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellConfig {
    pub fn new() -> Self {
        Self {
            mode: ShellMode::default(),
            shell: None,
            dry_run: false,
            quiet: false,
        }
    }

    pub fn with_mode(mut self, mode: ShellMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_shell(mut self, shell: String) -> Self {
        self.shell = Some(shell);
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
}

pub trait Executor: Send + Sync {
    fn run(&self, cmd: &str, shell: Option<&String>) -> Result<()>;

    fn output(&self) -> Option<String> {
        None
    }

    fn print_output(&self, message: &str) {
        println!("{message}");
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

    fn print_output(&self, message: &str) {
        let mut output = self.output.lock().unwrap();
        output.push_str(message);
        output.push('\n');
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
    quiet: bool,
}

impl CachingExecutor {
    pub fn new(
        inner: Box<dyn Executor>,
        cache_manager: braise_cache::CacheManager,
        quiet: bool,
    ) -> Self {
        Self {
            inner,
            cache_manager,
            quiet,
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
            if !self.quiet {
                println!(
                    "    {} {} ({})",
                    "→".cyan(),
                    "Using cached command output".dimmed(),
                    input.dimmed()
                );
            }

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

pub struct PersistentShellExecutor {
    pub dry_run: bool,
    shell_process: Arc<Mutex<Option<std::process::Child>>>,
    shell_command: String,
}

impl PersistentShellExecutor {
    pub fn new(dry_run: bool, shell: Option<&String>) -> Self {
        let shell_command = if let Some(shell) = shell {
            shell.clone()
        } else if cfg!(target_os = "windows") {
            "cmd".to_string()
        } else {
            "bash".to_string()
        };

        PersistentShellExecutor {
            dry_run,
            shell_process: Arc::new(Mutex::new(None)),
            shell_command,
        }
    }

    fn ensure_shell_process(&self) -> Result<()> {
        let mut process_guard = self.shell_process.lock().unwrap();

        if process_guard.is_none() {
            let mut cmd = if cfg!(target_os = "windows") {
                Command::new("cmd")
            } else {
                Command::new(&self.shell_command)
            };

            let child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    RuntimeError::other(format!("Failed to start shell process: {e}")).boxed()
                })?;

            *process_guard = Some(child);
        }

        Ok(())
    }

    fn execute_in_shell(&self, command: &str) -> Result<()> {
        self.ensure_shell_process()?;

        let mut process_guard = self.shell_process.lock().unwrap();
        if let Some(ref mut child) = *process_guard {
            let stdin = child.stdin.as_mut().unwrap();

            // Write the command followed by a unique delimiter
            let delimiter = "__BRAISE_COMMAND_END__";
            let full_command = if cfg!(target_os = "windows") {
                format!("{command}\necho {delimiter}\n")
            } else {
                format!("{command}; echo {delimiter}\n")
            };

            stdin.write_all(full_command.as_bytes()).map_err(|e| {
                RuntimeError::other(format!("Failed to write command to shell: {e}")).boxed()
            })?;

            stdin.flush().map_err(|e| {
                RuntimeError::other(format!("Failed to flush shell stdin: {e}")).boxed()
            })?;

            // Read output until we see our delimiter
            let stdout = child.stdout.as_mut().unwrap();
            let mut reader = BufReader::new(stdout);
            let mut output = String::new();

            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.trim() == delimiter {
                            break;
                        }
                        print!("{line}");
                        output.push_str(&line);
                    }
                    Err(e) => {
                        return Err(RuntimeError::other(format!(
                            "Failed to read shell output: {e}"
                        ))
                        .boxed());
                    }
                }
            }

            Ok(())
        } else {
            Err(RuntimeError::other("Shell process not available".to_string()).boxed())
        }
    }
}

impl Executor for PersistentShellExecutor {
    fn run(&self, input: &str, _shell: Option<&String>) -> Result<()> {
        if self.dry_run {
            println!("Dry run: {input}");
            return Ok(());
        }

        self.execute_in_shell(input)
    }
}

impl Drop for PersistentShellExecutor {
    fn drop(&mut self) {
        if let Ok(mut process_guard) = self.shell_process.lock()
            && let Some(mut child) = process_guard.take()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(b"exit\n");
                let _ = stdin.flush();
            }
            let _ = child.wait();
        }
    }
}

pub struct ConfigurableShellExecutor {
    config: ShellConfig,
    persistent_executor: Option<PersistentShellExecutor>,
    default_executor: Option<DefaultExecutor>,
}

impl ConfigurableShellExecutor {
    pub fn new(config: ShellConfig) -> Self {
        let persistent_executor = match config.mode {
            ShellMode::Persistent => Some(PersistentShellExecutor::new(
                config.dry_run,
                config.shell.as_ref(),
            )),
            ShellMode::Isolated => None,
        };

        let default_executor = match config.mode {
            ShellMode::Isolated => Some(DefaultExecutor::new(config.dry_run)),
            ShellMode::Persistent => None,
        };

        Self {
            config,
            persistent_executor,
            default_executor,
        }
    }
}

impl Executor for ConfigurableShellExecutor {
    fn run(&self, cmd: &str, shell: Option<&String>) -> Result<()> {
        match &self.config.mode {
            ShellMode::Isolated => {
                if let Some(executor) = &self.default_executor {
                    // Use the shell from config if available, otherwise use the passed shell
                    let effective_shell = self.config.shell.as_ref().or(shell);
                    executor.run(cmd, effective_shell)
                } else {
                    Err(RuntimeError::other("Isolated executor not available".to_string()).boxed())
                }
            }
            ShellMode::Persistent => {
                if let Some(executor) = &self.persistent_executor {
                    // Use the shell from config if available, otherwise use the passed shell
                    let effective_shell = self.config.shell.as_ref().or(shell);
                    executor.run(cmd, effective_shell)
                } else {
                    Err(
                        RuntimeError::other("Persistent executor not available".to_string())
                            .boxed(),
                    )
                }
            }
        }
    }
}
