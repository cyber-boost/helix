use std::path::PathBuf;
use anyhow::{Result, Context};
pub fn sign_binary(
    input: PathBuf,
    key: Option<String>,
    output: Option<PathBuf>,
    verify: bool,
    verbose: bool,
) -> Result<()> {
    if verify {
        verify_signature(&input, verbose)
    } else {
        create_signature(&input, key, output, verbose)
    }
}
fn create_signature(
    input: &PathBuf,
    key: Option<String>,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔐 Creating digital signature:");
        println!("  Input: {}", input.display());
        println!("  Key: {}", key.as_deref().unwrap_or("default"));
    }
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file not found: {}", input.display()));
    }
    let content = std::fs::read(input).context("Failed to read input file")?;
    let signature = generate_signature(&content, &key)?;
    let output_path = output
        .unwrap_or_else(|| {
            let mut path = input.clone();
            path.set_extension("sig");
            path
        });
    std::fs::write(&output_path, signature).context("Failed to write signature file")?;
    println!("✅ Signature created: {}", output_path.display());
    if verbose {
        println!("  Signature size: {} bytes", std::fs::metadata(& output_path) ?.len());
    }
    Ok(())
}
fn verify_signature(input: &PathBuf, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔍 Verifying digital signature:");
        println!("  Input: {}", input.display());
    }
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file not found: {}", input.display()));
    }
    let mut sig_path = input.clone();
    sig_path.set_extension("sig");
    if !sig_path.exists() {
        return Err(anyhow::anyhow!("Signature file not found: {}", sig_path.display()));
    }
    let content = std::fs::read(input).context("Failed to read input file")?;
    let signature = std::fs::read(&sig_path).context("Failed to read signature file")?;
    let is_valid = verify_signature_content(&content, &signature)?;
    if is_valid {
        println!("✅ Signature is valid");
    } else {
        println!("❌ Signature verification failed");
        std::process::exit(1);
    }
    Ok(())
}
fn generate_signature(content: &[u8], key: &Option<String>) -> Result<Vec<u8>> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content);
    if let Some(key) = key {
        hasher.update(key.as_bytes());
    }
    let hash = hasher.finalize();
    Ok(hash.to_vec())
}
fn verify_signature_content(content: &[u8], signature: &[u8]) -> Result<bool> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    Ok(hash.as_slice() == signature)
}