use crate::{Result, RuntimeError, Value};
use std::process::Command;

pub struct GitModule;

impl GitModule {
    pub fn new() -> Self {
        Self
    }

    fn is_dirty(&self) -> Result<Value> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        Ok(Value::Bool(!output.stdout.is_empty()))
    }

    fn is_clean(&self) -> Result<Value> {
        self.is_dirty().map(|v| Value::Bool(!v.to_bool()))
    }

    fn branch(&self) -> Result<Value> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Value::String(branch))
    }

    fn commit_hash(&self) -> Result<Value> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Value::String(hash))
    }

    fn commit_hash_short(&self) -> Result<Value> {
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Value::String(hash))
    }

    fn tag(&self) -> Result<Value> {
        let output = Command::new("git")
            .args(["describe", "--tags", "--exact-match"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        if output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(Value::String(tag))
        } else {
            Ok(Value::String(String::new())) // No tag on current commit
        }
    }
}

builtin_module! {
    GitModule {
        functions: {
            "is_dirty" => is_dirty,
            "is_clean" => is_clean,
            "branch" => branch,
            "commit_hash" => commit_hash,
            "commit_hash_short" => commit_hash_short,
            "tag" => tag,
        }
    }
}
