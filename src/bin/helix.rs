use helix_core::compiler::cli;
use std::process;

#[tokio::main]
async fn main() {
    std::panic::set_hook(
        Box::new(|panic_info| {
            eprintln!("❌ HELIX Compiler panicked!");
            if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                eprintln!("   Error: {}", s);
            }
            if let Some(location) = panic_info.location() {
                eprintln!("   Location: {}:{}", location.file(), location.line());
            }
            eprintln!("\n   This is a bug. Please report it at:");
            eprintln!("   https://github.com/cyber-boost/helix/issues");
        }),
    );
    if let Err(e) = cli::run().await {
        eprintln!("Error: {}", e);
        let mut cause = e.source();
        while let Some(err) = cause {
            eprintln!("  Caused by: {}", err);
            cause = err.source();
        }
        process::exit(1);
    }
}