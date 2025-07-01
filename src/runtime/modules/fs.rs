use crate::runtime::{Value, error::*};

pub struct FsModule;

impl FsModule {
    pub fn new() -> Self {
        Self
    }

    fn exists(&self, path: String) -> Result<Value> {
        let path = std::path::Path::new(&path);
        Ok(Value::Bool(path.exists()))
    }

    fn is_file(&self, path: String) -> Result<Value> {
        let path = std::path::Path::new(&path);
        Ok(Value::Bool(path.is_file()))
    }

    fn is_dir(&self, path: String) -> Result<Value> {
        let path = std::path::Path::new(&path);
        Ok(Value::Bool(path.is_dir()))
    }
}

builtin_module! {
    FsModule {
        functions: {
            "exists" => exists(String),
            "is_file" => is_file(String),
            "is_dir" => is_dir(String),
        }
    }
}
