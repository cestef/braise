use crate::{BraiseType, Span, Spanned};
use std::collections::HashMap;

pub type SpannedNode<T> = Spanned<T>;

#[derive(Debug, Clone)]
pub struct Config {
    pub recipes: Vec<SpannedNode<Recipe>>,
    pub shell: Option<String>,
    pub span: Span,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for recipe in &self.recipes {
            writeln!(f, "Recipe: {}", recipe.value.name)?;
            if !recipe.value.dependencies.is_empty() {
                writeln!(f, "  Dependencies: {:?}", recipe.value.dependencies)?;
            }
            for param in &recipe.value.parameters {
                writeln!(
                    f,
                    "  Parameter: {} ({})",
                    param.value.name, param.value.param_type
                )?;
                if let Some(default) = &param.value.default {
                    writeln!(f, "    Default: {}", default.value)?;
                }
            }
            for stmt in &recipe.value.body {
                writeln!(f, "  Statement: {:#?}", stmt.value)?;
            }
        }
        if let Some(shell) = &self.shell {
            writeln!(f, "Shell: {}", shell)?;
        }
        Ok(())
    }
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
    pub param_type: BraiseType,
    pub default: Option<SpannedNode<Expression>>,
    pub optional: bool,
}

impl Parameter {
    /// Create a new parameter
    pub fn new(name: String, param_type: BraiseType) -> Self {
        Self {
            name,
            param_type,
            default: None,
            optional: false,
        }
    }

    /// Make this parameter optional
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Set a default value
    pub fn with_default(mut self, default: SpannedNode<Expression>) -> Self {
        self.default = Some(default);
        self.optional = true;
        self
    }

    pub fn effective_type(&self) -> BraiseType {
        match (&self.param_type, self.has_default()) {
            // If already optional, don't double-wrap
            (BraiseType::Optional(_), _) => self.param_type.clone(),
            // If has default but not explicitly optional, wrap in Optional
            (inner_type, true) if !self.optional => {
                BraiseType::Optional(Box::new(inner_type.clone()))
            }
            // Otherwise use as-is
            _ => self.param_type.clone(),
        }
    }

    /// Check if this parameter has a default value
    pub fn has_default(&self) -> bool {
        self.default.is_some()
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
        param_type: BraiseType,
        inferred_type: Option<BraiseType>,
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
pub enum Expression {
    String(String),
    Number(f64),
    Bool(bool),
    Variable(String),
    FunctionCall {
        module: String,
        function: String,
        args: Vec<SpannedNode<Expression>>,
        return_type: Option<BraiseType>,
    },
    ModuleAccess {
        module: String,
        field: String,
        field_type: Option<BraiseType>,
    },
    Interpolation(Vec<InterpolationPart>),
    Array(Vec<SpannedNode<Expression>>),
    BinaryOp {
        left: Box<SpannedNode<Expression>>,
        op: BinaryOperator,
        right: Box<SpannedNode<Expression>>,
        result_type: Option<BraiseType>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<SpannedNode<Expression>>,
        result_type: Option<BraiseType>,
    },
    Conditional {
        condition: Box<SpannedNode<Expression>>,
        then_expr: Box<SpannedNode<Expression>>,
        else_expr: Box<SpannedNode<Expression>>,
        result_type: Option<BraiseType>,
    },
    RecipeRef {
        recipe: String,
        args: HashMap<String, SpannedNode<Expression>>,
    },
    Match {
        expr: Box<SpannedNode<Expression>>,
        arms: Vec<SpannedNode<MatchExpressionArm>>,
        result_type: Option<BraiseType>,
    },
}

impl Expression {
    /// Get the type of this expression (if known)
    pub fn get_type(&self) -> BraiseType {
        match self {
            Expression::String(_) => BraiseType::String,
            Expression::Number(_) => BraiseType::Number,
            Expression::Bool(_) => BraiseType::Bool,
            Expression::Variable(_) => BraiseType::Any,
            Expression::Array(_) => BraiseType::Array(Box::new(BraiseType::Any)),
            Expression::FunctionCall { return_type, .. } => {
                return_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::ModuleAccess { field_type, .. } => {
                field_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::BinaryOp { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::UnaryOp { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::Conditional { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::Match { result_type, .. } => result_type.clone().unwrap_or(BraiseType::Any),
            Expression::Interpolation(_) => BraiseType::String,
            Expression::RecipeRef { .. } => BraiseType::Recipe,
        }
    }

    /// Set the type information for this expression
    pub fn with_type(mut self, expr_type: BraiseType) -> Self {
        match &mut self {
            Expression::FunctionCall { return_type, .. } => *return_type = Some(expr_type),
            Expression::ModuleAccess { field_type, .. } => *field_type = Some(expr_type),
            Expression::BinaryOp { result_type, .. } => *result_type = Some(expr_type),
            Expression::UnaryOp { result_type, .. } => *result_type = Some(expr_type),
            Expression::Conditional { result_type, .. } => *result_type = Some(expr_type),
            Expression::Match { result_type, .. } => *result_type = Some(expr_type),
            _ => {}
        }
        self
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::String(s) => write!(f, "\"{}\"", s),
            Expression::Number(n) => write!(f, "{}", n),
            Expression::Bool(b) => write!(f, "{}", b),
            Expression::Variable(name) => write!(f, "{}", name),
            Expression::FunctionCall {
                module,
                function,
                args,
                ..
            } => {
                let args_str: Vec<String> = args.iter().map(|a| a.value.to_string()).collect();
                write!(f, "{}.{}({})", module, function, args_str.join(", "))
            }
            Expression::ModuleAccess { module, field, .. } => write!(f, "{}.{}", module, field),
            Expression::Interpolation(parts) => {
                let parts_str: Vec<String> = parts
                    .iter()
                    .map(|part| match part {
                        InterpolationPart::String(s) => s.clone(),
                        InterpolationPart::Expression(expr) => format!("${{{}}}", expr.value),
                    })
                    .collect();
                write!(f, "\"{}\"", parts_str.join(""))
            }
            Expression::Array(elements) => {
                let elems_str: Vec<String> = elements.iter().map(|e| e.value.to_string()).collect();
                write!(f, "[{}]", elems_str.join(", "))
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                write!(f, "({} {:?} {})", left.value, op, right.value)
            }
            Expression::UnaryOp { op, expr, .. } => write!(f, "{:?}({})", op, expr.value),
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                write!(
                    f,
                    "if {} then {} else {}",
                    condition.value, then_expr.value, else_expr.value
                )
            }
            Expression::RecipeRef { recipe, args } => {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.value))
                    .collect();
                write!(f, "{}({})", recipe, args_str.join(", "))
            }
            Expression::Match {
                expr,
                arms,
                result_type,
            } => {
                let arms_str: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let guard = if let Some(guard) = &arm.value.guard {
                            format!(" if {}", guard.value)
                        } else {
                            String::new()
                        };
                        format!(
                            "case {:?}{} => {}",
                            arm.value.pattern, guard, arm.value.expr.value
                        )
                    })
                    .collect();
                let result_type_str = if let Some(rt) = result_type {
                    format!(": {}", rt)
                } else {
                    String::new()
                };
                write!(
                    f,
                    "match {} {{\n{}\n}}{}",
                    expr.value,
                    arms_str.join("\n"),
                    result_type_str
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    String(String),
    Number(f64),
    Bool(bool),
    Wildcard,
    Variable(String),
    Or(Vec<SpannedNode<MatchPattern>>),
    Range {
        start: Option<f64>,
        end: Option<f64>,
        inclusive: bool,
    },
    Array {
        elements: Vec<SpannedNode<ArrayPatternElement>>,
        rest: Option<String>,
    },
    Guard {
        pattern: Box<SpannedNode<MatchPattern>>,
        condition: SpannedNode<Expression>,
    },
    Type(BraiseType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPatternElement {
    Pattern(MatchPattern),
    Wildcard,
    Rest(Option<String>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<SpannedNode<Expression>>,
    pub body: Vec<SpannedNode<Statement>>,
    pub bindings: HashMap<String, BraiseType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpressionArm {
    pub pattern: MatchPattern,
    pub guard: Option<SpannedNode<Expression>>,
    pub expr: SpannedNode<Expression>,
    pub bindings: HashMap<String, BraiseType>,
}

#[derive(Debug, Clone)]
pub struct PatternMatch<T> {
    pub matched: bool,
    pub bindings: HashMap<String, T>,
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

impl BinaryOperator {
    /// Get the result type of this binary operation
    pub fn result_type(&self, _left_type: &BraiseType, _right_type: &BraiseType) -> BraiseType {
        match self {
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::And
            | BinaryOperator::Or => BraiseType::Bool,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
}

impl UnaryOperator {
    /// Get the result type of this unary operation
    pub fn result_type(&self, _operand_type: &BraiseType) -> BraiseType {
        match self {
            UnaryOperator::Not => BraiseType::Bool,
        }
    }
}
