use crate::{Span, Spanned};

pub type SpannedNode<T> = Spanned<T>;

#[derive(Debug, Clone)]
pub struct Config {
    pub recipes: Vec<SpannedNode<Recipe>>,
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

#[derive(Debug, Clone)]
pub enum ParamType {
    String,
    Number,
    Bool,
    Array(Box<ParamType>),
    Enum(Vec<String>),
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
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<SpannedNode<Statement>>,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    String(String),
    Wildcard,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub enum InterpolationPart {
    String(String),
    Expression(SpannedNode<Expression>),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
}
