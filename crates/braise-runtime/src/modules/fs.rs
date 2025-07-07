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
}

builtin_module! {
    FsModule {
        functions: {
            "exists" => exists(BraiseType::String),
            "is_file" => is_file(BraiseType::String),
            "is_dir" => is_dir(BraiseType::String),
        }
    }
}
