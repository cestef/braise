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
            )
            .boxed());
        }
        Ok(TypedValue::new(num_value.sqrt(), BraiseType::Number))
    }

    fn rand(&self, min: TypedValue, max: TypedValue) -> Result<TypedValue> {
        let min_num = min.to_number().unwrap_or(0.0);
        let max_num = max.to_number().unwrap_or(1.0);

        if min_num >= max_num {
            return Err(RuntimeError::builtin_error(
                "rand expects min to be less than max".to_string(),
                "math",
            )
            .boxed());
        }

        let random_value = rand::random::<f64>() * (max_num - min_num) + min_num;
        Ok(TypedValue::new(random_value, BraiseType::Number))
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
            )
            .boxed());
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
            )
            .boxed());
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
            )
            .boxed());
        }
        Ok(TypedValue::new(num_value.log2(), BraiseType::Number))
    }

    fn log(&self, value: TypedValue, base: TypedValue) -> Result<TypedValue> {
        let num_value = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("log expects a number: {e}"), "math")
        })?;
        let base_value = base.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("log expects a base number: {e}"), "math")
        })?;
        if num_value <= 0.0 || base_value <= 1.0 {
            return Err(RuntimeError::builtin_error(
                "log expects a positive number and base greater than 1".to_string(),
                "math",
            )
            .boxed());
        }
        Ok(TypedValue::new(
            num_value.log(base_value),
            BraiseType::Number,
        ))
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

    // Multi-argument functions
    fn pow(&self, base: TypedValue, exponent: TypedValue) -> Result<TypedValue> {
        let base_num = base.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("pow expects base to be a number: {e}"), "math")
        })?;
        let exp_num = exponent.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("pow expects exponent to be a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(base_num.powf(exp_num), BraiseType::Number))
    }

    fn min(&self, a: TypedValue, b: TypedValue) -> Result<TypedValue> {
        let a_num = a.to_number().map_err(|e| {
            RuntimeError::builtin_error(
                format!("min expects first argument to be a number: {e}"),
                "math",
            )
        })?;
        let b_num = b.to_number().map_err(|e| {
            RuntimeError::builtin_error(
                format!("min expects second argument to be a number: {e}"),
                "math",
            )
        })?;
        Ok(TypedValue::new(a_num.min(b_num), BraiseType::Number))
    }

    fn max(&self, a: TypedValue, b: TypedValue) -> Result<TypedValue> {
        let a_num = a.to_number().map_err(|e| {
            RuntimeError::builtin_error(
                format!("max expects first argument to be a number: {e}"),
                "math",
            )
        })?;
        let b_num = b.to_number().map_err(|e| {
            RuntimeError::builtin_error(
                format!("max expects second argument to be a number: {e}"),
                "math",
            )
        })?;
        Ok(TypedValue::new(a_num.max(b_num), BraiseType::Number))
    }

    fn clamp(
        &self,
        value: TypedValue,
        min_val: TypedValue,
        max_val: TypedValue,
    ) -> Result<TypedValue> {
        let val_num = value.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("clamp expects value to be a number: {e}"), "math")
        })?;
        let min_num = min_val.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("clamp expects min to be a number: {e}"), "math")
        })?;
        let max_num = max_val.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("clamp expects max to be a number: {e}"), "math")
        })?;

        if min_num > max_num {
            return Err(RuntimeError::builtin_error(
                "clamp expects min to be less than or equal to max".to_string(),
                "math",
            )
            .boxed());
        }

        Ok(TypedValue::new(
            val_num.clamp(min_num, max_num),
            BraiseType::Number,
        ))
    }

    fn lerp(&self, start: TypedValue, end: TypedValue, t: TypedValue) -> Result<TypedValue> {
        let start_num = start.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("lerp expects start to be a number: {e}"), "math")
        })?;
        let end_num = end.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("lerp expects end to be a number: {e}"), "math")
        })?;
        let t_num = t.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("lerp expects t to be a number: {e}"), "math")
        })?;

        let result = start_num + (end_num - start_num) * t_num;
        Ok(TypedValue::new(result, BraiseType::Number))
    }

    fn hypot(&self, x: TypedValue, y: TypedValue) -> Result<TypedValue> {
        let x_num = x.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("hypot expects x to be a number: {e}"), "math")
        })?;
        let y_num = y.to_number().map_err(|e| {
            RuntimeError::builtin_error(format!("hypot expects y to be a number: {e}"), "math")
        })?;
        Ok(TypedValue::new(x_num.hypot(y_num), BraiseType::Number))
    }
}

builtin_module! {
    MathModule {
        functions: {
            "rand" => rand(BraiseType::Number, BraiseType::Number),
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
            "log" => log(BraiseType::Number, BraiseType::Number),
            "exp" => exp(BraiseType::Number),
            "tan" => tan(BraiseType::Number),
            "pow" => pow(BraiseType::Number, BraiseType::Number),
            "min" => min(BraiseType::Number, BraiseType::Number),
            "max" => max(BraiseType::Number, BraiseType::Number),
            "hypot" => hypot(BraiseType::Number, BraiseType::Number),
            "clamp" => clamp(BraiseType::Number, BraiseType::Number, BraiseType::Number),
            "lerp" => lerp(BraiseType::Number, BraiseType::Number, BraiseType::Number),
        },
        fields: {
            "PI" => pi,
        }
    }
}
