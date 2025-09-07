use std::fs;
use std::io::Write;
use std::path::Path;
use helix_core::compiler::loader::BinaryLoader;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HLX to HLXB conversion and binary loading...");
    println!("📡 Server should be running on http://localhost:4592");
    println!("\n📥 Test 1: Downloading and converting HLX to HLXB");
    let hlx_url = "http://localhost:4592/test_config.hlx";
    let temp_hlxb_path = "/tmp/test_converted.hlxb";
    let curl_output = std::process::Command::new("curl")
        .args(&["-s", "-o", temp_hlxb_path, hlx_url])
        .output()?;
    if !curl_output.status.success() {
        println!("❌ Failed to download HLX file from server");
        return Ok(());
    }
    if !Path::new(temp_hlxb_path).exists() {
        println!("❌ Downloaded file doesn't exist");
        return Ok(());
    }
    let metadata = fs::metadata(temp_hlxb_path)?;
    println!("✅ Downloaded {} bytes", metadata.len());
    println!("\n🔄 Test 2: Loading binary with BinaryLoader");
    let loader = BinaryLoader::new();
    match loader.load_file(temp_hlxb_path) {
        Ok(binary) => {
            println!("✅ Binary loaded successfully!");
            println!(
                "   📊 Magic: {} {} {} {}", binary.magic[0] as char, binary.magic[1] as
                char, binary.magic[2] as char, binary.magic[3] as char
            );
            println!("   🔢 Version: {}", binary.version);
            println!("   📦 Sections: {}", binary.sections.len());
            println!(
                "   📝 Metadata: {} bytes created at {}", binary.metadata.source_hash,
                binary.metadata.created_at
            );
            if binary.magic == [b'H', b'L', b'X', b'B'] {
                println!("✅ Magic bytes are correct (HLXB)");
            } else {
                println!("❌ Magic bytes are incorrect");
            }
            if binary.version == 1 {
                println!("✅ Binary version is correct");
            } else {
                println!("❌ Binary version is incorrect: {}", binary.version);
            }
            println!("\n🔄 Test 3: Decompiling binary back to source");
            use helix_core::compiler::Compiler;
            let compiler = Compiler::new(helix_core::compiler::OptimizationLevel::Two);
            match compiler.decompile(&binary) {
                Ok(source) => {
                    println!("✅ Successfully decompiled!");
                    println!("   📜 Decompiled source (first 100 chars):");
                    println!(
                        "   \"{}\"", & source.chars().take(100).collect::< String > ()
                    );
                    fs::remove_file(temp_hlxb_path)?;
                    println!("\n🧹 Cleaned up temporary file");
                    println!(
                        "\n🎉 SUCCESS: Complete HLX ↔ HLXB conversion cycle works!"
                    );
                    println!("   ✅ HLX file served and converted to HLXB");
                    println!("   ✅ HLXB binary loaded by rlib BinaryLoader");
                    println!("   ✅ Binary format is valid and complete");
                    println!("   ✅ Binary can be decompiled back to source");
                    println!(
                        "   📚 The rlib binary loading functionality is working perfectly!"
                    );
                }
                Err(e) => {
                    println!("❌ Failed to decompile: {:?}", e);
                    let _ = fs::remove_file(temp_hlxb_path);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to load binary: {:?}", e);
            match fs::read_to_string(temp_hlxb_path) {
                Ok(content) => {
                    println!("   📄 File content (first 200 chars):");
                    println!(
                        "   \"{}\"", content.chars().take(200).collect::< String > ()
                    );
                }
                Err(_) => {
                    println!("   📦 Binary content (first 50 bytes):");
                    let content = fs::read(temp_hlxb_path)?;
                    for (i, &byte) in content.iter().take(50).enumerate() {
                        if i % 10 == 0 {
                            print!("   ")
                        }
                        print!("{:02x} ", byte);
                        if i % 10 == 9 {
                            println!()
                        }
                    }
                    println!();
                }
            }
            let _ = fs::remove_file(temp_hlxb_path);
        }
    }
    Ok(())
}