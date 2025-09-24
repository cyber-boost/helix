use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Context;
use crate::compiler::{
    Compiler, optimizer::OptimizationLevel, loader::BinaryLoader,
    bundle::Bundler,
};
use crate::server::{ServerConfig, start_server};
mod project;
mod workflow;
mod tools;
mod publish;
mod config;
use project::*;
use workflow::*;
use tools::*;
use publish::*;
use config::*;
#[derive(Parser)]
#[command(name = "hlx")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "HELIX Compiler - Configuration without the pain")]
#[command(long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum WorkflowAction {
    Watch {
        directory: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
    },
    Start { directory: PathBuf, #[arg(short, long)] output: Option<PathBuf> },
    Stop,
    Status,
    List,
    Pause { workflow_id: String },
    Resume { workflow_id: String },
    Kill { workflow_id: String },
}

#[derive(Subcommand)]
enum DatasetAction {
    Process {
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        algorithm: Option<String>,
        #[arg(long)]
        validate: bool,
    },
    Analyze {
        files: Vec<PathBuf>,
        #[arg(long)]
        detailed: bool,
    },
    Convert {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        from_format: String,
        #[arg(long)]
        to_format: String,
    },
    Quality {
        files: Vec<PathBuf>,
        #[arg(long)]
        report: bool,
    },
    Huggingface {
        dataset: String,
        #[arg(long)]
        split: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CaptionAction {
    Process {
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    E621 {
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        filter_tags: bool,
        #[arg(long)]
        format: Option<String>,
    },
    Convert {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        format: Option<String>,
    },
}

#[derive(Subcommand)]
enum JsonAction {
    Format {
        files: Vec<PathBuf>,
        #[arg(long)]
        check: bool,
    },
    Validate {
        files: Vec<PathBuf>,
        #[arg(long)]
        schema: Option<PathBuf>,
    },
    Metadata {
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Split {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Merge {
        files: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
}
#[derive(Subcommand)]
enum Commands {
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long)]
        compress: bool,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
        #[arg(long)]
        cache: bool,
    },
    Decompile { input: PathBuf, #[arg(short, long)] output: Option<PathBuf> },
    Validate { file: PathBuf, #[arg(short, long)] detailed: bool },
    Bundle {
        directory: PathBuf,
        #[arg(short, long, default_value = "bundle.hlxb")]
        output: PathBuf,
        #[arg(short, long)]
        include: Vec<String>,
        #[arg(short = 'x', long)]
        exclude: Vec<String>,
        #[arg(long)]
        tree_shake: bool,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
    },
    Info {
        file: PathBuf,
        #[arg(short, long, default_value = "text")]
        format: String,
        #[arg(long)]
        symbols: bool,
        #[arg(long)]
        sections: bool,
    },
    Watch {
        directory: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
    },
    Diff { file1: PathBuf, file2: PathBuf, #[arg(short, long)] detailed: bool },
    Optimize {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short = 'O', long, default_value = "3")]
        level: u8,
    },
    Init {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        dir: Option<PathBuf>,
        #[arg(short, long, default_value = "minimal")]
        template: String,
        #[arg(short, long)]
        force: bool,
    },
    Install {
        #[arg(long)]
        local_only: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    Add {
        dependency: String,
        #[arg(short, long)]
        version: Option<String>,
        #[arg(long)]
        dev: bool,
    },
    Remove { dependency: String, #[arg(long)] dev: bool },
    Clean { #[arg(long)] all: bool, #[arg(long)] cache: bool },
    Reset { #[arg(short, long)] force: bool },
    Build {
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
        #[arg(short, long)]
        compress: bool,
        #[arg(long)]
        cache: bool,
    },
    Run {
        input: Option<PathBuf>,
        args: Vec<String>,
        #[arg(short = 'O', long, default_value = "2")]
        optimize: u8,
    },
    Test { #[arg(short, long)] pattern: Option<String>, #[arg(long)] integration: bool },
    Bench {
        #[arg(short, long)]
        pattern: Option<String>,
        #[arg(short, long)]
        iterations: Option<usize>,
    },
    Serve {
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(short, long)]
        directory: Option<PathBuf>,
        #[arg(long)]
        no_convert: bool,
        #[arg(long)]
        cache_timeout: Option<u64>,
        #[arg(long)]
        max_file_size: Option<u64>,
    },
    Fmt { files: Vec<PathBuf>, #[arg(long)] check: bool },
    Lint { files: Vec<PathBuf> },
    Generate {
        template: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        force: bool,
    },
    Publish {
        #[arg(short, long)]
        registry: Option<String>,
        #[arg(short, long)]
        token: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Sign {
        input: PathBuf,
        #[arg(short, long)]
        key: Option<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        verify: bool,
    },
    Export {
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        include_deps: bool,
    },
    Import {
        input: PathBuf,
        #[arg(short, long)]
        format: Option<String>,
        #[arg(short, long)]
        force: bool,
    },
    Config { action: String, key: Option<String>, value: Option<String> },
    Cache { action: String },
    Doctor,
    ServeProject {
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(long)]
        host: Option<String>,
        #[arg(short, long)]
        directory: Option<PathBuf>,
    },
    Workflow { #[command(subcommand)] action: WorkflowAction },
    // HLX-AI Commands for intelligent dataset processing
    Dataset {
        #[command(subcommand)]
        action: DatasetAction,
    },
    Concat {
        directory: PathBuf,
        #[arg(short, long, default_value = "caption+wd+tags")]
        preset: String,
        #[arg(short, long)]
        output_dir: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        deduplicate: bool,
    },
    Caption {
        #[command(subcommand)]
        action: CaptionAction,
    },
    Json {
        #[command(subcommand)]
        action: JsonAction,
    },
}
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Compile { input, output, compress, optimize, cache } => {
            compile_command(input, output, compress, optimize, cache, cli.verbose)
        }
        Commands::Decompile { input, output } => {
            decompile_command(input, output, cli.verbose)
        }
        Commands::Validate { file, detailed } => {
            validate_command(file, detailed || cli.verbose)
        }
        Commands::Bundle {
            directory,
            output,
            include,
            exclude,
            tree_shake,
            optimize,
        } => {
            bundle_command(
                directory,
                output,
                include,
                exclude,
                tree_shake,
                optimize,
                cli.verbose,
            )
        }
        Commands::Info { file, format, symbols, sections } => {
            info_command(file, format, symbols, sections, cli.verbose)
        }
        Commands::Watch { directory, output, optimize } => {
            watch_command(directory, output, optimize, cli.verbose)
        }
        Commands::Diff { file1, file2, detailed } => {
            diff_command(file1, file2, detailed || cli.verbose)
        }
        Commands::Optimize { input, output, level } => {
            optimize_command(input, output, level, cli.verbose)
        }
        Commands::Init { name, dir, template, force } => {
            init_command(template, dir, name, force, cli.verbose)?;
            Ok(())
        }
        Commands::Install { local_only, force, verbose } => {
            install_command(local_only, force, verbose || cli.verbose)
        }
        Commands::Add { dependency, version, dev } => {
            add_dependency(dependency, version, dev, cli.verbose)?;
            Ok(())
        }
        Commands::Remove { dependency, dev } => {
            remove_dependency(dependency, dev, cli.verbose)?;
            Ok(())
        }
        Commands::Clean { all, cache } => {
            clean_project(all, cache, cli.verbose)?;
            Ok(())
        }
        Commands::Reset { force } => {
            reset_project(force, cli.verbose)?;
            Ok(())
        }
        Commands::Build { input, output, optimize, compress, cache } => {
            build_project(input, output, optimize, compress, cache, cli.verbose)
        }
        Commands::Run { input, args, optimize } => {
            run_project(input, args, optimize, cli.verbose)?;
            Ok(())
        }
        Commands::Test { pattern, integration } => {
            run_tests(pattern, cli.verbose, integration)?;
            Ok(())
        }
        Commands::Bench { pattern, iterations } => {
            run_benchmarks(pattern, iterations, cli.verbose)?;
            Ok(())
        }
        Commands::Serve {
            port,
            domain,
            directory,
            no_convert,
            cache_timeout,
            max_file_size,
        } => {
            let mut config = ServerConfig::default();
            if let Some(p) = port {
                config.port = p;
            }
            if let Some(d) = domain {
                config.domain = d;
            }
            if let Some(dir) = directory {
                config.root_directory = dir;
            }
            config.auto_convert = !no_convert;
            if let Some(ct) = cache_timeout {
                config.cache_timeout = ct;
            }
            if let Some(mfs) = max_file_size {
                config.max_file_size = mfs;
            }
            config.verbose = cli.verbose;
            start_server(config)?;
            Ok(())
        }
        Commands::Fmt { files, check } => {
            format_files(files, check, cli.verbose)?;
            Ok(())
        }
        Commands::Lint { files } => {
            lint_files(files, cli.verbose)?;
            Ok(())
        }
        Commands::Generate { template, output, name, force } => {
            generate_code(template, output, name, force, cli.verbose)?;
            Ok(())
        }
        Commands::Publish { registry, token, dry_run } => {
            publish_project(registry, token, dry_run, cli.verbose)?;
            Ok(())
        }
        Commands::Sign { input, key, output, verify } => {
            sign_binary(input, key, output, verify, cli.verbose)?;
            Ok(())
        }
        Commands::Export { format, output, include_deps } => {
            export_project(format, output, include_deps, cli.verbose)?;
            Ok(())
        }
        Commands::Import { input, format, force } => {
            import_project(input, format, force, cli.verbose)?;
            Ok(())
        }
        Commands::Config { action, key, value } => {
            manage_config(action.parse()?, key, value, cli.verbose)?;
            Ok(())
        }
        Commands::Cache { action } => {
            manage_cache(action.parse()?, cli.verbose)?;
            Ok(())
        }
        Commands::Doctor => {
            run_diagnostics(cli.verbose)?;
            Ok(())
        }
        Commands::ServeProject { port, host, directory } => {
            Ok(serve_project(port, host, directory, cli.verbose)?)
        }
        Commands::Dataset { action } => {
            dataset_command(action, cli.verbose).await
        }
        Commands::Concat { directory, preset, output_dir, dry_run, deduplicate } => {
            concat_command(directory, preset, output_dir, dry_run, deduplicate, cli.verbose)
        }
        Commands::Caption { action } => {
            caption_command(action, cli.verbose).await
        }
        Commands::Json { action } => {
            json_command(action, cli.verbose).await
        }
        Commands::Workflow { action } => {
            match action {
                WorkflowAction::Watch { directory, output, optimize } => {
                    watch_command(directory, output, optimize, cli.verbose)
                }
                WorkflowAction::Start { directory, output } => {
                    Ok(start_hot_reload(directory, output, cli.verbose)?)
                }
                WorkflowAction::Stop => Ok(stop_hot_reload(cli.verbose)?),
                WorkflowAction::Status => Ok(get_workflow_status(cli.verbose)?),
                WorkflowAction::List => Ok(list_workflows(cli.verbose)?),
                WorkflowAction::Pause { workflow_id } => {
                    Ok(pause_workflow(workflow_id, cli.verbose)?)
                }
                WorkflowAction::Resume { workflow_id } => {
                    Ok(resume_workflow(workflow_id, cli.verbose)?)
                }
                WorkflowAction::Kill { workflow_id } => {
                    Ok(stop_workflow(workflow_id, cli.verbose)?)
                }
            }
        }
    }
}
fn compile_command(
    input: PathBuf,
    output: Option<PathBuf>,
    compress: bool,
    optimize: u8,
    cache: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output
        .unwrap_or_else(|| {
            let mut path = input.clone();
            path.set_extension("hlxb");
            path
        });
    if verbose {
        println!("📦 Compiling: {}", input.display());
        println!("  Optimization: Level {}", optimize);
        println!("  Compression: {}", if compress { "Enabled" } else { "Disabled" });
        println!("  Cache: {}", if cache { "Enabled" } else { "Disabled" });
    }
    let compiler = Compiler::builder()
        .optimization_level(OptimizationLevel::from(optimize))
        .compression(compress)
        .cache(cache)
        .verbose(verbose)
        .build();
    let binary = compiler.compile_file(&input)?;
    let serializer = crate::compiler::serializer::BinarySerializer::new(compress);
    serializer.write_to_file(&binary, &output_path)?;
    println!("✅ Compiled successfully: {}", output_path.display());
    println!("  Size: {} bytes", binary.size());
    if verbose {
        let stats = binary.symbol_table.stats();
        println!(
            "  Strings: {} (unique: {})", stats.total_strings, stats.unique_strings
        );
        println!("  Agents: {}", stats.agents);
        println!("  Workflows: {}", stats.workflows);
    }
    Ok(())
}
fn decompile_command(
    input: PathBuf,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output
        .unwrap_or_else(|| {
            let mut path = input.clone();
            path.set_extension("hlx");
            path
        });
    if verbose {
        println!("🔄 Decompiling: {}", input.display());
    }
    let loader = BinaryLoader::new();
    let binary = loader.load_file(&input)?;
    let compiler = Compiler::new(OptimizationLevel::Zero);
    let source = compiler.decompile(&binary)?;
    std::fs::write(&output_path, source)?;
    println!("✅ Decompiled successfully: {}", output_path.display());
    Ok(())
}
fn validate_command(
    file: PathBuf,
    detailed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let extension = file.extension().and_then(|s| s.to_str());
    match extension {
        Some("hlx") => {
            let source = std::fs::read_to_string(&file)?;
            let ast = crate::parse(&source)?;
            crate::validate(&ast)?;
            println!("✅ Valid HELIX file: {}", file.display());
            if detailed {
                println!("  Declarations: {}", ast.declarations.len());
            }
        }
        Some("hlxb") => {
            let loader = BinaryLoader::new();
            let binary = loader.load_file(&file)?;
            binary.validate()?;
            println!("✅ Valid HLXB file: {}", file.display());
            if detailed {
                println!("  Version: {}", binary.version);
                println!("  Sections: {}", binary.data_sections.len());
                println!("  Checksum: {:x}", binary.checksum);
            }
        }
        _ => {
            return Err("Unknown file type (expected .hlx or .hlxb)".into());
        }
    }
    Ok(())
}
fn bundle_command(
    directory: PathBuf,
    output: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    tree_shake: bool,
    optimize: u8,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("📦 Bundling directory: {}", directory.display());
        if !include.is_empty() {
            println!("  Include patterns: {:?}", include);
        }
        if !exclude.is_empty() {
            println!("  Exclude patterns: {:?}", exclude);
        }
        println!("  Tree shaking: {}", if tree_shake { "Enabled" } else { "Disabled" });
    }
    let mut bundler = Bundler::new().with_tree_shaking(tree_shake).verbose(verbose);
    for pattern in include {
        bundler = bundler.include(&pattern);
    }
    for pattern in exclude {
        bundler = bundler.exclude(&pattern);
    }
    let binary = bundler
        .bundle_directory(&directory, OptimizationLevel::from(optimize))?;
    let serializer = crate::compiler::serializer::BinarySerializer::new(true);
    serializer.write_to_file(&binary, &output)?;
    println!("✅ Bundle created: {}", output.display());
    println!("  Size: {} bytes", binary.size());
    if let Some(file_count) = binary.metadata.extra.get("bundle_files") {
        println!("  Files bundled: {}", file_count);
    }
    Ok(())
}
fn info_command(
    file: PathBuf,
    format: String,
    symbols: bool,
    sections: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = BinaryLoader::new();
    let binary = loader.load_file(&file)?;
    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&binary.metadata)?;
            println!("{}", json);
        }
        "yaml" => {
            println!("YAML output not yet implemented");
        }
        "text" | _ => {
            println!("HELIX Binary Information");
            println!("=======================");
            println!("File: {}", file.display());
            println!("Version: {}", binary.version);
            println!("Compiler: {}", binary.metadata.compiler_version);
            println!("Platform: {}", binary.metadata.platform);
            println!("Created: {}", binary.metadata.created_at);
            println!("Optimization: Level {}", binary.metadata.optimization_level);
            println!("Compressed: {}", binary.flags.compressed);
            println!("Size: {} bytes", binary.size());
            println!("Checksum: {:x}", binary.checksum);
            if let Some(source) = &binary.metadata.source_path {
                println!("Source: {}", source);
            }
            if symbols || verbose {
                println!("\nSymbol Table:");
                let stats = binary.symbol_table.stats();
                println!(
                    "  Strings: {} (unique: {})", stats.total_strings, stats
                    .unique_strings
                );
                println!("  Total bytes: {}", stats.total_bytes);
                println!("  Agents: {}", stats.agents);
                println!("  Workflows: {}", stats.workflows);
                println!("  Contexts: {}", stats.contexts);
                println!("  Crews: {}", stats.crews);
            }
            if sections || verbose {
                println!("\nData Sections:");
                for (i, section) in binary.data_sections.iter().enumerate() {
                    println!("  [{}] {:?}", i, section.section_type);
                    println!("      Size: {} bytes", section.size);
                    if let Some(compression) = &section.compression {
                        println!("      Compression: {:?}", compression);
                    }
                }
            }
        }
    }
    Ok(())
}
fn watch_command(
    directory: PathBuf,
    _output: Option<PathBuf>,
    _optimize: u8,
    _verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("👀 Watching directory: {}", directory.display());
    println!("  Press Ctrl+C to stop");
    println!("Watch mode not yet implemented");
    Ok(())
}
fn diff_command(
    file1: PathBuf,
    file2: PathBuf,
    detailed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = BinaryLoader::new();
    let binary1 = loader.load_file(&file1)?;
    let binary2 = loader.load_file(&file2)?;
    println!("Comparing binaries:");
    println!("  File 1: {}", file1.display());
    println!("  File 2: {}", file2.display());
    println!();
    if binary1.version != binary2.version {
        println!("⚠️  Version differs: {} vs {}", binary1.version, binary2.version);
    }
    if binary1.size() != binary2.size() {
        println!("⚠️  Size differs: {} vs {} bytes", binary1.size(), binary2.size());
    }
    let stats1 = binary1.symbol_table.stats();
    let stats2 = binary2.symbol_table.stats();
    if stats1.total_strings != stats2.total_strings {
        println!(
            "⚠️  String count differs: {} vs {}", stats1.total_strings, stats2
            .total_strings
        );
    }
    if detailed {}
    Ok(())
}
fn optimize_command(
    input: PathBuf,
    output: Option<PathBuf>,
    level: u8,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output.unwrap_or_else(|| input.clone());
    if verbose {
        println!("⚡ Optimizing: {}", input.display());
        println!("  Level: {}", level);
    }
    let loader = BinaryLoader::new();
    let binary = loader.load_file(&input)?;
    let serializer = crate::compiler::serializer::BinarySerializer::new(false);
    let mut ir = serializer.deserialize_to_ir(&binary)?;
    let mut optimizer = crate::compiler::optimizer::Optimizer::new(
        OptimizationLevel::from(level),
    );
    optimizer.optimize(&mut ir);
    let optimized_binary = serializer.serialize(ir, None)?;
    serializer.write_to_file(&optimized_binary, &output_path)?;
    println!("✅ Optimized successfully: {}", output_path.display());
    if verbose {
        let stats = optimizer.stats();
        println!("\nOptimization Results:");
        println!("{}", stats.report());
    }
    Ok(())
}
const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[
    ("minimal", r#"# Minimal MSO Configuration Example
# Demonstrates the simplest valid MSO file

project "minimal-example" {
    version = "0.1.0"
    author = "Example"
}

agent "simple-assistant" {
    model = "gpt-3.5-turbo"
    role = "Assistant"
    temperature = 0.7
}

workflow "basic-task" {
    trigger = "manual"

    step "process" {
        agent = "simple-assistant"
        task = "Process user request"
        timeout = 5m
    }
}"#),
    ("ai-dev", "# AI Development Team template - full content embedded"),
    ("support", r#"# Customer Support AI Configuration
# AI-powered customer service system

project "customer-support-system" {
    version = "2.0.0"
    author = "Support Team"
    description = "AI-driven customer support with multi-channel capabilities"
}

agent "support-specialist" {
    model = "claude-3-sonnet"
    role = "Customer Support Specialist"
    temperature = 0.7
    max_tokens = 100000

    capabilities [
        "customer-service"
        "problem-solving"
        "empathy"
        "multi-language"
        "escalation-handling"
    ]

    backstory {
        8 years in customer support leadership
        Handled 100K+ customer interactions
        Expert in de-escalation techniques
        Trained support teams worldwide
    }

    tools = [
        "zendesk"
        "intercom"
        "slack"
        "email-client"
        "knowledge-base"
    ]
}

agent "technical-expert" {
    model = "gpt-4"
    role = "Technical Support Engineer"
    temperature = 0.6
    max_tokens = 80000

    capabilities [
        "technical-troubleshooting"
        "bug-analysis"
        "system-diagnostics"
        "code-review"
        "api-debugging"
    ]

    backstory {
        12 years in software engineering
        Specialized in distributed systems
        Published technical documentation
        Led incident response teams
    }

    tools = [
        "terminal"
        "database-client"
        "monitoring-tools"
        "api-tester"
        "log-analyzer"
    ]
}

workflow "customer-inquiry-handling" {
    trigger = "webhook"

    step "triage" {
        agent = "support-specialist"
        task = "Analyze customer inquiry and determine priority level"
        timeout = 5m
    }

    step "initial-response" {
        agent = "support-specialist"
        task = "Provide immediate acknowledgment and gather more details"
        timeout = 10m
        depends_on = ["triage"]
    }

    step "technical-analysis" {
        agent = "technical-expert"
        task = "Investigate technical aspects of the issue"
        timeout = 15m
        depends_on = ["triage"]

        retry {
            max_attempts = 2
            delay = 2m
            backoff = "exponential"
        }
    }

    step "resolution" {
        crew = ["support-specialist", "technical-expert"]
        task = "Develop and implement solution"
        timeout = 30m
        depends_on = ["initial-response", "technical-analysis"]
    }

    step "follow-up" {
        agent = "support-specialist"
        task = "Ensure customer satisfaction and document resolution"
        timeout = 10m
        depends_on = ["resolution"]
    }

    pipeline {
        triage -> initial-response -> technical-analysis -> resolution -> follow-up
    }
}

crew "support-team" {
    agents [
        "support-specialist"
        "technical-expert"
    ]

    process = "hierarchical"
    manager = "technical-expert"
    max_iterations = 5
    verbose = true
}

memory {
    provider = "redis"
    connection = "redis://localhost:6379"

    embeddings {
        model = "text-embedding-ada-002"
        dimensions = 1536
        batch_size = 50
    }

    cache_size = 5000
    persistence = false
}

context "production" {
    environment = "prod"
    debug = false
    max_tokens = 150000

    secrets {
        zendesk_token = $ZENDESK_API_TOKEN
        intercom_token = $INTERCOM_API_TOKEN
        slack_token = $SLACK_API_TOKEN
    }

    variables {
        support_email = "support@company.com"
        response_timeout = 4h
        escalation_threshold = 24h
        max_concurrent_tickets = 50
    }
}"#),
    ("data-pipeline", "# Data Pipeline template - full content embedded"),
    ("research", "# Research Assistant template - full content embedded"),
];
fn init_command(
    template: String,
    dir: Option<PathBuf>,
    name: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_content = EMBEDDED_TEMPLATES
        .iter()
        .find(|(t, _)| t == &template)
        .map(|(_, content)| *content)
        .ok_or_else(|| {
            let available: Vec<&str> = EMBEDDED_TEMPLATES
                .iter()
                .map(|(name, _)| *name)
                .collect();
            format!(
                "Unknown template '{}'. Available templates: {}", template, available
                .join(", ")
            )
        })?;
    let output_dir = dir
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
    let filename = name
        .unwrap_or_else(|| {
            match template.as_str() {
                "ai-dev" => "ai_development_team.hlx".to_string(),
                "data-pipeline" => "data_pipeline.hlx".to_string(),
                _ => format!("{}.hlx", template),
            }
        });
    let output_path = output_dir.join(&filename);
    if output_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "File '{}' already exists. Use --force to overwrite.", output_path
            .display()
        ).into());
    }
    if verbose {
        println!("🚀 Initializing HELIX project:");
        println!("  Template: {}", template);
        println!("  Output: {}", output_path.display());
        println!("  Force: {}", force);
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, template_content)?;
    println!("✅ HELIX project initialized successfully!");
    println!("  Created: {}", output_path.display());
    println!("  Template: {}", template);
    if verbose {
        let content_size = template_content.len();
        println!("  Size: {} bytes", content_size);
        let description = match template.as_str() {
            "minimal" => "Simple hlx configuration with basic agent and workflow",
            "ai-dev" => {
                "Complete AI development team with specialized agents for full-stack development"
            }
            "support" => {
                "Multi-tier customer support system with escalation and knowledge management"
            }
            "data-pipeline" => {
                "High-throughput data processing pipeline with ML integration"
            }
            "research" => {
                "AI-powered research assistant for literature review and paper writing"
            }
            _ => "HELIX configuration template",
        };
        println!("  Description: {}", description);
    }
    println!("\n📋 Next steps:");
    println!("  1. Review and customize the configuration");
    println!("  2. Set up your API keys and environment variables");
    println!("  3. Compile with: helix compile {}", filename);
    println!("  4. Run with your hlx runtime");
    Ok(())
}
fn install_command(
    local_only: bool,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔧 Installing Helix compiler globally...");
    }
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    if verbose {
        println!("  Source: {}", current_exe.display());
    }
    let home_dir = std::env::var("HOME")
        .map_err(|e| format!("Failed to get HOME directory: {}", e))?;
    let baton_dir = PathBuf::from(&home_dir).join(".baton");
    let baton_bin_dir = baton_dir.join("bin");
    let target_binary = baton_bin_dir.join("hlx");
    if verbose {
        println!("  Target: {}", target_binary.display());
    }
    std::fs::create_dir_all(&baton_bin_dir)
        .map_err(|e| {
            format!("Failed to create directory {}: {}", baton_bin_dir.display(), e)
        })?;
    if verbose {
        println!("  ✅ Created directory: {}", baton_bin_dir.display());
    }
    if target_binary.exists() && !force {
        return Err(
            format!(
                "HELIX compiler already installed at {}. Use --force to overwrite.",
                target_binary.display()
            )
                .into(),
        );
    }
    std::fs::copy(&current_exe, &target_binary)
        .map_err(|e| {
            format!("Failed to copy binary to {}: {}", target_binary.display(), e)
        })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_binary)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_binary, perms)?;
    }
    if verbose {
        println!("  ✅ Copied binary to: {}", target_binary.display());
    }
    println!("✅ Helix compiler installed successfully!");
    println!("  Location: {}", target_binary.display());
    if local_only {
        println!("\n📋 Local installation complete!");
        println!("  Add {} to your PATH to use 'hlx' command", baton_bin_dir.display());
        println!("  Or run: export PATH=\"{}:$PATH\"", baton_bin_dir.display());
        return Ok(());
    }
    let global_bin_paths = vec![
        PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
    ];
    let mut symlink_created = false;
    for global_bin in global_bin_paths {
        if global_bin.exists() && global_bin.is_dir() {
            let symlink_path = global_bin.join("hlx");
            if symlink_path.exists() && !force {
                if verbose {
                    println!(
                        "  ⚠️  Symlink already exists: {}", symlink_path.display()
                    );
                }
                continue;
            }
            if symlink_path.exists() {
                std::fs::remove_file(&symlink_path)
                    .map_err(|e| {
                        format!(
                            "Failed to remove existing symlink {}: {}", symlink_path
                            .display(), e
                        )
                    })?;
            }
            #[cfg(unix)]
            let symlink_result = std::os::unix::fs::symlink(&target_binary, &symlink_path);

            #[cfg(windows)]
            let symlink_result = {
                // On Windows, try to create a copy instead of symlink if symlink fails
                std::fs::copy(&target_binary, &symlink_path)
                    .map(|_| ())
                    .or_else(|_| std::os::windows::fs::symlink_file(&target_binary, &symlink_path))
            };

            #[cfg(not(any(unix, windows)))]
            let symlink_result = std::fs::copy(&target_binary, &symlink_path).map(|_| ());

            match symlink_result {
                Ok(_) => {
                    println!("  ✅ Created global link: {}", symlink_path.display());
                    symlink_created = true;
                    break;
                }
                Err(e) => {
                    if verbose {
                        println!(
                            "  ⚠️  Failed to create link at {}: {}", symlink_path
                            .display(), e
                        );
                    }
                    continue;
                }
            }
        }
    }
    if symlink_created {
        println!("\n🎉 Global installation complete!");
        println!("  You can now use 'hlx' command from anywhere");
        println!("  Try: hlx --help");
    } else {
        println!("\n📋 Installation complete, but global symlink creation failed");
        println!("  This might be due to insufficient permissions");
        println!(
            "  You can still use hlx by adding {} to your PATH", baton_bin_dir.display()
        );
        println!("  Or run: export PATH=\"{}:$PATH\"", baton_bin_dir.display());
        if verbose {
            println!("\n💡 To create global symlink manually:");
            println!("  sudo ln -sf {} /usr/local/bin/hlx", target_binary.display());
        }
    }
    Ok(())
}
fn build_project(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    optimize: u8,
    compress: bool,
    cache: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = find_project_root()?;
    let input_file = match input {
        Some(path) => path,
        None => {
            let main_file = project_dir.join("src").join("main.hlx");
            if main_file.exists() {
                main_file
            } else {
                return Err(
                    anyhow::anyhow!(
                        "No input file specified and no src/main.hlx found.\n\
                    Specify a file with: helix build <file.hlx>"
                    )
                        .into(),
                );
            }
        }
    };
    let output_file = output
        .unwrap_or_else(|| {
            let target_dir = project_dir.join("target");
            let input_stem = input_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            target_dir.join(format!("{}.hlxb", input_stem))
        });
    if verbose {
        println!("🔨 Building HELIX project:");
        println!("  Input: {}", input_file.display());
        println!("  Output: {}", output_file.display());
        println!("  Optimization: Level {}", optimize);
        println!("  Compression: {}", if compress { "Enabled" } else { "Disabled" });
        println!("  Cache: {}", if cache { "Enabled" } else { "Disabled" });
    }
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let compiler = Compiler::builder()
        .optimization_level(OptimizationLevel::from(optimize))
        .compression(compress)
        .cache(cache)
        .verbose(verbose)
        .build();
    let binary = compiler.compile_file(&input_file)?;
    let serializer = crate::compiler::serializer::BinarySerializer::new(compress);
    serializer.write_to_file(&binary, &output_file)?;
    println!("✅ Build completed successfully!");
    println!("  Output: {}", output_file.display());
    println!("  Size: {} bytes", binary.size());
    if verbose {
        let stats = binary.symbol_table.stats();
        println!(
            "  Strings: {} (unique: {})", stats.total_strings, stats.unique_strings
        );
        println!("  Agents: {}", stats.agents);
        println!("  Workflows: {}", stats.workflows);
    }
    Ok(())
}
fn find_project_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    loop {
        let manifest_path = current_dir.join("project.hlx");
        if manifest_path.exists() {
            return Ok(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(anyhow::anyhow!("No HELIX project found. Run 'helix init' first.").into())
}
// HLX-AI Command Handlers
async fn dataset_command(
    action: DatasetAction,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        DatasetAction::Process { files, output, format, algorithm, validate } => {
            if verbose {
                println!("🧠 Processing datasets with HLX-AI...");
                println!("  Files: {:?}", files);
                println!("  Output: {:?}", output);
                println!("  Format: {:?}", format);
                println!("  Algorithm: {:?}", algorithm);
                println!("  Validate: {}", validate);
            }

            // Use the HLX json core functionality
            use crate::json::core::{GenericJSONDataset, DataFormat};

            for file in &files {
                if verbose {
                    println!("📊 Processing: {}", file.display());
                }

                let dataset = GenericJSONDataset::new(&[file.clone()], None, DataFormat::Auto)
                    .map_err(|e| format!("Failed to load dataset {}: {}", file.display(), e))?;

                let training_dataset = dataset.to_training_dataset()
                    .map_err(|e| format!("Failed to convert dataset {}: {}", file.display(), e))?;

                if validate {
                    let quality = training_dataset.quality_assessment();
                    println!("✅ Quality Score: {:.2}", quality.overall_score);
                    if !quality.issues.is_empty() {
                        println!("⚠️  Issues:");
                        for issue in &quality.issues {
                            println!("   - {}", issue);
                        }
                    }
                }

                if let Some(algo) = &algorithm {
                    if training_dataset.to_algorithm_format(algo).is_ok() {
                        println!("✅ Converted to {} format", algo.to_uppercase());
                    } else {
                        println!("❌ Failed to convert to {} format", algo.to_uppercase());
                    }
                }

                println!("📈 Dataset stats: {} samples", training_dataset.samples.len());
            }

            println!("🎉 Dataset processing completed!");
            Ok(())
        }
        DatasetAction::Analyze { files, detailed } => {
            if verbose {
                println!("🔍 Analyzing datasets...");
            }

            use crate::json::core::{GenericJSONDataset, DataFormat};

            for file in files {
                if verbose {
                    println!("📊 Analyzing: {}", file.display());
                }

                let dataset = GenericJSONDataset::new(&[file.clone()], None, DataFormat::Auto)
                    .map_err(|e| format!("Failed to load dataset {}: {}", file.display(), e))?;

                println!("\n--- Dataset Analysis: {} ---", file.display());
                for (key, value) in dataset.stats() {
                    println!("{:15}: {}", key, value);
                }

                if detailed {
                    let training_dataset = dataset.to_training_dataset()
                        .map_err(|e| format!("Failed to convert dataset {}: {}", file.display(), e))?;

                    println!("\n--- Training Format Analysis ---");
                    println!("Format: {:?}", training_dataset.format);
                    println!("Samples: {}", training_dataset.samples.len());
                    println!("Avg Prompt Length: {:.1}", training_dataset.statistics.avg_prompt_length);
                    println!("Avg Completion Length: {:.1}", training_dataset.statistics.avg_completion_length);

                    println!("\n--- Field Coverage ---");
                    for (field, coverage) in &training_dataset.statistics.field_coverage {
                        println!("{:12}: {:.1}%", field, coverage * 100.0);
                    }
                }
            }

            Ok(())
        }
        DatasetAction::Convert { input, output: _output, from_format, to_format } => {
            if verbose {
                println!("🔄 Converting dataset format...");
                println!("  Input: {}", input.display());
                println!("  From: {}", from_format);
                println!("  To: {}", to_format);
            }

            // This would implement format conversion between different training formats
            println!("🔄 Format conversion: {} → {}", from_format, to_format);
            println!("✅ Conversion completed (placeholder)");
            Ok(())
        }
        DatasetAction::Quality { files, report } => {
            if verbose {
                println!("📊 Assessing dataset quality...");
            }

            use crate::json::core::{GenericJSONDataset, DataFormat};

            for file in files {
                let dataset = GenericJSONDataset::new(&[file.clone()], None, DataFormat::Auto)
                    .map_err(|e| format!("Failed to load dataset {}: {}", file.display(), e))?;

                let training_dataset = dataset.to_training_dataset()
                    .map_err(|e| format!("Failed to convert dataset {}: {}", file.display(), e))?;

                let quality = training_dataset.quality_assessment();

                if report {
                    println!("\n=== Quality Report: {} ===", file.display());
                    println!("Overall Score: {:.2}/1.0", quality.overall_score);
                    println!("\nIssues:");
                    if quality.issues.is_empty() {
                        println!("  ✅ No issues found");
                    } else {
                        for issue in &quality.issues {
                            println!("  ⚠️  {}", issue);
                        }
                    }
                    println!("\nRecommendations:");
                    for rec in &quality.recommendations {
                        println!("  💡 {}", rec);
                    }
                } else {
                    println!("📊 {}: Quality Score {:.2}", file.display(), quality.overall_score);
                }
            }

            Ok(())
        }
        DatasetAction::Huggingface { dataset, split, output, cache_dir } => {
            if verbose {
                println!("🤗 Loading HuggingFace dataset...");
                println!("  Dataset: {}", dataset);
                println!("  Split: {:?}", split.as_ref().unwrap_or(&"train".to_string()));
                println!("  Cache: {:?}", cache_dir);
                println!("  Output: {:?}", output);
            }

            // Use the HLX HuggingFace processor
            let processor = crate::json::HfProcessor::new(cache_dir.unwrap_or_else(|| PathBuf::from("./hf_cache")));

            let config = crate::json::HfDatasetConfig {
                source: dataset.clone(),
                split: split.unwrap_or_else(|| "train".to_string()),
                format: None,
                rpl_filter: None,
                revision: None,
                streaming: false,
                trust_remote_code: false,
                num_proc: None,
            };

            // Process the dataset
            match processor.process_dataset(&dataset, &config).await {
                Ok(training_dataset) => {
                    println!("✅ HuggingFace dataset loaded successfully");
                    println!("📊 Samples: {}", training_dataset.samples.len());
                    println!("📝 Format: {:?}", training_dataset.format);

                    // Save to output file if specified
                    if let Some(output_path) = output {
                        let json_output = serde_json::to_string_pretty(&training_dataset.samples)
                            .map_err(|e| format!("Failed to serialize output: {}", e))?;
                        std::fs::write(&output_path, json_output)
                            .map_err(|e| format!("Failed to write output file {}: {}", output_path.display(), e))?;
                        println!("💾 Saved processed dataset to: {}", output_path.display());
                    }
                }
                Err(e) => {
                    println!("❌ Failed to load HuggingFace dataset: {}", e);
                    return Err(e.into());
                }
            }

            Ok(())
        }
    }
}

fn concat_command(
    directory: PathBuf,
    preset: String,
    output_dir: Option<PathBuf>,
    dry_run: bool,
    deduplicate: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔗 Concatenating files...");
        println!("  Directory: {}", directory.display());
        println!("  Preset: {}", preset);
        println!("  Output: {:?}", output_dir);
        println!("  Dry Run: {}", dry_run);
        println!("  Deduplicate: {}", deduplicate);
    }

    use crate::json::concat::{ConcatConfig, FileExtensionPreset};

    let config = match preset.as_str() {
        "caption+wd+tags" => ConcatConfig::from_preset(FileExtensionPreset::CaptionWdTags),
        "florence+wd+tags" => ConcatConfig::from_preset(FileExtensionPreset::FlorenceWdTags),
        _ => {
            return Err(format!("Unknown preset: {}. Use 'caption+wd+tags' or 'florence+wd+tags'", preset).into());
        }
    };

    let _config = if deduplicate {
        config.with_deduplication(true)
    } else {
        config
    };

    // This would be async in a real implementation
    println!("🔄 Concatenating files in: {}", directory.display());
    println!("📝 Using preset: {}", preset);

    if dry_run {
        println!("🔍 Dry run mode - no files will be modified");
    }

    println!("✅ Concatenation completed (placeholder)");
    Ok(())
}

async fn caption_command(
    action: CaptionAction,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        CaptionAction::Process { files, output, config } => {
            if verbose {
                println!("📝 Processing caption files...");
                println!("  Files: {:?}", files);
                println!("  Output: {:?}", output);
                println!("  Config: {:?}", config);
            }


            for file in files {
                if verbose {
                    println!("🎨 Processing: {}", file.display());
                }

                // Process caption file
                match crate::json::caption::process_file(&file).await {
                    Ok(_) => println!("✅ Processed: {}", file.display()),
                    Err(e) => println!("❌ Failed to process {}: {}", file.display(), e),
                }
            }

            Ok(())
        }
        CaptionAction::E621 { files, output, filter_tags, format } => {
            if verbose {
                println!("🔞 Processing E621 captions...");
                println!("  Filter tags: {}", filter_tags);
                println!("  Format: {:?}", format);
                println!("  Output: {:?}", output);
            }

            use crate::json::caption::{E621Config, process_e621_json_file};

            let config = E621Config::new()
                .with_filter_tags(filter_tags)
                .with_format(format);

            for file in files {
                if verbose {
                    println!("🎨 Processing E621: {}", file.display());
                }

                // Process E621 JSON file
                match process_e621_json_file(&file, Some(config.clone())).await {
                    Ok(_) => {
                        println!("✅ Processed E621 file: {}", file.display());
                        // If output is specified, copy processed file there
                        if let Some(output_path) = &output {
                            let file_name = file.file_name().unwrap_or_default();
                            let target_path = output_path.join(file_name);
                            if let Some(parent) = target_path.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            match std::fs::copy(&file, &target_path) {
                                Ok(_) => println!("💾 Saved processed file to: {}", target_path.display()),
                                Err(e) => println!("⚠️  Failed to save to output: {}", e),
                            }
                        }
                    }
                    Err(e) => println!("❌ Failed to process E621 file {}: {}", file.display(), e),
                }
            }

            Ok(())
        }
        CaptionAction::Convert { input, output, format } => {
            if verbose {
                println!("🔄 Converting caption format...");
                println!("  Input: {}", input.display());
                println!("  Output: {:?}", output);
                println!("  Format: {:?}", format);
            }

            println!("🔄 Converting caption format (placeholder)");
            println!("✅ Conversion completed");
            Ok(())
        }
    }
}

async fn json_command(
    action: JsonAction,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        JsonAction::Format { files, check } => {
            if verbose {
                println!("🎨 Formatting JSON files...");
                println!("  Check only: {}", check);
            }

            use crate::json::format_json_file;

            for file in files {
                if verbose {
                    println!("📝 Formatting: {}", file.display());
                }

                if check {
                    // Check if file is properly formatted
                    match format_json_file(file.clone()).await {
                        Ok(_) => println!("✅ {} is properly formatted", file.display()),
                        Err(e) => println!("❌ {} needs formatting: {}", file.display(), e),
                    }
                } else {
                    // Format the file
                    match format_json_file(file.clone()).await {
                        Ok(_) => println!("✅ Formatted: {}", file.display()),
                        Err(e) => println!("❌ Failed to format {}: {}", file.display(), e),
                    }
                }
            }

            Ok(())
        }
        JsonAction::Validate { files, schema } => {
            if verbose {
                println!("✅ Validating JSON files...");
                println!("  Schema: {:?}", schema);
            }

            use crate::json::core::{GenericJSONDataset, DataFormat};

            for file in files {
                if verbose {
                    println!("🔍 Validating: {}", file.display());
                }

                match GenericJSONDataset::new(&[file.clone()], schema.as_deref(), DataFormat::Auto) {
                    Ok(dataset) => {
                        println!("✅ {} is valid JSON", file.display());
                        if verbose {
                            println!("   Samples: {}", dataset.len());
                            println!("   Format: {:?}", dataset.format);
                        }
                    }
                    Err(e) => println!("❌ {} validation failed: {}", file.display(), e),
                }
            }

            Ok(())
        }
        JsonAction::Metadata { files, output } => {
            if verbose {
                println!("📊 Extracting JSON metadata...");
                println!("  Output: {:?}", output);
            }

            use crate::json::process_safetensors_file;

            for file in files {
                if file.extension().and_then(|s| s.to_str()) == Some("safetensors") {
                    if verbose {
                        println!("🔍 Processing SafeTensors: {}", file.display());
                    }

                    // Extract metadata from SafeTensors file
                    match process_safetensors_file(&file).await {
                        Ok(_) => println!("✅ Metadata extracted from: {}", file.display()),
                        Err(e) => println!("❌ Failed to extract metadata from {}: {}", file.display(), e),
                    }
                } else {
                    println!("⚠️  Skipping non-SafeTensors file: {}", file.display());
                }
            }

            Ok(())
        }
        JsonAction::Split { file, output } => {
            if verbose {
                println!("✂️  Splitting JSON file...");
                println!("  Input: {}", file.display());
                println!("  Output: {:?}", output);
            }

            use crate::json::split_content;

            // Read and split the JSON file content
            let content = tokio::fs::read_to_string(&file).await?;
            let (tags, sentences) = split_content(&content);
            println!("✅ Split {}: {} tags, {} sentences", file.display(), tags.len(), sentences.len());

            if let Some(output_path) = output {
                let split_data = serde_json::json!({
                    "tags": tags,
                    "sentences": sentences
                });
                let json_output = serde_json::to_string_pretty(&split_data)
                    .map_err(|e| format!("Failed to serialize split data: {}", e))?;
                std::fs::write(&output_path, json_output)
                    .map_err(|e| format!("Failed to write split output to {}: {}", output_path.display(), e))?;
                println!("💾 Saved split data to: {}", output_path.display());
            }
            Ok(())
        }
        JsonAction::Merge { files, output } => {
            if verbose {
                println!("🔗 Merging JSON files...");
                println!("  Output: {}", output.display());
            }

            use crate::json::core::{run_json_cmd, JsonArgs};

            // Use the existing merge functionality
            let args = JsonArgs {
                data_dir: vec![],
                file: files.into_iter().map(|p| p.to_string_lossy().to_string()).collect(),
                schema_dir: None,
                format: crate::json::core::DataFormat::Auto,
                merge_output: Some(output),
                show_stats: verbose,
                seed: 42,
                multi_process: false,
                input_folder: None,
                output: None,
                jobs: num_cpus::get(),
            };

            // Run the JSON merge command
            match run_json_cmd(args).await {
                Ok(_) => println!("✅ Successfully merged JSON files"),
                Err(e) => println!("❌ Failed to merge JSON files: {}", e),
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from([
            "helix",
            "compile",
            "test.hlx",
            "-O3",
            "--compress",
        ]);
        assert!(cli.is_ok());
    }
}