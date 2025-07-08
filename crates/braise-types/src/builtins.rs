use crate::BraiseType;
use std::collections::HashMap;

/// Registry of builtin module types for the Braise language
#[derive(Debug, Clone)]
pub struct BuiltinTypeRegistry {
    modules: HashMap<String, ModuleTypes>,
}

#[derive(Debug, Clone)]
pub struct ModuleTypes {
    /// Function name -> return type
    pub functions: HashMap<String, BraiseType>,
    /// Field name -> field type  
    pub fields: HashMap<String, BraiseType>,
}

impl BuiltinTypeRegistry {
    /// Create a new registry with all builtin types
    pub fn new() -> Self {
        let mut modules = HashMap::new();

        // Environment module
        let env_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("get".to_string(), BraiseType::String);
                funcs.insert("has".to_string(), BraiseType::Bool);
                funcs
            },
            fields: {
                let mut fields = HashMap::new();
                fields.insert("HOME".to_string(), BraiseType::String);
                fields.insert("PWD".to_string(), BraiseType::String);
                fields.insert("CI".to_string(), BraiseType::Bool);
                fields
            },
        };
        modules.insert("env".to_string(), env_module);

        // CPU module
        let cpu_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("count".to_string(), BraiseType::Number);
                funcs.insert("physical_count".to_string(), BraiseType::Number);
                funcs
            },
            fields: {
                let mut fields = HashMap::new();
                fields.insert("arch".to_string(), BraiseType::String);
                fields
            },
        };
        modules.insert("cpu".to_string(), cpu_module);

        // Git module
        let git_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("branch".to_string(), BraiseType::String);
                funcs.insert("commit_hash".to_string(), BraiseType::String);
                funcs.insert("commit_hash_short".to_string(), BraiseType::String);
                funcs.insert("is_clean".to_string(), BraiseType::Bool);
                funcs.insert("is_dirty".to_string(), BraiseType::Bool);
                funcs.insert("tag".to_string(), BraiseType::String);
                funcs
            },
            fields: HashMap::new(),
        };
        modules.insert("git".to_string(), git_module);

        // Filesystem module
        let fs_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("exists".to_string(), BraiseType::Bool);
                funcs.insert("is_file".to_string(), BraiseType::Bool);
                funcs.insert("is_dir".to_string(), BraiseType::Bool);
                funcs
            },
            fields: HashMap::new(),
        };
        modules.insert("fs".to_string(), fs_module);

        // OS module
        let os_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("name".to_string(), BraiseType::String);
                funcs.insert("version".to_string(), BraiseType::String);
                funcs
            },
            fields: HashMap::new(),
        };
        modules.insert("os".to_string(), os_module);

        // Input module for user interaction
        let input_module = ModuleTypes {
            functions: {
                let mut funcs = HashMap::new();
                funcs.insert("text".to_string(), BraiseType::String);
                funcs.insert("num".to_string(), BraiseType::Number);
                funcs.insert("confirm".to_string(), BraiseType::Bool);
                funcs.insert("select".to_string(), BraiseType::String);
                funcs.insert("multiselect".to_string(), BraiseType::Array(Box::new(BraiseType::String)));
                funcs.insert("password".to_string(), BraiseType::String);
                funcs
            },
            fields: HashMap::new(),
        };
        modules.insert("input".to_string(), input_module);

        Self { modules }
    }

    /// Get the return type of a builtin function
    pub fn get_function_type(&self, module: &str, function: &str) -> Option<&BraiseType> {
        self.modules
            .get(module)
            .and_then(|m| m.functions.get(function))
    }

    /// Get the type of a builtin field
    pub fn get_field_type(&self, module: &str, field: &str) -> Option<&BraiseType> {
        self.modules.get(module).and_then(|m| m.fields.get(field))
    }

    /// Check if a module exists
    pub fn has_module(&self, module: &str) -> bool {
        self.modules.contains_key(module)
    }

    /// Check if a function exists in a module
    pub fn has_function(&self, module: &str, function: &str) -> bool {
        self.modules
            .get(module)
            .map_or(false, |m| m.functions.contains_key(function))
    }

    /// Check if a field exists in a module
    pub fn has_field(&self, module: &str, field: &str) -> bool {
        self.modules
            .get(module)
            .map_or(false, |m| m.fields.contains_key(field))
    }

    /// Get all modules
    pub fn modules(&self) -> &HashMap<String, ModuleTypes> {
        &self.modules
    }

    /// Get all functions for a module
    pub fn get_module_functions(&self, module: &str) -> Option<&HashMap<String, BraiseType>> {
        self.modules.get(module).map(|m| &m.functions)
    }

    /// Get all fields for a module
    pub fn get_module_fields(&self, module: &str) -> Option<&HashMap<String, BraiseType>> {
        self.modules.get(module).map(|m| &m.fields)
    }

    /// Add a custom module (for extensibility)
    pub fn add_module(&mut self, name: String, module: ModuleTypes) {
        self.modules.insert(name, module);
    }

    /// Add a function to an existing module
    pub fn add_function(
        &mut self,
        module: &str,
        function: String,
        return_type: BraiseType,
    ) -> Result<(), String> {
        let module_types = self
            .modules
            .get_mut(module)
            .ok_or_else(|| format!("Module '{}' not found", module))?;

        module_types.functions.insert(function, return_type);
        Ok(())
    }

    /// Add a field to an existing module
    pub fn add_field(
        &mut self,
        module: &str,
        field: String,
        field_type: BraiseType,
    ) -> Result<(), String> {
        let module_types = self
            .modules
            .get_mut(module)
            .ok_or_else(|| format!("Module '{}' not found", module))?;

        module_types.fields.insert(field, field_type);
        Ok(())
    }
}

impl Default for BuiltinTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleTypes {
    /// Create a new empty module
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    /// Create a module with functions only
    pub fn with_functions(functions: HashMap<String, BraiseType>) -> Self {
        Self {
            functions,
            fields: HashMap::new(),
        }
    }

    /// Create a module with fields only
    pub fn with_fields(fields: HashMap<String, BraiseType>) -> Self {
        Self {
            functions: HashMap::new(),
            fields,
        }
    }

    /// Create a module with both functions and fields
    pub fn with_both(
        functions: HashMap<String, BraiseType>,
        fields: HashMap<String, BraiseType>,
    ) -> Self {
        Self { functions, fields }
    }

    /// Add a function to this module
    pub fn add_function(&mut self, name: String, return_type: BraiseType) {
        self.functions.insert(name, return_type);
    }

    /// Add a field to this module
    pub fn add_field(&mut self, name: String, field_type: BraiseType) {
        self.fields.insert(name, field_type);
    }
}

impl Default for ModuleTypes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_registry_creation() {
        let registry = BuiltinTypeRegistry::new();

        // Test env module
        assert!(registry.has_module("env"));
        assert!(registry.has_function("env", "get"));
        assert!(registry.has_field("env", "HOME"));
        assert_eq!(
            registry.get_function_type("env", "get"),
            Some(&BraiseType::String)
        );
        assert_eq!(
            registry.get_field_type("env", "HOME"),
            Some(&BraiseType::String)
        );

        // Test cpu module
        assert!(registry.has_module("cpu"));
        assert!(registry.has_function("cpu", "count"));
        assert_eq!(
            registry.get_function_type("cpu", "count"),
            Some(&BraiseType::Number)
        );

        // Test git module
        assert!(registry.has_module("git"));
        assert!(registry.has_function("git", "branch"));
        assert_eq!(
            registry.get_function_type("git", "is_clean"),
            Some(&BraiseType::Bool)
        );

        // Test fs module
        assert!(registry.has_module("fs"));
        assert!(registry.has_function("fs", "exists"));
        assert_eq!(
            registry.get_function_type("fs", "exists"),
            Some(&BraiseType::Bool)
        );
    }

    #[test]
    fn test_custom_module_addition() {
        let mut registry = BuiltinTypeRegistry::new();

        let mut custom_module = ModuleTypes::new();
        custom_module.add_function("test_func".to_string(), BraiseType::String);
        custom_module.add_field("test_field".to_string(), BraiseType::Number);

        registry.add_module("custom".to_string(), custom_module);

        assert!(registry.has_module("custom"));
        assert!(registry.has_function("custom", "test_func"));
        assert!(registry.has_field("custom", "test_field"));
    }

    #[test]
    fn test_module_extension() {
        let mut registry = BuiltinTypeRegistry::new();

        registry
            .add_function("env", "new_func".to_string(), BraiseType::Bool)
            .unwrap();
        assert!(registry.has_function("env", "new_func"));
        assert_eq!(
            registry.get_function_type("env", "new_func"),
            Some(&BraiseType::Bool)
        );

        // Try to add to non-existent module
        assert!(
            registry
                .add_function("nonexistent", "func".to_string(), BraiseType::String)
                .is_err()
        );
    }
}
