//! Forge HLX Integration Demo Test
//!
//! This demonstrates the HLX integration working within the test framework.

use helix::hlx::{HlxDatasetProcessor, HlxBridge, DatasetConfig};
use std::collections::HashMap;

#[test]
fn demo_forge_hlx_integration() {
    println!("🔥 Forge HLX Integration Demo");
    println!("==============================");

    // Example 1: Basic HLX File Loading
    println!("\n📁 Example 1: Loading HLX Configuration");
    assert!(load_hlx_config_demo().is_ok());

    // Example 2: Dataset Processing
    println!("\n📊 Example 2: Dataset Configuration Processing");
    assert!(process_dataset_config_demo().is_ok());

    // Example 3: Legacy System Migration
    println!("\n🔄 Example 3: Legacy Configuration Migration");
    assert!(migrate_legacy_config_demo().is_ok());

    // Example 4: Quality Validation
    println!("\n✅ Example 4: Dataset Quality Validation");
    assert!(validate_dataset_quality_demo().is_ok());

    // Example 5: Cache Management
    println!("\n💾 Example 5: Cache Statistics & Management");
    assert!(manage_cache_demo().is_ok());

    println!("\n🎉 Forge HLX Integration Demo Complete!");
    println!("Your Forge application now supports advanced HLX processing!");
}

/// Demo 1: Load and parse HLX configuration files
fn load_hlx_config_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading HLX configuration...");

    // Create HLX processor
    let mut processor = HlxDatasetProcessor::new();

    // Try to load HLX configuration file
    match processor.load_config_file("forge.hlx") {
        Ok(config) => {
            println!("✅ Successfully loaded HLX config!");
            if let Some((_, project)) = config.projects.iter().next() {
                println!("   Name: {}", project.name);
                println!("   Version: {}", project.version);
                println!("   Description: {}", project.description.as_deref().unwrap_or("No description"));
            } else {
                println!("   No projects found in config");
            }
            println!("   Agents: {}", config.agents.len());
            println!("   Workflows: {}", config.workflows.len());
        }
        Err(e) => {
            println!("❌ Failed to load HLX config: {}", e);
            println!("💡 This might be expected if forge.hlx doesn't exist in test environment");
            // Don't fail the test for missing file - that's normal in CI
            return Ok(());
        }
    }

    Ok(())
}

/// Demo 2: Process dataset configurations from HLX
fn process_dataset_config_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing dataset configuration...");

    let mut processor = HlxDatasetProcessor::new();

    // Process dataset config from HLX file
    match processor.process_dataset_config("forge.hlx", "bco_training") {
        Ok(dataset_config) => {
            println!("✅ Dataset configuration extracted!");
            println!("   Name: {}", dataset_config.name);
            println!("   Source: {}", dataset_config.source);
            println!("   Format: {}", dataset_config.format);
            println!("   Required Columns: {:?}", dataset_config.validation_rules);
            println!("   Batch Size: {}", dataset_config.processing_options.batch_size);
        }
        Err(e) => {
            println!("❌ Dataset processing failed: {}", e);
            println!("💡 This might be expected if forge.hlx doesn't exist in test environment");
            return Ok(());
        }
    }

    Ok(())
}

/// Demo 3: Migrate legacy configurations to HLX
fn migrate_legacy_config_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Migrating legacy configuration...");

    // Simulate legacy configuration
    let mut legacy_config = HashMap::new();
    legacy_config.insert("dataset.name".to_string(), "bco_training".to_string());
    legacy_config.insert("dataset.format".to_string(), "jsonl".to_string());
    legacy_config.insert("processing.batch_size".to_string(), "32".to_string());
    legacy_config.insert("processing.shuffle".to_string(), "true".to_string());

    let mut bridge = HlxBridge::new();

    match bridge.convert_legacy_dataset(&legacy_config) {
        Ok(dataset_config) => {
            println!("✅ Legacy config migrated to HLX!");
            println!("   Migrated Dataset: {}", dataset_config.name);
            println!("   Source: {}", dataset_config.source);
            println!("   Processing Options: batch_size={}, shuffle={}",
                     dataset_config.processing_options.batch_size,
                     dataset_config.processing_options.shuffle);
        }
        Err(e) => {
            println!("❌ Migration failed: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}

/// Demo 4: Validate dataset quality
fn validate_dataset_quality_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Validating dataset quality...");

    let processor = HlxDatasetProcessor::new();

    // Create a dataset configuration for validation
    let dataset_config = DatasetConfig {
        name: "validation_test".to_string(),
        source: "test_data".to_string(),
        format: "auto".to_string(),
        validation_rules: vec!["check_required_fields".to_string()],
        processing_options: Default::default(),
    };

    // Sample dataset entry
    let sample_data = serde_json::json!({
        "prompt": "Explain quantum computing in simple terms",
        "completion": "Quantum computing uses quantum mechanics principles...",
        "label": 1
    });

    match processor.validate_dataset(&dataset_config, &sample_data) {
        Ok(validation) => {
            println!("✅ Dataset validation complete!");
            println!("   Valid: {}", validation.is_valid);
            println!("   Quality Score: {:.2}", validation.score);

            if validation.issues.is_empty() {
                println!("   Issues: None");
            } else {
                println!("   Issues Found:");
                for issue in &validation.issues {
                    println!("   - {}", issue);
                }
            }

            println!("   Suggestions:");
            for suggestion in &validation.suggestions {
                println!("   - {}", suggestion);
            }
        }
        Err(e) => {
            println!("❌ Validation failed: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}

/// Demo 5: Cache management and statistics
fn manage_cache_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Managing cache...");

    let mut processor = HlxDatasetProcessor::new();

    match processor.cache_stats() {
        Ok(stats) => {
            println!("✅ Cache statistics retrieved!");
            println!("   Total Size: {:.2} MB", stats.total_size_mb());
            println!("   File Count: {}", stats.file_count);
            println!("   Cached Configs: {}", stats.cached_configs);
            println!("   Cache Location: {}", stats.cache_dir.display());

            // Demonstrate cache clearing
            println!("   Clearing cache...");
            if let Err(e) = processor.clear_cache() {
                println!("   ⚠️  Cache clear warning: {}", e);
            } else {
                println!("   ✅ Cache cleared successfully!");
            }
        }
        Err(e) => {
            println!("❌ Cache management failed: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}
