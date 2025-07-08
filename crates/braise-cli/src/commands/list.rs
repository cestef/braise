use core::Result;
use lexer::tokenize;
use owo_colors::OwoColorize;
use parser::Parser as BraiseParser;

pub fn list_recipes(contents: &str, file: &str) -> Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(tokens, contents.to_string().into(), file.to_string());
    let ast = parser.parse()?;

    if ast.recipes.is_empty() {
        println!("{} {}", "No recipes found in".dimmed(), file.bold());
        return Ok(());
    }

    println!("{}\n", "Available recipes".cyan().bold());
    for recipe in &ast.recipes {
        let deps = if recipe.value.dependencies.is_empty() {
            String::new()
        } else {
            format!(
                " {} {}",
                "→".dimmed(),
                recipe
                    .value
                    .dependencies
                    .iter()
                    .map(|e| e.blue().to_string())
                    .collect::<Vec<_>>()
                    .join(&format!(" {} ", "→".dimmed()))
            )
        };

        print!("  {}{}", recipe.value.name.bold(), deps);

        if !recipe.value.parameters.is_empty() {
            let params: Vec<String> = recipe
                .value
                .parameters
                .iter()
                .map(|p| {
                    if p.value.default.is_some() {
                        format!("{}?", p.value.name.green()).dimmed().to_string()
                    } else {
                        p.value.name.green().to_string()
                    }
                })
                .collect();
            print!(" {}{}{}", "(".dimmed(), params.join(" "), ")".dimmed());
        }
        println!();
    }

    Ok(())
}