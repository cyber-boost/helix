use anyhow::Result;
use helix::hlx::{HlxDatasetProcessor, start_default_server};

fn main() -> Result<()> {
    println!("🚀 My HLX-Powered Application");

    // Load and process HLX configurations
    let mut processor = HlxDatasetProcessor::new();
    let config = processor.load_config_file("config.hlx")?;

    println!("✅ Loaded {} configuration sections", config.sections.len());

    // Access dynamic sections
    if let Some(app_config) = config.sections.get("app") {
        let version = app_config.get("version")
            .and_then(|v| v.as_string())
            .unwrap_or("unknown");
        println!("📦 App version: {}", version);
    }

    // Start HLX configuration server
    let server = start_default_server();
    println!("🌐 HLX server ready on port 4592");

    // In a real application, you might:
    // - server.start()?; // This blocks
    // - Or integrate with a web framework
    // - Or use the processor for batch processing

    println!("🎉 Application ready!");

    Ok(())
}
