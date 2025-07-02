#[derive(Debug, Clone)]
pub struct Config {
    pub recipes: Vec<Recipe>,
}

#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub dependencies: Vec<String>,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: ParamType,
    pub default: Option<Expression>,
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
    Run(Expression),
    Print(Expression),
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    Match {
        expr: Expression,
        arms: Vec<MatchArm>,
    },
    For {
        var: String,
        iterable: Expression,
        body: Vec<Statement>,
        is_async: bool,
    },
    Exit(Expression),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Statement>,
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
        args: Vec<Expression>,
    },
    ModuleAccess {
        module: String,
        field: String,
    },
    Interpolation(Vec<InterpolationPart>),
    Array(Vec<Expression>),
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum InterpolationPart {
    String(String),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    // Logical
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
}
