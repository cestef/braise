use crate::BraiseType;
use std::collections::HashMap;

/// Type checker for expressions and statements with scope management
#[derive(Debug, Clone)]
pub struct TypeChecker {
    /// Current type context (variable name -> type)
    context: HashMap<String, BraiseType>,
    /// Parent scope for nested contexts
    parent: Option<Box<TypeChecker>>,
}

impl TypeChecker {
    /// Create a new empty type checker
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
            parent: None,
        }
    }

    /// Create a type checker with an initial context
    pub fn with_context(context: HashMap<String, BraiseType>) -> Self {
        Self {
            context,
            parent: None,
        }
    }

    /// Add a variable to the type context
    pub fn define_variable(&mut self, name: String, var_type: BraiseType) {
        self.context.insert(name, var_type);
    }

    /// Get the type of a variable, checking parent scopes if needed
    pub fn get_variable_type(&self, name: &str) -> Option<&BraiseType> {
        self.context.get(name).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_variable_type(name))
        })
    }

    /// Check if a variable is defined in current or parent scopes
    pub fn is_variable_defined(&self, name: &str) -> bool {
        self.context.contains_key(name)
            || self
                .parent
                .as_ref()
                .map_or(false, |parent| parent.is_variable_defined(name))
    }

    /// Create a new scope that inherits from this one
    pub fn enter_scope(&self) -> TypeChecker {
        TypeChecker {
            context: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Get all variables in current scope (not including parent scopes)
    pub fn get_current_scope_variables(&self) -> &HashMap<String, BraiseType> {
        &self.context
    }

    /// Get all variables including parent scopes
    pub fn get_all_variables(&self) -> HashMap<String, BraiseType> {
        let mut all_vars = HashMap::new();

        if let Some(parent) = &self.parent {
            all_vars.extend(parent.get_all_variables());
        }

        // Override with current scope variables
        all_vars.extend(self.context.clone());

        all_vars
    }

    /// Remove a variable from the current scope
    pub fn remove_variable(&mut self, name: &str) -> Option<BraiseType> {
        self.context.remove(name)
    }

    /// Clear all variables in the current scope
    pub fn clear_scope(&mut self) {
        self.context.clear();
    }

    /// Check if this is the root scope (no parent)
    pub fn is_root_scope(&self) -> bool {
        self.parent.is_none()
    }

    /// Get the depth of this scope (root = 0, child = 1, etc.)
    pub fn scope_depth(&self) -> usize {
        match &self.parent {
            Some(parent) => 1 + parent.scope_depth(),
            None => 0,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution context for runtime type tracking
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Variable storage with type information
    variables: HashMap<String, crate::TypedValue>,
    /// Optional shell override
    pub shell: Option<String>,
}

impl ExecutionContext {
    /// Create a new empty execution context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            shell: None,
        }
    }

    /// Set a variable with its typed value
    pub fn set(&mut self, name: String, value: crate::TypedValue) {
        self.variables.insert(name, value);
    }

    /// Get a variable's value
    pub fn get(&self, name: &str) -> Option<&crate::TypedValue> {
        self.variables.get(name)
    }

    /// Check if a variable exists
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Remove a variable
    pub fn remove(&mut self, name: &str) -> Option<crate::TypedValue> {
        self.variables.remove(name)
    }

    /// Get all variable names
    pub fn variable_names(&self) -> Vec<&String> {
        self.variables.keys().collect()
    }

    /// Get all variables as a reference
    pub fn variables(&self) -> &HashMap<String, crate::TypedValue> {
        &self.variables
    }

    /// Set the shell override
    pub fn set_shell(&mut self, shell: String) {
        self.shell = Some(shell);
    }

    /// Clear the shell override
    pub fn clear_shell(&mut self) {
        self.shell = None;
    }

    /// Merge another context into this one
    pub fn merge(&mut self, other: ExecutionContext) {
        self.variables.extend(other.variables);
        if other.shell.is_some() {
            self.shell = other.shell;
        }
    }

    /// Create a type checker from current execution context
    pub fn to_type_checker(&self) -> TypeChecker {
        let context = self
            .variables
            .iter()
            .map(|(name, value)| (name.clone(), value.value_type.clone()))
            .collect();
        TypeChecker::with_context(context)
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BraiseType, TypedValue};

    #[test]
    fn test_type_checker_scopes() {
        let mut root = TypeChecker::new();
        root.define_variable("x".to_string(), BraiseType::String);

        let mut child = root.enter_scope();
        child.define_variable("y".to_string(), BraiseType::Number);

        // Child can see both variables
        assert!(child.is_variable_defined("x"));
        assert!(child.is_variable_defined("y"));

        // Root can only see its own variable
        assert!(root.is_variable_defined("x"));
        assert!(!root.is_variable_defined("y"));
    }

    #[test]
    fn test_scope_depth() {
        let root = TypeChecker::new();
        assert_eq!(root.scope_depth(), 0);

        let child = root.enter_scope();
        assert_eq!(child.scope_depth(), 1);

        let grandchild = child.enter_scope();
        assert_eq!(grandchild.scope_depth(), 2);
    }

    #[test]
    fn test_execution_context() {
        let mut ctx = ExecutionContext::new();

        let value = TypedValue::new("hello", BraiseType::String);
        ctx.set("test".to_string(), value.clone());

        assert!(ctx.contains("test"));
        assert_eq!(ctx.get("test"), Some(&value));

        let type_checker = ctx.to_type_checker();
        assert_eq!(
            type_checker.get_variable_type("test"),
            Some(&BraiseType::String)
        );
    }

    #[test]
    fn test_context_merge() {
        let mut ctx1 = ExecutionContext::new();
        ctx1.set(
            "x".to_string(),
            TypedValue::new("hello", BraiseType::String),
        );

        let mut ctx2 = ExecutionContext::new();
        ctx2.set("y".to_string(), TypedValue::new(42.0, BraiseType::Number));
        ctx2.set_shell("bash".to_string());

        ctx1.merge(ctx2);

        assert!(ctx1.contains("x"));
        assert!(ctx1.contains("y"));
        assert_eq!(ctx1.shell, Some("bash".to_string()));
    }
}
