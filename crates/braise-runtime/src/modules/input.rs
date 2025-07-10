use crate::{BraiseType, Result, RuntimeError};
use core::{TypedValue, ValueData};
use inquire::{Confirm, CustomType, MultiSelect, Password, Select, Text};

pub struct InputModule;

impl InputModule {
    pub fn new() -> Self {
        Self
    }

    fn text(&self, prompt: TypedValue) -> Result<TypedValue> {
        let prompt_text = prompt.to_string();

        let result = Text::new(&prompt_text).prompt().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read text input: {e}"), "input")
        })?;

        Ok(TypedValue::new(result, BraiseType::String))
    }

    fn num(&self, prompt: TypedValue) -> Result<TypedValue> {
        let prompt_text = prompt.to_string();

        let result = CustomType::<f64>::new(&prompt_text)
            .with_error_message("Please enter a valid number")
            .prompt()
            .map_err(|e| {
                RuntimeError::builtin_error(format!("Failed to read number input: {e}"), "input")
            })?;

        Ok(TypedValue::new(result, BraiseType::Number))
    }

    fn confirm(&self, prompt: TypedValue) -> Result<TypedValue> {
        let prompt_text = prompt.to_string();

        let result = Confirm::new(&prompt_text)
            .with_default(false)
            .prompt()
            .map_err(|e| {
                RuntimeError::builtin_error(format!("Failed to read confirmation: {e}"), "input")
            })?;

        Ok(TypedValue::new(result, BraiseType::Bool))
    }

    fn select(&self, args: TypedValue) -> Result<TypedValue> {
        if let ValueData::Array(options) = &args.value {
            if options.is_empty() {
                return Err(RuntimeError::builtin_error(
                    "Select function requires at least one option".to_string(),
                    "input",
                )
                .boxed());
            }

            let string_options: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();

            let result = Select::new("Please select an option:", string_options)
                .prompt()
                .map_err(|e| {
                    RuntimeError::builtin_error(format!("Failed to read selection: {e}"), "input")
                })?;

            Ok(TypedValue::new(result, BraiseType::String))
        } else {
            Err(RuntimeError::builtin_error(
                "Select function requires an array of options".to_string(),
                "input",
            )
            .boxed())
        }
    }

    fn multiselect(&self, args: TypedValue) -> Result<TypedValue> {
        if let ValueData::Array(options) = &args.value {
            if options.is_empty() {
                return Err(RuntimeError::builtin_error(
                    "Multiselect function requires at least one option".to_string(),
                    "input",
                )
                .boxed());
            }

            let string_options: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();

            let selected = MultiSelect::new("Please select options:", string_options)
                .prompt()
                .map_err(|e| {
                    RuntimeError::builtin_error(
                        format!("Failed to read multiselection: {e}"),
                        "input",
                    )
                })?;

            let selected_values: Vec<TypedValue> = selected
                .into_iter()
                .map(|s| TypedValue::new(s, BraiseType::String))
                .collect();

            Ok(TypedValue::new(
                selected_values,
                BraiseType::Array(Box::new(BraiseType::String)),
            ))
        } else {
            Err(RuntimeError::builtin_error(
                "Multiselect function requires an array of options".to_string(),
                "input",
            )
            .boxed())
        }
    }

    fn password(&self, prompt: TypedValue) -> Result<TypedValue> {
        let prompt_text = prompt.to_string();

        let result = Password::new(&prompt_text)
            .without_confirmation()
            .prompt()
            .map_err(|e| {
                RuntimeError::builtin_error(format!("Failed to read password: {e}"), "input")
            })?;

        Ok(TypedValue::new(result, BraiseType::String))
    }
}

builtin_module! {
    InputModule {
        functions: {
            "text" => text(BraiseType::String),
            "num" => num(BraiseType::String),
            "confirm" => confirm(BraiseType::String),
            "select" => select(BraiseType::Array(Box::new(BraiseType::String))),
            "multiselect" => multiselect(BraiseType::Array(Box::new(BraiseType::String))),
            "password" => password(BraiseType::String),
        }
    }
}
