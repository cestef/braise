use std::collections::HashMap;

use super::value::Value;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub variables: HashMap<String, Value>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}
