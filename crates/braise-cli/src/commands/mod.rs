use clap::{Parser, Subcommand};

pub mod format;
pub mod info;
pub mod list;
pub mod run;

pub use format::format_recipe;
pub use info::show_recipe_info;
pub use list::list_recipes;
pub use run::run_recipe;

#[derive(Parser, Debug)]
#[command(name = "braise")]
#[command(about = "Run your tasks like a chef!")]
#[command(version)]
pub struct Cli {
    /// Path to the recipe file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Dry run mode
    #[arg(short, long)]
    pub dry: bool,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,

    /// Disable caching entirely
    #[arg(long)]
    pub no_cache: bool,

    /// Cache directory path
    #[arg(long)]
    pub cache_dir: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    #[command(external_subcommand)]
    External(Vec<String>),
}
