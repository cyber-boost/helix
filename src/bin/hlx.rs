use clap::{Parser, Subcommand, CommandFactory};
use std::process;
use helix::dna::{atp, bch, cmd, exp, ffi, hel, map, mds, ngs, ops, out};

#[derive(Parser)]
#[command(name = "hlx")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "HELIX Compiler - Configuration without the pain")]
#[command(long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    #[arg(short, long, global = true)]
    quiet: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add new files or entries
    Add(cmd::add::AddArgs),
    /// Benchmark project
    Bench(cmd::bench::BenchArgs),
    /// Bundle project files
    Bundle(cmd::bundle::BundleArgs),
    /// Manage cache
    Cache(cmd::cache::CacheArgs),
    /// Clean project
    Clean(cmd::clean::CleanArgs),
    /// Compile project
    Compile(cmd::compile::CompileArgs),
    /// Shell completions
    Completions(cmd::completions::CompletionsArgs),
    /// Dataset operations
    Dataset(cmd::dataset::DatasetArgs),
    /// Show diff between files or versions
    Diff(cmd::diff::DiffArgs),
    /// Run diagnostics
    Doctor(cmd::doctor::DoctorArgs),
    /// Export data or files
    Export(cmd::export::ExportArgs),
    /// Filter operations
    Filter(cmd::filter::FilterArgs),
    /// Format code
    Fmt(cmd::fmt::FmtArgs),
    /// Generate code or files
    Generate(cmd::generate::GenerateArgs),
    /// Import data or files
    Import(cmd::import::ImportArgs),
    /// Show project or file info
    Info(cmd::info::InfoArgs),
    /// Initialize new project
    Init(cmd::init::InitArgs),
    /// Lint code
    Lint(cmd::lint::LintArgs),
    /// Optimize project or files
    Optimize(cmd::optimizer::OptimizeArgs),
    /// Publish project
    Publish(cmd::publish::PublishArgs),
    /// Remove files or entries
    Remove(cmd::rm::RemoveArgs),
    /// Reset project or configuration to default state
    Reset(cmd::reset::ResetArgs),
    /// Schema operations
    Schema(cmd::schema::SchemaArgs),
    /// Search operations
    Search(cmd::search::SearchArgs),
    /// Serve project
    Serve(cmd::serve::ServeArgs),
    /// Sign files or releases
    Sign(cmd::sign::SignArgs),
    /// Test project
    Test(cmd::test::TestArgs),
    /// Validate project or files
    Validate(cmd::validate::ValidateArgs),
    /// Watch files for changes
    Watch(cmd::watch::WatchArgs),
    /// Workflow operations
    Workflow(cmd::workflow::WorkflowArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Add(args) => cmd::add::run(args).await,
        Commands::Bench(args) => cmd::bench::run(args),
        Commands::Bundle(args) => cmd::bundle::run(args),
        Commands::Cache(args) => cmd::cache::run(args),
        Commands::Clean(args) => cmd::clean::run(args).await,
        Commands::Compile(args) => cmd::compile::run(args),
        Commands::Completions(args) => cmd::completions::run(args),
        Commands::Dataset(args) => cmd::dataset::run(args).await,
        Commands::Diff(args) => cmd::diff::run(args),
        Commands::Doctor(args) => cmd::doctor::run(args),
        Commands::Export(args) => cmd::export::run(args),
        Commands::Filter(args) => cmd::filter::run(args).await,
        Commands::Fmt(args) => cmd::fmt::run(args),
        Commands::Generate(args) => cmd::generate::run(args),
        Commands::Import(args) => cmd::import::run(args).await,
        Commands::Info(args) => cmd::info::run(args),
        Commands::Init(args) => cmd::init::run(cmd::init::InitInstallArgs::Init(args)),
        Commands::Lint(args) => cmd::lint::run(args).await,
        Commands::Optimize(args) => cmd::optimizer::run(args),
        Commands::Publish(args) => cmd::publish::run(args),
        Commands::Remove(args) => cmd::rm::run(args).await,
        Commands::Reset(args) => cmd::reset::run(args).await,
        Commands::Schema(args) => cmd::schema::run(args),
        Commands::Search(args) => cmd::search::run(args),
        Commands::Serve(args) => cmd::serve::run(args),
        Commands::Sign(args) => cmd::sign::run(args),
        Commands::Test(args) => cmd::test::run(args).await,
        Commands::Validate(args) => cmd::validate::run(args),
        Commands::Watch(args) => cmd::watch::run(args),
        Commands::Workflow(args) => cmd::workflow::run(args),
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

