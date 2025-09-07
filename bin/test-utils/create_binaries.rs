use std::fs;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔨 Creating persistent HELIX binary examples...\n");
    fs::create_dir_all("binaries")?;
    let examples = vec![
        ("test_example.hlxbb", "Simple test configuration"), ("examples/minimal.hlxbb",
        "Minimal agent configuration"), ("examples/research_assistant.hlxbb",
        "Research assistant system"), ("examples/ai_development_team.hlxbb",
        "AI development team"),
    ];
    let opt_levels = vec![
        ("zero", helix_core::compiler::OptimizationLevel::Zero), ("one",
        helix_core::compiler::OptimizationLevel::One), ("two",
        helix_core::compiler::OptimizationLevel::Two), ("three",
        helix_core::compiler::OptimizationLevel::Three),
    ];
    for (mso_path, description) in &examples {
        if !Path::new(mso_path).exists() {
            println!("⚠️  Skipping {} (file not found)", mso_path);
            continue;
        }
        println!("📄 Processing: {} ({})", mso_path, description);
        let source = fs::read_to_string(mso_path)?;
        let source_size = source.len();
        for (level_name, level) in &opt_levels {
            let compiler = helix_core::compiler::Compiler::new(*level);
            match compiler.compile_source(&source, None) {
                Ok(binary) => {
                    let filename = Path::new(mso_path)
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    let binary_path = format!(
                        "binaries/{}_opt_{}.hlxb", filename, level_name
                    );
                    let serializer = helix_core::compiler::serializer::BinarySerializer::new(
                        true,
                    );
                    serializer.write_to_file(&binary, Path::new(&binary_path))?;
                    let binary_size = binary.size();
                    let compression_ratio = (1.0
                        - (binary_size as f64 / source_size as f64)) * 100.0;
                    println!(
                        "  ✅ {} → {} bytes ({:.1}% compression)", level_name
                        .to_uppercase(), binary_size, compression_ratio
                    );
                }
                Err(e) => {
                    println!(
                        "  ❌ {} compilation failed: {}", level_name.to_uppercase(), e
                    );
                }
            }
        }
        println!("  📊 Source: {} bytes\n", source_size);
    }
    let comprehensive_example = r#"
# Comprehensive HELIX Example - All Features
project "comprehensive-ai-system" {
    version = "1.0.0"
    author = "HELIX Language Team"
    description = "Demonstrates all HELIX Language features"
}

agent "senior-rust-engineer" {
    model = "claude-3-opus"
    temperature = 0.3
    timeout = 45m
    max_tokens = 8000
    
    capabilities [
        "rust-programming"
        "system-architecture"
        "performance-optimization" 
        "security-analysis"
    ]
    
    backstory {
        role = "Senior Software Engineer"
        experience = "15+ years in systems programming"
        expertise = ["rust", "distributed-systems", "compilers"]
        specialization = "High-performance backend systems"
    }
}

agent "ai-researcher" {
    model = "claude-3-opus"
    temperature = 0.7
    timeout = 30m
    
    capabilities [
        "machine-learning"
        "research-analysis"
        "paper-review"
        "algorithm-design"
    ]
}

memory "codebase-knowledge" {
    type = "vector"
    provider = "chroma"
    embeddings = "openai-ada-002"
    
    collections {
        "rust-code" = {
            path = "./src"
            patterns = ["*.rs"]
            chunk_size = 1000
        }
        "documentation" = {
            path = "./docs"
            patterns = ["*.md", "*.txt"]
        }
    }
}

context "development" {
    secrets {
        "openai-api-key" = $OPENAI_API_KEY
        "github-token" = $GITHUB_TOKEN
        "database-url" = $DEV_DATABASE_URL
    }
    
    memory = "codebase-knowledge"
    logging = true
    debug = true
}

workflow "code-review-and-optimize" {
    trigger = "pull-request"
    timeout = 60m
    
    step "security-review" {
        agent = "senior-rust-engineer"
        task = "Review code for security vulnerabilities and unsafe practices"
        timeout = 20m
    }
    
    step "performance-analysis" {
        agent = "senior-rust-engineer" 
        task = "Analyze performance bottlenecks and suggest optimizations"
        depends_on = ["security-review"]
        timeout = 25m
    }
    
    step "research-validation" {
        agent = "ai-researcher"
        task = "Validate algorithms against current research"
        depends_on = ["security-review"]
        timeout = 15m
    }
    
    retry {
        max_attempts = 3
        delay = 5m
        exponential_backoff = true
    }
}

pipeline "ml-data-processing" {
    input = "raw-data"
    
    stage "preprocessing" {
        agent = "ai-researcher"
        parallel = true
        batch_size = 1000
    }
    
    stage "feature-extraction" {
        agent = "ai-researcher"
        depends_on = ["preprocessing"]
    }
    
    stage "validation" {
        agent = "senior-rust-engineer"
        depends_on = ["feature-extraction"]
    }
    
    output = "processed-data"
}

crew "engineering-research-team" {
    agents [
        "senior-rust-engineer"
        "ai-researcher"
    ]
    
    process = "hierarchical"
    manager = "senior-rust-engineer"
    
    context = "development"
    memory = "codebase-knowledge"
    
    max_iterations = 10
    consensus_threshold = 0.8
}
"#;
    println!("📄 Creating comprehensive example...");
    fs::write("comprehensive_example.hlxbb", comprehensive_example)?;
    let source_size = comprehensive_example.len();
    println!("  📊 Source: {} bytes", source_size);
    for (level_name, level) in &opt_levels {
        let compiler = helix_core::compiler::Compiler::new(*level);
        match compiler.compile_source(comprehensive_example, None) {
            Ok(binary) => {
                let binary_path = format!(
                    "binaries/comprehensive_opt_{}.hlxb", level_name
                );
                let serializer = helix_core::compiler::serializer::BinarySerializer::new(
                    true,
                );
                serializer.write_to_file(&binary, Path::new(&binary_path))?;
                let binary_size = binary.size();
                let compression_ratio = (1.0 - (binary_size as f64 / source_size as f64))
                    * 100.0;
                println!(
                    "  ✅ {} → {} bytes ({:.1}% compression)", level_name
                    .to_uppercase(), binary_size, compression_ratio
                );
            }
            Err(e) => {
                println!(
                    "  ❌ {} compilation failed: {}", level_name.to_uppercase(), e
                );
            }
        }
    }
    println!("\n🎉 Binary examples created successfully!");
    println!("📁 Check the 'binaries/' directory for .hlxb files");
    println!("\n📋 Created binaries:");
    if let Ok(entries) = fs::read_dir("binaries") {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".hlxb") {
                        if let Ok(metadata) = entry.metadata() {
                            println!("  • {} ({} bytes)", name, metadata.len());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}