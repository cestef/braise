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

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<SpannedNode<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    String(String),
    Wildcard,
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
pub struct MatchExpressionArm {
    pub pattern: MatchPattern,
    pub expr: SpannedNode<Expression>,
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
