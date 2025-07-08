use braise_errors::CliError;
use core::Result;
use owo_colors::OwoColorize;

pub fn format_recipe(contents: &str, file: &str, stdout: bool) -> Result<()> {
    let formatted = fmt::Formatter::format(contents);

    if stdout {
        print!("{formatted}");
    } else {
        std::fs::write(file, formatted)
            .map_err(|e| CliError::read_recipe_error(e, file.to_string()))?;
        println!("{} {}", "Formatted".green().bold(), file.dimmed());
    }

    Ok(())
}
