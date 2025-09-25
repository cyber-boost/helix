use anyhow::Result;
use helix::json::*;
use serde_json::json;

/// Demonstration of HLX-AI capabilities for intelligent dataset processing
fn main() -> Result<()> {
    println!("🚀 HLX-AI Dataset Processing Demonstration");
    println!("==========================================\n");

    // Example 1: Preference Dataset (DPO-style)
    println!("📊 Example 1: Preference Dataset Processing");
    println!("-------------------------------------------");

    let preference_data = vec![
        json!({
            "prompt": "Explain quantum computing in simple terms",
            "chosen": "Quantum computing uses quantum mechanics principles like superposition and entanglement to perform calculations much faster than classical computers for certain problems.",
            "rejected": "Quantum computing is just regular computing but with quantum particles instead of bits."
        }),
        json!({
            "prompt": "What is machine learning?",
            "chosen": "Machine learning is a subset of artificial intelligence that enables computers to learn from data without being explicitly programmed, using algorithms that improve automatically through experience.",
            "rejected": "Machine learning is when computers learn stuff automatically."
        }),
    ];

    let mut pref_dataset = GenericJSONDataset {
        data: preference_data,
        format: DataFormat::Auto,
        schema: None,
    };

    let training_format = pref_dataset.detect_training_format()?;
    println!("🔍 Detected format: {:?}", training_format);

    let training_dataset = pref_dataset.to_training_dataset()?;
    println!("✅ Converted to universal TrainingDataset with {} samples", training_dataset.samples.len());

    // Quality assessment
    let quality_report = training_dataset.quality_assessment();
    println!("📈 Quality Score: {:.2}", quality_report.overall_score);
    if !quality_report.issues.is_empty() {
        println!("⚠️  Issues found:");
        for issue in &quality_report.issues {
            println!("   - {}", issue);
        }
    }

    // Convert to different algorithms
    println!("\n🔄 Algorithm Conversions:");
    let dpo_result = training_dataset.to_algorithm_format("dpo");
    println!("   ✅ DPO format: {}", dpo_result.is_ok());

    let bco_result = training_dataset.to_algorithm_format("bco");
    println!("   ✅ BCO format: {}", bco_result.is_ok());

    // Example 2: HuggingFace Dataset Processing (Mock)
    println!("\n🤗 Example 2: HuggingFace Dataset Processing");
    println!("--------------------------------------------");
    println!("✅ HuggingFace integration ready - supports datasets like:");
    println!("   • Anthropic/hh-rlhf (preference data)");
    println!("   • Dahoas/rm-static (preference data)");
    println!("   • Hello-SimpleAI/HC3 (completion data)");
    println!("   • databricks/databricks-dolly-15k (instruction data)");

    // Example 3: Completion Dataset
    println!("\n📝 Example 3: Completion Dataset Processing");
    println!("------------------------------------------");

    let completion_data = vec![
        json!({
            "prompt": "The capital of France is",
            "completion": "Paris, the beautiful city on the Seine River.",
            "label": 1.0
        }),
        json!({
            "prompt": "2 + 2 =",
            "completion": "4, which is the sum of two plus two.",
            "label": 0.0
        }),
    ];

    let mut comp_dataset = GenericJSONDataset {
        data: completion_data,
        format: DataFormat::Auto,
        schema: None,
    };

    let comp_training_format = comp_dataset.detect_training_format()?;
    println!("🔍 Detected format: {:?}", comp_training_format);

    let comp_training_dataset = comp_dataset.to_training_dataset()?;
    let comp_quality = comp_training_dataset.quality_assessment();
    println!("📈 Quality Score: {:.2}", comp_quality.overall_score);

    // Convert to BCO
    let bco_dataset = comp_training_dataset.to_algorithm_format("bco")?;
    println!("✅ Converted to BCO format for training");

    println!("\n🎉 HLX-AI Demo Complete!");
    println!("=========================");
    println!("✨ Zero-configuration dataset processing achieved");
    println!("🔄 Universal format conversion enabled");
    println!("📊 Intelligent quality assessment active");
    println!("🤖 Ready for AI-to-AI dataset workflows");

    Ok(())
}
