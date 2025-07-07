use crate::{BraiseType, Result, RuntimeError, TypedValue};
use std::process::Command;

pub struct GitModule;

impl GitModule {
    pub fn new() -> Self {
        Self
    }

    fn is_dirty(&self) -> Result<TypedValue> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        Ok(TypedValue::new(!output.stdout.is_empty(), BraiseType::Bool))
    }

    fn is_clean(&self) -> Result<TypedValue> {
        self.is_dirty()
            .map(|v| TypedValue::new(!v.to_bool(), BraiseType::Bool))
    }

    fn branch(&self) -> Result<TypedValue> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(TypedValue::new(branch, BraiseType::String))
    }

    fn commit_hash(&self) -> Result<TypedValue> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(TypedValue::new(hash, BraiseType::String))
    }

    fn commit_hash_short(&self) -> Result<TypedValue> {
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(TypedValue::new(hash, BraiseType::String))
    }

    fn tag(&self) -> Result<TypedValue> {
        let output = Command::new("git")
            .args(["describe", "--tags", "--exact-match"])
            .output()
            .map_err(|e| RuntimeError::BuiltinError(format!("Git command failed: {e}")))?;

        if output.status.success() {
            let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(TypedValue::new(tag, BraiseType::String))
        } else {
            Ok(TypedValue::new(String::new(), BraiseType::String))
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
