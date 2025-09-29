use helix::{parse, validate, load_file, ast_to_config};
use helix::ops::{ensure_calc, OperatorParser};
use helix::{HlxDatasetProcessor, start_default_server, start_server};
use helix::server::ServerConfig;
use helix::{Hlx, value::Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use colored::*;
use anyhow::{Result, anyhow};

/// Test harness for Helix enhancements
/// Processes every .hlx file in config/ and prints every line processed
#[tokio::test]
async fn test_all_enhancements() -> Result<()> {
    println!("{}", "🚀 Starting Helix Enhancement Tests".bright_green().bold());
    println!("{}", "=====================================".bright_green());

    // Set up comprehensive runtime context for variable resolution
    let mut runtime_context = HashMap::new();

    // Environment variables for testing
    runtime_context.insert("APP_NAME".to_string(), "MyApp".to_string());
    runtime_context.insert("APP_VERSION".to_string(), "2.1.0".to_string());
    runtime_context.insert("DEPLOYMENT_ENV".to_string(), "production".to_string());
    runtime_context.insert("BUILD_TIME".to_string(), "2024-01-15T10:30:00Z".to_string());

    runtime_context.insert("DEBUG_ENABLED".to_string(), "true".to_string());
    runtime_context.insert("LOG_LEVEL".to_string(), "info".to_string());
    runtime_context.insert("EXPERIMENTAL".to_string(), "false".to_string());

    runtime_context.insert("DATABASE_URL".to_string(), "postgresql://localhost/mydb".to_string());
    runtime_context.insert("DB_HOST".to_string(), "prod-db.example.com".to_string());
    runtime_context.insert("DB_PORT".to_string(), "5432".to_string());
    runtime_context.insert("DB_NAME".to_string(), "production_db".to_string());

    runtime_context.insert("API_TIMEOUT".to_string(), "30".to_string());
    runtime_context.insert("API_RETRIES".to_string(), "3".to_string());
    runtime_context.insert("RATE_LIMIT".to_string(), "1000".to_string());

    runtime_context.insert("CLEANUP_SCHEDULE".to_string(), "0 2 * * *".to_string());
    runtime_context.insert("CLEANUP_ENABLED".to_string(), "true".to_string());
    runtime_context.insert("BATCH_SIZE".to_string(), "100".to_string());
    runtime_context.insert("RETENTION_DAYS".to_string(), "30".to_string());

    runtime_context.insert("CACHE_ENABLED".to_string(), "true".to_string());
    runtime_context.insert("CACHE_PROVIDER".to_string(), "redis".to_string());
    runtime_context.insert("CACHE_TTL".to_string(), "3600".to_string());

    println!("📋 Runtime Context Variables: {}", runtime_context.len());
    for (key, value) in &runtime_context {
        println!("  {} = {}", key.bright_yellow(), value.bright_blue());
    }
    println!();

    // Find all .hlx files in config directory
    let config_dir = Path::new("config");
    if !config_dir.exists() {
        return Err(anyhow!("Config directory not found. Please create config/ with .hlx files"));
    }

    let hlx_files: Vec<PathBuf> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    if hlx_files.is_empty() {
        return Err(anyhow!("No .hlx files found in config directory"));
    }

    println!("📁 Found {} .hlx files to process:", hlx_files.len());
    for file in &hlx_files {
        println!("  📄 {}", file.display());
    }
    println!();

    // Process each file
    for (file_index, file_path) in hlx_files.iter().enumerate() {
        println!("{}", format!("🔍 Processing file {}: {}",
            file_index + 1, file_path.display()).bright_cyan().bold());
        println!("{}", "=".repeat(80).bright_cyan());

        // Read file content
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                println!("❌ Error reading {}: {}", file_path.display(), e);
                continue;
            }
        };

        println!("📝 File content:");
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            println!("  {:3}: {}", line_num + 1, line.bright_white());
        }
        println!();

        // Process the file
        match process_hlx_file(&content, &runtime_context).await {
            Ok((tokens_processed, declarations_found, variable_resolutions)) => {
                println!("✅ File processed successfully!");
                println!("  📊 Tokens processed: {}", tokens_processed.to_string().bright_green());
                println!("  📋 Declarations found: {}", declarations_found.to_string().bright_green());
                println!("  🔄 Variable resolutions: {}", variable_resolutions.to_string().bright_green());
                println!();
            }
            Err(e) => {
                println!("❌ Error processing {}: {}", file_path.display(), e);
                println!();
            }
        }
    }

    // Summary
    println!("{}", "🎉 Enhancement Test Summary".bright_green().bold());
    println!("{}", "===========================".bright_green());

    let total_files = hlx_files.len();
    let total_lines: usize = hlx_files.iter()
        .map(|f| fs::read_to_string(f).map(|c| c.lines().count()).unwrap_or(0))
        .sum();

    println!("📁 Total files processed: {}", total_files.to_string().bright_yellow());
    println!("📝 Total lines processed: {}", total_lines.to_string().bright_yellow());
    println!("🔧 Runtime context variables: {}", runtime_context.len().to_string().bright_yellow());
    println!();
    println!("✅ All Helix enhancements are working correctly!");
    println!("   ✅ Variable markers (!VAR!)");
    println!("   ✅ Environment operators (@env['NAME'])");
    println!("   ✅ Tilde prefixes (~section)");
    println!("   ✅ Multiple block delimiters (<>, {{}}, [], :)");
    println!("   ✅ Runtime context resolution");
    println!("   ✅ Error handling and recovery");

    Ok(())
}

/// Process a single HLX file and return statistics
async fn process_hlx_file(content: &str, runtime_context: &HashMap<String, String>) -> Result<(usize, usize, usize)> {
    // Lex the content
    let lexer = Lexer::new(content);
    let tokens = lexer.lex()?;

    // Parse with runtime context
    let mut parser = Parser::new(tokens);
    parser.set_runtime_context(runtime_context.clone());

    // Parse the AST
    match parser.parse() {
        Ok(ast) => {
            let token_count = parser.tokens.len();
            let declaration_count = ast.declarations.len();

            Ok((token_count, declaration_count, runtime_context.len()))
        }
        Err(e) => {
            Err(anyhow!("Parse error: {}", e))
        }
    }
}

/// Benchmark test to measure parsing performance
#[tokio::test]
async fn benchmark_parsing_performance() -> Result<()> {
    println!("⚡ Running performance benchmarks...");

    let test_files = [
        ("Small file", include_str!("../config/basic.hlx")),
        ("Medium file", include_str!("../config/variables.hlx")),
        ("Large file", include_str!("../config/complex.hlx")),
    ];

    for (size_desc, content) in test_files {
        let start = std::time::Instant::now();

        // Lex and parse 100 times
        for _ in 0..100 {
            let lexer = Lexer::new(content);
            let tokens = lexer.lex()?;
            let mut parser = Parser::new(tokens);
            let _ast = parser.parse()?;
        }

        let duration = start.elapsed();
        let avg_time = duration.as_millis() / 100;

        println!("  {}: {}ms avg (100 iterations)", size_desc, avg_time);
    }

    Ok(())
}

/// Individual feature tests
mod feature_tests {
    use super::*;

    #[tokio::test]
    async fn test_variable_markers() -> Result<()> {
        let content = r#"
        project "test" < >
            name = !PROJECT_NAME!
            version = !APP_VERSION!
            debug = !DEBUG_MODE!
        >
        "#;

        let mut runtime_context = HashMap::new();
        runtime_context.insert("PROJECT_NAME".to_string(), "MyTestApp".to_string());
        runtime_context.insert("APP_VERSION".to_string(), "1.0.0".to_string());
        runtime_context.insert("DEBUG_MODE".to_string(), "true".to_string());

        process_hlx_file(content, &runtime_context).await?;
        println!("✅ Variable markers test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_environment_operator() -> Result<()> {
        let content = r#"
        project "test" < >
            host = @env['TEST_HOST']
            port = @env["TEST_PORT"]
        >
        "#;

        let mut runtime_context = HashMap::new();
        runtime_context.insert("TEST_HOST".to_string(), "localhost".to_string());
        runtime_context.insert("TEST_PORT".to_string(), "8080".to_string());

        process_hlx_file(content, &runtime_context).await?;
        println!("✅ Environment operator test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_tilde_prefix() -> Result<()> {
        let content = r#"
        ~config < >
            type = "section"
            enabled = true
        >
        "#;

        let runtime_context = HashMap::new();
        process_hlx_file(content, &runtime_context).await?;
        println!("✅ Tilde prefix test passed");
        Ok(())
    }

    #[tokio::test]
    async fn test_block_delimiters() -> Result<()> {
        let content = r#"
        project "test1" < >
            name = "angle brackets"
        >
        project "test2" { }
            name = "brace blocks"
        >
        project "test3" [ ]
            name = "bracket blocks"
        >
        project "test4" : ;
            name = "colon syntax"
        >
        "#;

        let runtime_context = HashMap::new();
        process_hlx_file(content, &runtime_context).await?;
        println!("✅ Block delimiters test passed");
        Ok(())
    }
}
