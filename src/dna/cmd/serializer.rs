use clap::Args;
use std::path::PathBuf;
use crate::mds::serializer::BinarySerializer;
use crate::HelixIR;

#[derive(Args)]
pub struct SerializeArgs {
    /// Input IR file to serialize
    #[arg(short, long)]
    input: PathBuf,

    /// Output binary file path
    #[arg(short, long)]
    output: PathBuf,

    /// Enable compression
    #[arg(long, default_value_t = false)]
    compress: bool,
}

pub fn run(args: SerializeArgs) -> anyhow::Result<()> {
    // Read the IR from the input file
    let ir_data = std::fs::read(&args.input)
        .map_err(|e| anyhow::anyhow!("Failed to read input IR file: {}", e))?;
    let ir: HelixIR = bincode::deserialize(&ir_data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize IR: {}", e))?;

    // Create serializer
    let serializer = BinarySerializer::new(args.compress);

    // Serialize IR to binary
    let binary = serializer.serialize(ir, Some(&args.input))
        .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;

    // Write binary to output file
    serializer.write_to_file(&binary, &args.output)
        .map_err(|e| anyhow::anyhow!("Failed to write binary: {}", e))?;

    println!("Serialized binary written to {}", args.output.display());
    Ok(())
}
