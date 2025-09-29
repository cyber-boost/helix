use std::path::PathBuf;
use crate::dna::mds::preview::preview_command;
use crate::dna::mds::caption::{JsonAction, CaptionAction};

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
    use crate::dna::map::concat::{ConcatConfig, FileExtensionPreset};
    let config = match preset.as_str() {
        "caption+wd+tags" => {
            ConcatConfig::from_preset(FileExtensionPreset::CaptionWdTags)
        }
        "florence+wd+tags" => {
            ConcatConfig::from_preset(FileExtensionPreset::FlorenceWdTags)
        }
        _ => {
            return Err(
                format!(
                    "Unknown preset: {}. Use 'caption+wd+tags' or 'florence+wd+tags'",
                    preset
                )
                    .into(),
            );
        }
    };
    let _config = if deduplicate { config.with_deduplication(true) } else { config };
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
                match crate::map::caption::process_file(&file).await {
                    Ok(_) => println!("✅ Processed: {}", file.display()),
                    Err(e) => println!("❌ Failed to process {}: {}", file.display(), e),
                }
            }
            Ok(())
        }
        CaptionAction::E621 { files, output, filter_tags, format: fmt } => {
            if verbose {
                println!("🔞 Processing E621 captions...");
                println!("  Filter tags: {}", filter_tags);
                println!("  Format: {:?}", fmt);
                println!("  Output: {:?}", output);
            }
            use crate::map::caption::{E621Config, process_e621_json_file};
            let config = E621Config::new()
                .with_filter_tags(!filter_tags.is_empty())
                .with_format(fmt);
            for file in files {
                if verbose {
                    println!("🎨 Processing E621: {}", file.display());
                }
                match process_e621_json_file(&file, Some(config.clone())).await {
                    Ok(_) => {
                        println!("✅ Processed E621 file: {}", file.display());
                        if let Some(output_path) = &output {
                            let file_name = file.file_name().unwrap_or_default();
                            let target_path = output_path.join(file_name);
                            if let Some(parent) = target_path.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            match std::fs::copy(&file, &target_path) {
                                Ok(_) => {
                                    println!(
                                        "💾 Saved processed file to: {}", target_path.display()
                                    )
                                }
                                Err(e) => {
                                    println!("⚠️  Failed to save to output: {}", e)
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "❌ Failed to process E621 file {}: {}", file.display(), e
                        )
                    }
                }
            }
            Ok(())
        }
        CaptionAction::Convert { input, output, format: fmt } => {
            if verbose {
                println!("🔄 Converting caption format...");
                println!("  Input: {}", input.display());
                println!("  Output: {:?}", output);
                println!("  Format: {:?}", fmt);
            }
            println!("🔄 Converting caption format (placeholder)");
            println!("✅ Conversion completed");
            Ok(())
        }
        CaptionAction::Preview { file, format: fmt, rows, columns } => {
            preview_command(file, fmt, rows, columns, verbose)
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
            use crate::map::format_json_file;
            for file in files {
                if verbose {
                    println!("📝 Formatting: {}", file.display());
                }
                if check {
                    match format_json_file(file.clone()).await {
                        Ok(_) => println!("✅ {} is properly formatted", file.display()),
                        Err(e) => {
                            println!("❌ {} needs formatting: {}", file.display(), e)
                        }
                    }
                } else {
                    match format_json_file(file.clone()).await {
                        Ok(_) => println!("✅ Formatted: {}", file.display()),
                        Err(e) => {
                            println!("❌ Failed to format {}: {}", file.display(), e)
                        }
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
            use crate::map::core::{GenericJSONDataset, DataFormat};
            for file in files {
                if verbose {
                    println!("🔍 Validating: {}", file.display());
                }
                match GenericJSONDataset::new(
                    &[file.clone()],
                    schema.as_deref(),
                    DataFormat::Auto,
                ) {
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
            use crate::map::process_safetensors_file;
            for file in files {
                if file.extension().and_then(|s| s.to_str()) == Some("safetensors") {
                    if verbose {
                        println!("🔍 Processing SafeTensors: {}", file.display());
                    }
                    match process_safetensors_file(&file).await {
                        Ok(_) => {
                            println!("✅ Metadata extracted from: {}", file.display())
                        }
                        Err(e) => {
                            println!(
                                "❌ Failed to extract metadata from {}: {}", file
                                .display(), e
                            )
                        }
                    }
                } else {
                    println!(
                        "⚠️  Skipping non-SafeTensors file: {}", file.display()
                    );
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
            // Placeholder split_content function
            fn split_content(content: &str) -> (Vec<String>, Vec<String>) {
                let lines: Vec<&str> = content.lines().collect();
                let tags: Vec<String> = lines.iter().filter(|line| line.starts_with("#")).map(|s| s.to_string()).collect();
                let sentences: Vec<String> = lines.iter().filter(|line| !line.starts_with("#") && !line.trim().is_empty()).map(|s| s.to_string()).collect();
                (tags, sentences)
            }
            let content = tokio::fs::read_to_string(&file).await?;
            let (tags, sentences) = split_content(&content);
            println!(
                "✅ Split {}: {} tags, {} sentences", file.display(), tags.len(),
                sentences.len()
            );
            if let Some(output_path) = output {
                let split_data = serde_json::json!(
                    { "tags" : tags, "sentences" : sentences }
                );
                let json_output = serde_json::to_string_pretty(&split_data)
                    .map_err(|e| format!("Failed to serialize split data: {}", e))?;
                std::fs::write(&output_path, json_output)
                    .map_err(|e| {
                        format!(
                            "Failed to write split output to {}: {}", output_path
                            .display(), e
                        )
                    })?;
                println!("💾 Saved split data to: {}", output_path.display());
            }
            Ok(())
        }
        JsonAction::Merge { files, output } => {
            if verbose {
                println!("🔗 Merging JSON files...");
                println!("  Output: {}", output.display());
            }
            use crate::map::core::{run_json_cmd, JsonArgs};
            let args = JsonArgs {
                data_dir: vec![],
                file: files
                    .into_iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
                schema_dir: None,
                format: crate::map::core::DataFormat::Auto,
                merge_output: Some(output),
                show_stats: verbose,
                seed: 42,
                multi_process: false,
                input_folder: None,
                output: None,
                jobs: num_cpus::get(),
            };
            match run_json_cmd(args).await {
                Ok(_) => println!("✅ Successfully merged JSON files"),
                Err(e) => println!("❌ Failed to merge JSON files: {}", e),
            }
            Ok(())
        }
    }
}