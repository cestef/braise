use std::collections::HashMap;

use crate::{Span, Spanned};

pub type SpannedNode<T> = Spanned<T>;

#[derive(Debug, Clone)]
pub struct Config {
    pub recipes: Vec<SpannedNode<Recipe>>,
    pub shell: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub dependencies: Vec<String>,
    pub parameters: Vec<SpannedNode<Parameter>>,
    pub body: Vec<SpannedNode<Statement>>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: ParamType,
    pub default: Option<SpannedNode<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    String,
    Number,
    Bool,
    Array(Box<ParamType>),
    Enum(Vec<String>),
    Recipe,
    // TODO: more types ?
}

impl ParamType {
    /// Check if a type is compatible with another (for validation)
    pub fn is_compatible_with(&self, other: &ParamType) -> bool {
        match (self, other) {
            (ParamType::String, ParamType::String) => true,
            (ParamType::Number, ParamType::Number) => true,
            (ParamType::Bool, ParamType::Bool) => true,
            (ParamType::Array(a), ParamType::Array(b)) => a.is_compatible_with(b),
            (ParamType::Enum(a), ParamType::Enum(b)) => a == b,
            (ParamType::Number, ParamType::String) => true,
            (ParamType::Bool, ParamType::String) => true,
            _ => false,
        }
    }

    /// Get the default value for this type
    pub fn default_value(&self) -> Expression {
        match self {
            ParamType::String => Expression::String(String::new()),
            ParamType::Number => Expression::Number(0.0),
            ParamType::Bool => Expression::Bool(false),
            ParamType::Array(_) => Expression::Array(Vec::new()),
            ParamType::Enum(variants) => {
                if let Some(first) = variants.first() {
                    Expression::String(first.clone())
                } else {
                    Expression::String(String::new())
                }
            }
            ParamType::Recipe => Expression::RecipeRef {
                recipe: String::new(),
                args: HashMap::new(),
            },
        }
    }
}

impl std::fmt::Display for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamType::String => write!(f, "string"),
            ParamType::Number => write!(f, "number"),
            ParamType::Bool => write!(f, "bool"),
            ParamType::Array(element_type) => write!(f, "array[{element_type}]"),
            ParamType::Enum(variants) => write!(f, "enum[{}]", variants.join(", ")),
            ParamType::Recipe => write!(f, "recipe"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Run(SpannedNode<Expression>),
    Print(SpannedNode<Expression>),
    If {
        condition: SpannedNode<Expression>,
        then_block: Vec<SpannedNode<Statement>>,
        else_block: Option<Vec<SpannedNode<Statement>>>,
    },
    Match {
        expr: SpannedNode<Expression>,
        arms: Vec<SpannedNode<MatchArm>>,
    },
    For {
        var: String,
        iterable: SpannedNode<Expression>,
        body: Vec<SpannedNode<Statement>>,
        is_async: bool,
    },
    Exit(SpannedNode<Expression>),
    Let {
        name: String,
        value: Option<SpannedNode<Expression>>,
        param_type: ParamType,
    },
    Assign {
        name: String,
        value: SpannedNode<Expression>,
    },
    Call {
        recipe: SpannedNode<Expression>,
        args: HashMap<String, SpannedNode<Expression>>,
    },
    Shell {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    // Literal patterns
    String(String),
    Number(f64),
    Bool(bool),

    // Wildcard pattern
    Wildcard,

    // Variable binding pattern (captures the value)
    Variable(String),

    // Or pattern (multiple alternatives)
    Or(Vec<SpannedNode<MatchPattern>>),

    // Range patterns
    Range {
        start: Option<f64>, // None means unbounded
        end: Option<f64>,   // None means unbounded
        inclusive: bool,    // true for ..=, false for ..
    },

    // Array patterns
    Array {
        elements: Vec<SpannedNode<ArrayPatternElement>>,
        rest: Option<String>, // Variable to capture remaining elements
    },

    // Guard pattern (pattern with condition)
    Guard {
        pattern: Box<SpannedNode<MatchPattern>>,
        condition: SpannedNode<Expression>,
    },

    // Type checking pattern
    Type(ParamType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternElement {
    // Exact element pattern
    Pattern(MatchPattern),
    // Wildcard element
    Wildcard,
    // Rest pattern (captures remaining elements)
    Rest(Option<String>), // None for .._, Some(name) for ..name
}

// Enhanced match arm to support variable bindings
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<SpannedNode<Expression>>, // Additional guard condition
    pub body: Vec<SpannedNode<Statement>>,
    pub bindings: HashMap<String, String>, // Maps pattern variables to their types
}

// For expression matches
#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpressionArm {
    pub pattern: MatchPattern,
    pub guard: Option<SpannedNode<Expression>>,
    pub expr: SpannedNode<Expression>,
    pub bindings: HashMap<String, String>,
}

// Helper for pattern matching in runtime
#[derive(Debug, Clone)]
pub struct PatternMatch<T> {
    pub matched: bool,
    pub bindings: HashMap<String, T>,
}

impl MatchPattern {
    /// Check if this pattern can match the given value type
    pub fn can_match_type(&self, value_type: &ParamType) -> bool {
        match (self, value_type) {
            (MatchPattern::String(_), ParamType::String) => true,
            (MatchPattern::Number(_), ParamType::Number) => true,
            (MatchPattern::Bool(_), ParamType::Bool) => true,
            (MatchPattern::Array { .. }, ParamType::Array(_)) => true,
            (MatchPattern::Type(pattern_type), _) => pattern_type == value_type,
            (MatchPattern::Wildcard, _) => true,
            (MatchPattern::Variable(_), _) => true,
            (MatchPattern::Or(patterns), _) => {
                patterns.iter().any(|p| p.value.can_match_type(value_type))
            }
            (MatchPattern::Range { .. }, ParamType::Number) => true,
            (MatchPattern::Guard { pattern, .. }, _) => pattern.value.can_match_type(value_type),
            _ => false,
        }
    }

    /// Get all variable bindings in this pattern
    pub fn get_bindings(&self) -> Vec<String> {
        let mut bindings = Vec::new();
        self.collect_bindings(&mut bindings);
        bindings
    }

    fn collect_bindings(&self, bindings: &mut Vec<String>) {
        match self {
            MatchPattern::Variable(name) => bindings.push(name.clone()),
            MatchPattern::Or(patterns) => {
                for pattern in patterns {
                    pattern.value.collect_bindings(bindings);
                }
            }
            MatchPattern::Array { elements, rest } => {
                for element in elements {
                    match &element.value {
                        ArrayPatternElement::Pattern(pattern) => {
                            pattern.collect_bindings(bindings);
                        }
                        ArrayPatternElement::Rest(Some(name)) => {
                            bindings.push(name.clone());
                        }
                        _ => {}
                    }
                }
                if let Some(rest_name) = rest {
                    bindings.push(rest_name.clone());
                }
            }
            MatchPattern::Guard { pattern, .. } => {
                pattern.value.collect_bindings(bindings);
            }
            _ => {}
        }
    }

    /// Check if this pattern is exhaustive for the given type
    pub fn is_exhaustive_for_type(&self, value_type: &ParamType) -> bool {
        match self {
            MatchPattern::Wildcard | MatchPattern::Variable(_) => true,
            MatchPattern::Type(pattern_type) => pattern_type == value_type,
            MatchPattern::Or(patterns) => {
                // This is simplified - true exhaustiveness checking is complex
                patterns.len() > 1
                    && patterns
                        .iter()
                        .any(|p| p.value.is_exhaustive_for_type(value_type))
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    String(String),
    Number(f64),
    Bool(bool),
    Variable(String),
    FunctionCall {
        module: String,
        function: String,
        args: Vec<SpannedNode<Expression>>,
    },
    ModuleAccess {
        module: String,
        field: String,
    },
    Interpolation(Vec<InterpolationPart>),
    Array(Vec<SpannedNode<Expression>>),
    BinaryOp {
        left: Box<SpannedNode<Expression>>,
        op: BinaryOperator,
        right: Box<SpannedNode<Expression>>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<SpannedNode<Expression>>,
    },
    Conditional {
        condition: Box<SpannedNode<Expression>>,
        then_expr: Box<SpannedNode<Expression>>,
        else_expr: Box<SpannedNode<Expression>>,
    },
    RecipeRef {
        recipe: String,
        args: HashMap<String, SpannedNode<Expression>>,
    },
    Match {
        expr: Box<SpannedNode<Expression>>,
        arms: Vec<SpannedNode<MatchExpressionArm>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationPart {
    String(String),
    Expression(SpannedNode<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
}
