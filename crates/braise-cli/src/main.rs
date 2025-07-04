use clap::{Parser, Subcommand};
use core::{cli::CliError, constants::DEFAULT_FILES};
use lexer::tokenize;
use parser::Parser as BraiseParser;
use runtime::Runtime;

mod utils;

#[derive(Parser, Debug)]
#[command(name = "braise")]
#[command(about = "Run your tasks like a chef!")]
#[command(version)]
struct Cli {
    /// Path to the recipe file
    #[arg(short, long, global = true)]
    file: Option<String>,

    /// Dry run mode
    #[arg(short, long, global = true)]
    dry: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Recipe name (when no subcommand is used)
    #[arg(value_name = "RECIPE")]
    recipe: Option<String>,

    /// Recipe parameters
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recipes
    #[command(alias = "ls")]
    List,
    /// Format recipe file
    #[command(alias = "fmt")]
    Format {
        /// Output to stdout
        #[arg(long)]
        stdout: bool,
    },
    /// Start LSP server
    Lsp,
    /// Show recipe info
    Info { recipe: String },
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    let file = if let Some(file) = cli.file {
        file
    } else {
        utils::find_first_existing_file(DEFAULT_FILES).ok_or(CliError::NoRecipeFileFound)?
    };

    let contents = std::fs::read_to_string(&file).map_err(|e| CliError::ReadRecipeError {
        src: Box::new(e),
        file: file.clone(),
    })?;

    match cli.command {
        Some(Commands::List) => list_recipes(&contents, &file)?,
        Some(Commands::Format { stdout }) => format_recipe(&contents, &file, stdout)?,
        Some(Commands::Lsp) => start_lsp()?,
        Some(Commands::Info { recipe }) => show_recipe_info(&contents, &file, &recipe)?,
        None => {
            // Default behavior: run a recipe
            if let Some(recipe_name) = cli.recipe {
                run_recipe(&contents, &file, &recipe_name, &cli.args, cli.dry)?;
            } else {
                // No recipe specified, show available recipes
                list_recipes(&contents, &file)?;
            }
        }
    }

    Ok(())
}

fn run_recipe(
    contents: &str,
    file: &str,
    recipe_name: &str,
    args: &[String],
    dry_run: bool,
) -> core::Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(tokens, contents.to_string(), file.to_string());
    let ast = parser.parse()?;

    let mut runtime = Runtime::new(ast);
    if dry_run {
        runtime = runtime.with_dry_run();
    }

    let params = utils::extract_args(args);
    runtime.execute_recipe(recipe_name, params)?;

    Ok(())
}

fn list_recipes(contents: &str, file: &str) -> core::Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(tokens, contents.to_string(), file.to_string());
    let ast = parser.parse()?;

    if ast.recipes.is_empty() {
        println!("No recipes found in {}", file);
        return Ok(());
    }

    println!("📋 Available recipes:");
    for recipe in &ast.recipes {
        let deps = if recipe.value.dependencies.is_empty() {
            String::new()
        } else {
            format!(" → {}", recipe.value.dependencies.join(", "))
        };

        print!("  {}{}", recipe.value.name, deps);

        if !recipe.value.parameters.is_empty() {
            let params: Vec<String> = recipe
                .value
                .parameters
                .iter()
                .map(|p| {
                    if p.value.default.is_some() {
                        format!("[{}]", p.value.name)
                    } else {
                        p.value.name.clone()
                    }
                })
                .collect();
            print!(" ({})", params.join(" "));
        }
        println!();
    }

    Ok(())
}

fn format_recipe(contents: &str, file: &str, stdout: bool) -> core::Result<()> {
    let formatted = fmt::Formatter::format(contents);

    if stdout {
        print!("{}", formatted);
    } else {
        std::fs::write(file, formatted).map_err(|e| CliError::ReadRecipeError {
            src: Box::new(e),
            file: file.to_string(),
        })?;
        println!("✨ Formatted {}", file);
    }

    Ok(())
}

fn start_lsp() -> core::Result<()> {
    println!("🚀 Starting Braise Language Server...");
    // LSP implementation would go here
    Ok(())
}

fn show_recipe_info(contents: &str, file: &str, recipe_name: &str) -> core::Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(tokens, contents.to_string(), file.to_string());
    let ast = parser.parse()?;

    let recipe = ast
        .recipes
        .iter()
        .find(|r| r.value.name == recipe_name)
        .ok_or_else(|| core::runtime::RuntimeError::UndefinedRecipe(recipe_name.to_string()))?;

    println!("🔍 Recipe: {}", recipe.value.name);

    if !recipe.value.dependencies.is_empty() {
        println!("   Dependencies: {}", recipe.value.dependencies.join(" → "));
    }

    if !recipe.value.parameters.is_empty() {
        println!("   Parameters:");
        for param in &recipe.value.parameters {
            let default = if let Some(ref default_expr) = param.value.default {
                format!(" = {}", format_expression_preview(&default_expr.value))
            } else {
                String::new()
            };
            println!(
                "     {}: {}{}",
                param.value.name, param.value.param_type, default
            );
        }
    }

    println!("   Steps: {}", recipe.value.body.len());

    Ok(())
}

fn format_expression_preview(expr: &core::ast::Expression) -> String {
    use core::ast::Expression;
    match expr {
        Expression::String(s) => format!("\"{}\"", s),
        Expression::Number(n) => n.to_string(),
        Expression::Bool(b) => b.to_string(),
        Expression::Variable(name) => name.clone(),
        Expression::FunctionCall {
            module, function, ..
        } => {
            format!("{}.{}()", module, function)
        }
        Expression::ModuleAccess { module, field } => {
            format!("{}.{}", module, field)
        }
        Expression::Array(elements) => {
            if elements.len() <= 2 {
                let items: Vec<String> = elements
                    .iter()
                    .map(|e| format_expression_preview(&e.value))
                    .collect();
                format!("[{}]", items.join(", "))
            } else {
                format!("[{} items]", elements.len())
            }
        }
        _ => "...".to_string(),
    }
}
