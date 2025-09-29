//! Simple HLX Usage Example
//!
//! This example demonstrates the extremely simple HLX API:
//! - `hlx.section.key` - dot notation access
//! - `hlx[section][key]` - bracket notation access
//! - `hlx.get.section.key` - get method
//! - `hlx.set.section.key` - set method
//! - `hlx.server.start()` - start server
//! - `hlx.watch()` - watch mode
//! - `hlx.process()` - process/compile file
//! - `hlx.compile()` - compile

use helix::{Hlx, value::Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple HLX Interface Demo ===\n");

    // 1. Load an HLX file (text or binary)
    println!("1. Loading HLX file...");
    // let hlx = Hlx::load("config.hlx").await?;
    // For demo, create empty and populate
    let mut hlx = Hlx::new().await?;

    // 2. Set values using the set method
    println!("2. Setting configuration values...");
    hlx.set("database", "host", Value::String("localhost".to_string()));
    hlx.set("database", "port", Value::Number(5432.0));
    hlx.set("database", "name", Value::String("mydb".to_string()));

    hlx.set("server", "host", Value::String("0.0.0.0".to_string()));
    hlx.set("server", "port", Value::Number(8080.0));

    // 3. Dot notation access: hlx.get(section, key)
    println!("3. Using dot notation access...");
    if let Some(host) = hlx.get("database", "host") {
        println!("Database host: {:?}", host);
    }

    // 4. Set method: hlx.set(section, key, value)
    println!("4. Using set method...");
    hlx.set("cache", "ttl", Value::Number(3600.0));
    hlx.set("cache", "enabled", Value::Bool(true));

    // 5. Execute operators directly
    println!("5. Executing operators...");

    // Date and time
    let date = hlx.execute(r#"@date("Y-m-d H:i:s")"#).await?;
    println!("Current date: {:?}", date);

    // UUID generation
    let uuid = hlx.execute("@uuid()").await?;
    println!("Generated UUID: {:?}", uuid);

    // Math operations
    let math = hlx.execute(r#"@math("10 + 5 * 2")"#).await?;
    println!("Math result: {:?}", math);

    // Calculator with variables
    let calc = hlx.execute(r#"@calc("price = 100; tax = 0.08; total = price * (1 + tax)")"#).await?;
    println!("Calculator result: {:?}", calc);

    // String operations
    let upper = hlx.execute(r#"@string("hello world", "upper")"#).await?;
    println!("Uppercase: {:?}", upper);

    // JSON operations
    let json = hlx.execute(r#"@json('{"name":"test","value":42}', "parse")"#).await?;
    println!("JSON parse: {:?}", json);

    // Base64 encoding
    let b64 = hlx.execute(r#"@base64("hello", "encode")"#).await?;
    println!("Base64 encode: {:?}", b64);

    // Hash operations
    let hash = hlx.execute(r#"@hash("password", "sha256")"#).await?;
    println!("SHA256 hash: {:?}", hash);

    // 6. Conditional operations
    println!("6. Conditional operations...");

    let if_result = hlx.execute(r#"@if(condition="@math('5 > 3')", then="greater", else="less")"#).await?;
    println!("If condition: {:?}", if_result);

    let switch_result = hlx.execute(r#"@switch(value="2", cases="{'1':'one','2':'two','3':'three'}", default="unknown")"#).await?;
    println!("Switch result: {:?}", switch_result);

    // 7. Array operations
    println!("7. Array operations...");

    let filter_result = hlx.execute(r#"@filter(array="[1,2,3,4,5]", condition="@math('value > 3')")"#).await?;
    println!("Filter result: {:?}", filter_result);

    let map_result = hlx.execute(r#"@map(array="[1,2,3]", transform="@math('value * 2')")"#).await?;
    println!("Map result: {:?}", map_result);

    let reduce_result = hlx.execute(r#"@reduce(array="[1,2,3,4]", initial="0", operation="@math('acc + value')")"#).await?;
    println!("Reduce result: {:?}", reduce_result);

    // 8. Show all sections and keys
    println!("8. Configuration structure...");
    println!("Sections: {:?}", hlx.sections());

    for section in hlx.sections() {
        if let Some(keys) = hlx.keys(section) {
            println!("  {}: {:?}", section, keys);
        }
    }

    // 9. Save configuration
    println!("9. Saving configuration...");
    // hlx.save()?; // Would save to file if file_path was set

    println!("\n=== Demo Complete ===");
    println!("The HLX interface provides extremely simple access to all Helix functionality!");
    println!("- Load files: Hlx::load('file.hlx')");
    println!("- Access data: hlx['section']['key'] or hlx.get('section', 'key')");
    println!("- Execute operators: hlx.execute('@operator(params)')");
    println!("- Server: hlx.server().await?");
    println!("- Watch: hlx.watch().await?");
    println!("- Process: hlx.process().await?");
    println!("- Compile: hlx.compile().await?");

    Ok(())
}
