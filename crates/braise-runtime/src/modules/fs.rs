use crate::{BraiseType, Result, TypedValue};

pub struct FsModule;

impl FsModule {
    pub fn new() -> Self {
        Self
    }

    fn exists(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        Ok(TypedValue::new(path.exists(), BraiseType::Bool))
    }

    fn is_file(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        Ok(TypedValue::new(path.is_file(), BraiseType::Bool))
    }

    fn is_dir(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        Ok(TypedValue::new(path.is_dir(), BraiseType::Bool))
    }

    fn read(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to read file: {e}"), "fs")
        })?;
        Ok(TypedValue::new(content, BraiseType::String))
    }

    fn write(&self, path: TypedValue, content: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        std::fs::write(path, content.to_string()).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to write file: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn rm(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        std::fs::remove_file(path).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to remove file: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn mkdir(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        std::fs::create_dir(path).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to create directory: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn rmdir(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        std::fs::remove_dir(path).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to remove directory: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn ls(&self, path: TypedValue) -> Result<TypedValue> {
        let binding = path.to_string();
        let path = std::path::Path::new(&binding);
        let entries = std::fs::read_dir(path).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to read directory: {e}"), "fs")
        })?;
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                crate::RuntimeError::builtin_error(
                    format!("Failed to read directory entry: {e}"),
                    "fs",
                )
            })?;
            result.push(TypedValue::new(
                entry.file_name().to_string_lossy().to_string(),
                BraiseType::String,
            ));
        }
        Ok(TypedValue::new(
            result,
            BraiseType::Array(Box::new(BraiseType::String)),
        ))
    }

    fn cp(&self, src: TypedValue, dest: TypedValue) -> Result<TypedValue> {
        let src_binding = src.to_string();
        let dest_binding = dest.to_string();
        std::fs::copy(src_binding, dest_binding).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to copy file: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn mv(&self, src: TypedValue, dest: TypedValue) -> Result<TypedValue> {
        let src_binding = src.to_string();
        let dest_binding = dest.to_string();
        std::fs::rename(src_binding, dest_binding).map_err(|e| {
            crate::RuntimeError::builtin_error(format!("Failed to move file: {e}"), "fs")
        })?;
        Ok(TypedValue::new(true, BraiseType::Bool))
    }

    fn glob(&self, pattern: TypedValue) -> Result<TypedValue> {
        let binding = pattern.to_string();
        let result: Vec<_> = glob::glob(&binding)
            .map_err(|e| crate::RuntimeError::builtin_error(format!("Glob error: {e}"), "fs"))?
            .filter_map(Result::ok)
            .map(|path| TypedValue::new(path.to_string_lossy().to_string(), BraiseType::String))
            .collect();
        Ok(TypedValue::new(
            result,
            BraiseType::Array(Box::new(BraiseType::String)),
        ))
    }
}

builtin_module! {
    FsModule {
        functions: {
            "exists" => exists(BraiseType::String),
            "is_file" => is_file(BraiseType::String),
            "is_dir" => is_dir(BraiseType::String),
            "read" => read(BraiseType::String),
            "write" => write(BraiseType::String, BraiseType::String),
            "rm" => rm(BraiseType::String),
            "mkdir" => mkdir(BraiseType::String),
            "rmdir" => rmdir(BraiseType::String),
            "ls" => ls(BraiseType::String),
            "cp" => cp(BraiseType::String, BraiseType::String),
            "mv" => mv(BraiseType::String, BraiseType::String),
            "glob" => glob(BraiseType::String),
        }
    }
}
