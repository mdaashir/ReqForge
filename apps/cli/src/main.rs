//! ReqForge CLI
//!
//! Run collections, execute tests, and manage workspaces from the command line.

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "reqforge")]
#[command(version, about = "ReqForge - API Development CLI", long_about = None)]
struct Cli {
    /// Path to the workspace directory (defaults to current directory)
    #[arg(short, long, global = true, default_value = ".")]
    workspace: String,

    /// Output format
    #[arg(short, long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable coloured output
    Human,
    /// JSON output for scripting
    Json,
    /// JUnit XML for CI systems
    Junit,
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Search the marketplace for plugins
    Search {
        /// Search query (name, id, or description)
        query: Option<String>,
        /// Filter by tag (e.g. --tag auth)
        #[arg(short, long)]
        tag: Option<String>,
        /// Marketplace server URL
        #[arg(short, long, default_value = "https://plugins.reqforge.io")]
        server: String,
    },
    /// Show detailed info for a plugin
    Info {
        /// Plugin ID
        id: String,
        /// Marketplace server URL
        #[arg(short, long, default_value = "https://plugins.reqforge.io")]
        server: String,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Run a request from a file or inline spec
    Run {
        /// Collection ID to run
        #[arg(short, long)]
        collection: String,
        /// Optional request name within the collection to run a single item
        #[arg(short, long)]
        request: Option<String>,
        /// Environment name to activate before running
        #[arg(short, long)]
        env: Option<String>,
    },
    /// List all collections in the workspace
    List,
    /// Validate all collections (parse + schema check)
    Validate,
    /// Show workspace info
    Info,

    /// Start a local mock server
    Mock {
        /// Port to listen on (0 = random)
        #[arg(short, long, default_value_t = 0)]
        port: u16,
    },

    /// Import a collection from a file (Postman, cURL, etc.)
    Import {
        /// Path to the input file
        #[arg(short, long)]
        file: String,
        /// Format (postman, curl, insomnia, bruno). Auto-detected if omitted.
        #[arg(short, long)]
        format: Option<String>,
        /// Name for the imported collection (defaults to original)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Export a collection to JSON or YAML
    Export {
        /// Collection ID to export
        #[arg(short, long)]
        collection: String,
        /// Output format (json, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file path (auto-generated if omitted)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Run tests from a collection
    Test {
        /// Collection ID to test
        #[arg(short, long)]
        collection: String,
        /// Environment name to activate
        #[arg(short, long)]
        env: Option<String>,
    },

    /// Search and install plugins from the marketplace
    #[command(subcommand)]
    Plugin(PluginCommands),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run {
            collection,
            request,
            env,
        } => {
            commands::run::execute(
                &cli.workspace,
                &collection,
                request.as_deref(),
                env.as_deref(),
                cli.format,
            )
            .await
        }
        Commands::List => commands::list::execute(&cli.workspace, cli.format).await,
        Commands::Validate => commands::validate::execute(&cli.workspace, cli.format).await,
        Commands::Info => commands::info::execute(&cli.workspace, cli.format).await,
        Commands::Mock { port } => commands::mock::execute(&cli.workspace, Some(port)).await,
        Commands::Import { file, format, name } => {
            commands::import_::execute(&cli.workspace, &file, format.as_deref(), name.as_deref())
                .await
        }
        Commands::Export {
            collection,
            format,
            output,
        } => {
            commands::export::execute(&cli.workspace, &collection, &format, output.as_deref()).await
        }
        Commands::Test { collection, env } => {
            commands::test::execute(&cli.workspace, &collection, env.as_deref(), cli.format).await
        }
        Commands::Plugin(ref sub) => commands::plugin::execute(sub, cli.format).await,
    };

    if let Err(err) = result {
        output::print_error(&err.to_string(), cli.format);
        std::process::exit(1);
    }

    Ok(())
}
