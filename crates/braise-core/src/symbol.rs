use crate::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub definition_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Recipe,
    Parameter,
    Variable,
    Module,
    Function,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Vec<Symbol>>,
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, symbol: Symbol) -> Result<(), String> {
        if let Some(current_scope) = self.scopes.last_mut() {
            if current_scope.contains_key(&symbol.name) {
                return Err(format!("Symbol '{}' already defined", symbol.name));
            }
            current_scope.insert(symbol.name.clone(), symbol.clone());
        }

        self.symbols
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);

        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn get_all_symbols(&self) -> &HashMap<String, Vec<Symbol>> {
        &self.symbols
    }
}
