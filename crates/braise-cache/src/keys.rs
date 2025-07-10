use std::collections::HashMap;
use std::path::{Path, PathBuf};

use braise_types::TypedValue;
use sha2::{Digest, Sha256};

use crate::CacheLevel;

pub struct CacheKeyGenerator;

impl CacheKeyGenerator {
    pub fn generate_recipe_key(
        recipe_name: &str,
        params: &HashMap<String, TypedValue>,
        file_hashes: &HashMap<PathBuf, String>,
    ) -> String {
        let mut hasher = Sha256::new();

        // Add recipe name
        hasher.update(recipe_name.as_bytes());

        // Add sorted parameters
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by_key(|(k, _)| *k);

        for (key, value) in sorted_params {
            hasher.update(key.as_bytes());
            hasher.update(Self::hash_typed_value(value).as_bytes());
        }

        // Add file hashes
        let mut sorted_files: Vec<_> = file_hashes.iter().collect();
        sorted_files.sort_by_key(|(path, _)| *path);

        for (path, hash) in sorted_files {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(hash.as_bytes());
        }

        let hash = hex::encode(hasher.finalize());
        format!("recipe:{}:{}", recipe_name, &hash[..16])
    }

    pub fn generate_command_key(
        command: &str,
        working_dir: &Path,
        env_vars: &HashMap<String, String>,
    ) -> String {
        let mut hasher = Sha256::new();

        // Add command
        hasher.update(command.as_bytes());

        // Add working directory
        hasher.update(working_dir.to_string_lossy().as_bytes());

        // Add relevant environment variables (sorted)
        let mut sorted_env: Vec<_> = env_vars.iter().collect();
        sorted_env.sort_by_key(|(k, _)| *k);

        for (key, value) in sorted_env {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }

        let hash = hex::encode(hasher.finalize());
        format!("cmd:{}", &hash[..16])
    }

    pub fn generate_builtin_key(module: &str, function: &str, args: &[TypedValue]) -> String {
        let mut hasher = Sha256::new();

        // Add module and function
        hasher.update(module.as_bytes());
        hasher.update(function.as_bytes());

        // Add arguments
        for arg in args {
            hasher.update(Self::hash_typed_value(arg).as_bytes());
        }

        let hash = hex::encode(hasher.finalize());
        format!("builtin:{}:{}:{}", module, function, &hash[..16])
    }

    pub fn generate_expression_key(
        expression: &str,
        variables: &HashMap<String, TypedValue>,
    ) -> String {
        let mut hasher = Sha256::new();

        // Add expression
        hasher.update(expression.as_bytes());

        // Add variables (sorted)
        let mut sorted_vars: Vec<_> = variables.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| *k);

        for (key, value) in sorted_vars {
            hasher.update(key.as_bytes());
            hasher.update(Self::hash_typed_value(value).as_bytes());
        }

        let hash = hex::encode(hasher.finalize());
        format!("expr:{}", &hash[..16])
    }

    pub fn extract_level_from_key(key: &str) -> Option<CacheLevel> {
        match key.split(':').next() {
            Some("recipe") => Some(CacheLevel::Recipe),
            Some("cmd") => Some(CacheLevel::Command),
            Some("builtin") => Some(CacheLevel::Builtin),
            Some("expr") => Some(CacheLevel::Expression),
            _ => None,
        }
    }

    pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn hash_file_metadata(path: &Path) -> Result<String, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        let mut hasher = Sha256::new();

        // Hash file size and modification time
        hasher.update(metadata.len().to_le_bytes());

        if let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hasher.update(duration.as_secs().to_le_bytes());
        }

        Ok(hex::encode(hasher.finalize()))
    }

    pub fn hash_directory(path: &Path) -> Result<String, std::io::Error> {
        let mut hasher = Sha256::new();

        // Walk directory and hash all files
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .sort_by_file_name()
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                hasher.update(path.to_string_lossy().as_bytes());
                hasher.update(&Self::hash_file_metadata(path)?);
            }
        }

        Ok(hex::encode(hasher.finalize()))
    }

    fn hash_typed_value(value: &TypedValue) -> String {
        let mut hasher = Sha256::new();

        match &value.value {
            braise_types::ValueData::String(s) => {
                hasher.update(b"string");
                hasher.update(s.as_bytes());
            }
            braise_types::ValueData::Number(n) => {
                hasher.update(b"number");
                hasher.update(n.to_le_bytes());
            }
            braise_types::ValueData::Bool(b) => {
                hasher.update(b"bool");
                hasher.update([if *b { 1 } else { 0 }]);
            }
            braise_types::ValueData::Array(arr) => {
                hasher.update(b"array");
                for item in arr {
                    hasher.update(Self::hash_typed_value(item).as_bytes());
                }
            }
            braise_types::ValueData::Recipe(name, args) => {
                hasher.update(b"recipe");
                hasher.update(name.as_bytes());
                let mut sorted_args: Vec<_> = args.iter().collect();
                sorted_args.sort_by_key(|(k, _)| *k);
                for (key, value) in sorted_args {
                    hasher.update(key.as_bytes());
                    hasher.update(Self::hash_typed_value(value).as_bytes());
                }
            }
            braise_types::ValueData::None => {
                hasher.update(b"none");
            }
            braise_types::ValueData::Error { message, code } => {
                hasher.update(b"error");
                hasher.update(message.as_bytes());
                if let Some(code) = code {
                    hasher.update(code.to_le_bytes());
                }
            }
        }

        hex::encode(hasher.finalize())
    }
}
