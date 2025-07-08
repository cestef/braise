use core::TypedValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub variables: HashMap<String, TypedValue>,
    pub shell: Option<String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            shell: None,
        }
    }

    pub fn set(&mut self, name: String, value: TypedValue) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&TypedValue> {
        self.variables.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.variables.len()
    }

    pub fn set_shell(&mut self, shell: String) {
        self.shell = Some(shell);
    }
}
