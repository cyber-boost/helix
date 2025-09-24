//! Example of using HLX as a dependency in another Rust project
//!
//! This demonstrates how to use HLX programmatically without the CLI

use helix_core::hlx::{HlxDatasetProcessor, start_default_server, start_server};
use helix_core::server::ServerConfig;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 HLX Programmatic Usage Example");
    println!("==================================");

    // 1. Load configuration files programmatically
    println!("\n📁 Loading HLX configuration files...");
    let mut processor = HlxDatasetProcessor::new();

    // Load a configuration file
    match processor.load_config_file("forge.hlx") {
        Ok(config) => {
            println!("✅ Successfully loaded configuration!");
            println!("  📊 Sections found: {}", config.sections.len());
            for (section_name, section_data) in &config.sections {
                println!("    - {}: {} properties", section_name, section_data.len());
            }

            // Access specific configuration values
            if let Some(training_config) = config.sections.get("training") {
                let trainer = training_config.get("trainer")
                    .and_then(|v| v.as_string())
                    .unwrap_or("unknown");
                println!("  🎯 Training trainer: {}", trainer);
            }

            if let Some(model_config) = config.sections.get("model") {
                let model_type = model_config.get("type")
                    .and_then(|v| v.as_string())
                    .unwrap_or("unknown");
                println!("  🤖 Model type: {}", model_type);
            }
        }
        Err(e) => {
            println!("❌ Failed to load configuration: {}", e);
        }
    }

    // 2. Start HLX server programmatically
    println!("\n🌐 Starting HLX Configuration Server...");

    // Option A: Start with default settings
    let server = start_default_server();
    println!("✅ Server created with default settings:");
    println!("  📍 Port: 4592");
    println!("  🌐 Domain: localhost");
    println!("  📁 Root: current directory");

    // Option B: Start with custom configuration
    let custom_config = ServerConfig {
        port: 8080,
        domain: "127.0.0.1".to_string(),
        root_directory: std::env::current_dir()?,
        auto_convert: true,
        cache_timeout: 300,
        max_file_size: 50 * 1024 * 1024, // 50MB
        allowed_extensions: vec!["hlx".to_string(), "hlxb".to_string(), "json".to_string()],
        cors_enabled: true,
        verbose: true,
    };

    let custom_server = start_server(custom_config);
    println!("✅ Custom server configured:");
    println!("  📍 Port: 8080");
    println!("  🌐 Domain: 127.0.0.1");
    println!("  📁 Max file size: 50MB");
    println!("  🔄 CORS enabled: true");

    // 3. Use server in background (in real app, you'd want to handle this properly)
    println!("\n⚡ Server would start here in production...");
    println!("💡 To actually start the server:");
    println!("   server.start()?; // This blocks and runs the server");
    println!("   custom_server.start()?; // This also blocks");

    // Simulate server running (in real usage, this would be handled by a server framework)
    let server_handle = thread::spawn(move || {
        println!("🔧 Server simulation running...");
        thread::sleep(Duration::from_millis(100));
        println!("🔧 Server simulation complete!");
    });

    server_handle.join().expect("Server thread panicked");

    // 4. Process datasets programmatically
    println!("\n📊 Processing datasets...");
    let dataset_config = processor.process_dataset_config("forge.hlx", "example_dataset")?;
    println!("✅ Dataset config created:");
    println!("  📝 Name: {}", dataset_config.name);
    println!("  📋 Format: {}", dataset_config.format);
    println!("  📦 Batch size: {}", dataset_config.processing_options.batch_size);

    // 5. Validate datasets
    println!("\n🔍 Validating dataset quality...");
    let sample_data = serde_json::json!({
        "prompt": "What is the capital of France?",
        "completion": "Paris",
        "difficulty": "easy"
    });

    let validation = processor.validate_dataset(&dataset_config, &sample_data)?;
    println!("✅ Dataset validation result:");
    println!("  ✅ Valid: {}", validation.is_valid);
    println!("  📊 Quality score: {:.2}", validation.score);
    println!("  📝 Suggestions: {:?}", validation.suggestions);

    println!("\n🎉 HLX Programmatic Usage Complete!");
    println!("📚 Key functions available:");
    println!("  • processor.load_config_file(\"file.hlx\")");
    println!("  • start_server(ServerConfig)");
    println!("  • start_default_server()");
    println!("  • processor.process_dataset_config()");
    println!("  • processor.validate_dataset()");

    Ok(())
}
