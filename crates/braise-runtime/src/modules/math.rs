use crate::{BraiseType, Result, RuntimeError};
use core::TypedValue;

pub struct MathModule;

impl MathModule {
    pub fn new() -> Self {
        Self
    }

    fn abs(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("abs expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.abs(), BraiseType::Number))
    }

    fn sqrt(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("sqrt expects a number: {e}"), "math")
        })?;
        if num_value < 0.0 {
            return Err(RuntimeError::builtin_error(
                "sqrt expects a non-negative number".to_string(),
                "math",
            ));
        }
        Ok(TypedValue::new(num_value.sqrt(), BraiseType::Number))
    }

    fn rand(&self) -> Result<TypedValue> {
        use rand::Rng;
        let mut rng = rand::rng();
        Ok(TypedValue::new(rng.random::<f64>(), BraiseType::Number))
    }

    fn cos(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("cos expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.cos(), BraiseType::Number))
    }

    fn sin(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("sin expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.sin(), BraiseType::Number))
    }

    fn tan(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("tan expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.tan(), BraiseType::Number))
    }

    fn ln(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("ln expects a number: {e}"), "math")
        })?;
        if num_value <= 0.0 {
            return Err(RuntimeError::builtin_error(
                "ln expects a positive number".to_string(),
                "math",
            ));
        }
        Ok(TypedValue::new(num_value.ln(), BraiseType::Number))
    }

    fn log10(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("log10 expects a number: {e}"), "math")
        })?;
        if num_value <= 0.0 {
            return Err(RuntimeError::builtin_error(
                "log10 expects a positive number".to_string(),
                "math",
            ));
        }
        Ok(TypedValue::new(num_value.log10(), BraiseType::Number))
    }

    fn log2(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("log2 expects a number: {e}"), "math")
        })?;
        if num_value <= 0.0 {
            return Err(RuntimeError::builtin_error(
                "log2 expects a positive number".to_string(),
                "math",
            ));
        }
        Ok(TypedValue::new(num_value.log2(), BraiseType::Number))
    }

    fn exp(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("exp expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.exp(), BraiseType::Number))
    }

    fn floor(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("floor expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.floor(), BraiseType::Number))
    }

    fn ceil(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("ceil expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.ceil(), BraiseType::Number))
    }

    fn round(&self, value: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("round expects a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(num_value.round(), BraiseType::Number))
    }

    fn pi(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(std::f64::consts::PI, BraiseType::Number))
    }
}

builtin_module! {
    MathModule {
        functions: {
            "rand" => rand,
            "cos" => cos(BraiseType::Number),
            "sin" => sin(BraiseType::Number),
            "abs" => abs(BraiseType::Number),
            "sqrt" => sqrt(BraiseType::Number),
            "floor" => floor(BraiseType::Number),
            "ceil" => ceil(BraiseType::Number),
            "round" => round(BraiseType::Number),
            "ln" => ln(BraiseType::Number),
            "log10" => log10(BraiseType::Number),
            "log2" => log2(BraiseType::Number),
            "exp" => exp(BraiseType::Number),
            "tan" => tan(BraiseType::Number),
        },
        fields: {
            "PI" => pi,
        }
    }
}
