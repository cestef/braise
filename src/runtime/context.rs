use std::collections::HashMap;

use super::value::Value;

pub struct ExecutionContext {
    pub variables: HashMap<String, Value>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    // pub fn with_parameters(params: HashMap<String, Value>) -> Self {
    //     Self { variables: params }
    // }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }
}
