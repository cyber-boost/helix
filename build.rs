use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_or_create_helix_dir() -> std::io::Result<PathBuf> {

    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE")) // fallback for Windows
        .map(PathBuf::from)
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not locate a home directory",
            )
        })?;
    let helix_dir = home_dir.join(".dna").join("hlx");
    if !helix_dir.exists() {
        fs::create_dir_all(&helix_dir)?;
        
    }
    let _ = ensure_calc()?;
    Ok(helix_dir)
}

/// Ensure calculator directory exists and return its path.
/// If calculator.pest does not exist, create it with the default grammar.
pub fn ensure_calc() -> std::io::Result<PathBuf> {

    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not locate a home directory",
            )
        })?;
    let calc_dir = home_dir.join(".dna").join("calc");
    if !calc_dir.exists() {
        fs::create_dir_all(&calc_dir)?;
    }
    Ok(calc_dir)
}


fn main() {

    let _ = ensure_calc();
    
    let _ = get_or_create_helix_dir();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src_grammar = Path::new(&manifest_dir).join("src").join("dna").join("ops").join("ulator.pest");

    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("Could not determine home directory");
    let dest_dir = Path::new(&home).join(".dna").join("calc");

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        panic!("Failed to create {:?}: {}", dest_dir, e);
    }

    let dest_file = dest_dir.join("ulator.pest");

    if let Err(e) = fs::copy(&src_grammar, &dest_file) {
        panic!(
            "Failed to copy grammar from {:?} to {:?}: {}",
            src_grammar, dest_file, e
        );
    }
    println!(
        "cargo:rustc-env=ULATOR_GRAMMAR_PATH={}",
        dest_file.display()
    );

    println!("cargo:rerun-if-changed={}", src_grammar.display());

}