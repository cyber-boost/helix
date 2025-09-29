use anyhow::Result;
use std::path::PathBuf;

pub fn preview_command(
    file: PathBuf,
    format: Option<String>,
    rows: Option<usize>,
    columns: Option<Vec<String>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::out::helix_format::HlxReader;
    use std::fs::File;
    use std::io::BufReader;
    if verbose {
        println!("🔍 Previewing file: {}", file.display());
    }
    if !file.exists() {
        return Err(format!("File not found: {}", file.display()).into());
    }
    let file_handle = File::open(&file)?;
    let reader = BufReader::new(file_handle);
    let mut hlx_reader = HlxReader::new(reader);
    match hlx_reader.read_header() {
        Ok(header) => {
            println!("📋 File Information:");
            println!("  Format: Helix Data v{}", env!("CARGO_PKG_VERSION"));
            println!("  Schema Fields: {}", header.fields.len());
            println!("  Available Columns:");
            for (i, field) in header.fields.iter().enumerate() {
                println!("    {}. {} ({})", i + 1, field.name, field.field_type);
            }
            println!("  Total Rows: {}", header.row_count);
            println!(
                "  Compression: {}", if header.is_compressed() { "Yes" } else { "No" }
            );
        }
        Err(e) => {
            println!("⚠️  Could not read header: {}", e);
        }
    }
    match hlx_reader.get_preview() {
        Ok(Some(preview_rows)) => {
            let display_rows = rows.unwrap_or(10);
            let rows_to_show = std::cmp::min(display_rows, preview_rows.len());
            println!("\n📊 Preview Data (first {} rows):", rows_to_show);
            if rows_to_show == 0 {
                println!("  No preview data available");
                return Ok(());
            }
            if let Some(first_row) = preview_rows.first() {
                if let Some(row_obj) = first_row.as_object() {
                    let headers: Vec<&str> = row_obj
                        .keys()
                        .map(|s| s.as_str())
                        .collect();
                    if let Some(specific_columns) = &columns {
                        let filtered_headers: Vec<&str> = headers
                            .iter()
                            .filter(|h| specific_columns.contains(&h.to_string()))
                            .copied()
                            .collect();
                        print_headers(&filtered_headers);
                        print_filtered_rows(
                            &preview_rows[..rows_to_show],
                            &filtered_headers,
                        );
                    } else {
                        print_headers(&headers);
                        print_rows(&preview_rows[..rows_to_show], &headers);
                    }
                }
            }
        }
        Ok(None) => {
            println!("\n📊 No preview data available in this file");
        }
        Err(e) => {
            println!("\n⚠️  Could not read preview data: {}", e);
        }
    }
    Ok(())
}pub fn print_headers(headers: &[&str]) {
    print!("  ");
    for (i, header) in headers.iter().enumerate() {
        if i > 0 {
            print!(" │ ");
        }
        print!("{:<20}", header);
    }
    println!();
    print!("  ");
    for _ in headers {
        print!("{:-<21}", "");
    }
    println!();
}
pub fn print_rows(rows: &[serde_json::Value], headers: &[&str]) {
    for row in rows {
        if let Some(row_obj) = row.as_object() {
            print!("  ");
            for (i, header) in headers.iter().enumerate() {
                if i > 0 {
                    print!(" │ ");
                }
                let value = row_obj
                    .get(*header)
                    .map(|v| format_value(v))
                    .unwrap_or_else(|| "null".to_string());
                print!("{:<20}", value);
            }
            println!();
        }
    }
}
pub fn print_filtered_rows(rows: &[serde_json::Value], headers: &[&str]) {
    for row in rows {
        if let Some(row_obj) = row.as_object() {
            print!("  ");
            for (i, header) in headers.iter().enumerate() {
                if i > 0 {
                    print!(" │ ");
                }
                let value = row_obj
                    .get(*header)
                    .map(|v| format_value(v))
                    .unwrap_or_else(|| "null".to_string());
                print!("{:<20}", value);
            }
            println!();
        }
    }
}
pub fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => format!("{:?}", value),
    }
}
