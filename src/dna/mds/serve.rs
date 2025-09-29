use std::path::PathBuf;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use anyhow::{Result, Context};

pub fn serve_project(
    port: Option<u16>,
    host: Option<String>,
    directory: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    let project_dir = directory
        .unwrap_or_else(|| {
            find_project_root().unwrap_or_else(|_| std::env::current_dir().unwrap())
        });
    let port = port.unwrap_or(8080);
    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let target_dir = project_dir.join("target");
    if !target_dir.exists() {
        return Err(
            anyhow::anyhow!(
                "Target directory not found. Run 'helix build' first to compile your project."
            ),
        );
    }
    let address = format!("{}:{}", host, port);
    if verbose {
        println!("🌐 Starting MSO server:");
        println!("  Address: {}", address);
        println!("  Directory: {}", target_dir.display());
    }
    let listener = TcpListener::bind(&address).context("Failed to bind to address")?;
    println!("✅ MSO server started successfully!");
    println!("  🌐 Server running at: http://{}", address);
    println!("  📁 Serving files from: {}", target_dir.display());
    println!("  📋 Available endpoints:");
    println!("    GET / - List available binaries");
    println!("    GET /<filename> - Download binary file");
    println!("    GET /health - Health check");
    println!("  Press Ctrl+C to stop");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let target_dir = target_dir.clone();
                thread::spawn(move || {
                    handle_connection(stream, target_dir);
                });
            }
            Err(e) => {
                eprintln!("❌ Failed to accept connection: {}", e);
            }
        }
    }
    Ok(())
}
pub fn handle_connection(mut stream: TcpStream, target_dir: PathBuf) {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let request = String::from_utf8_lossy(&buffer[..size]);
            let response = handle_request(&request, &target_dir);
            if let Err(e) = stream.write_all(response.as_bytes()) {
                eprintln!("❌ Failed to write response: {}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to read request: {}", e);
        }
    }
}
pub fn handle_request(request: &str, target_dir: &PathBuf) -> String {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return create_error_response("400 Bad Request", "Empty request");
    }
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return create_error_response("400 Bad Request", "Invalid request line");
    }
    let method = parts[0];
    let path = parts[1];
    match method {
        "GET" => handle_get_request(path, target_dir),
        _ => {
            create_error_response(
                "405 Method Not Allowed",
                "Only GET requests are supported",
            )
        }
    }
}
pub fn handle_get_request(path: &str, target_dir: &PathBuf) -> String {
    match path {
        "/" => list_binaries(target_dir),
        "/health" => create_health_response(),
        _ => {
            let filename = &path[1..];
            if filename.is_empty() {
                return list_binaries(target_dir);
            }
            serve_binary_file(filename, target_dir)
        }
    }
}
pub fn list_binaries(target_dir: &PathBuf) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n");
    html.push_str(
        "<html><head><title>MSO Server - Available Binaries</title></head><body>\n",
    );
    html.push_str("<h1>MSO Server - Available Binaries</h1>\n");
    html.push_str("<p>Available compiled HELIX binaries:</p>\n");
    html.push_str("<ul>\n");
    if let Ok(entries) = std::fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".hlxb") {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    html.push_str(
                        &format!(
                            "<li><a href=\"/{}\">{}</a> ({} bytes)</li>\n", filename,
                            filename, size
                        ),
                    );
                }
            }
        }
    }
    html.push_str("</ul>\n");
    html.push_str("<hr>\n");
    html.push_str("<p><a href=\"/health\">Health Check</a></p>\n");
    html.push_str("</body></html>\n");
    create_html_response(html)
}
pub fn serve_binary_file(filename: &str, target_dir: &PathBuf) -> String {
    if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return create_error_response("400 Bad Request", "Invalid filename");
    }
    let file_path = target_dir.join(filename);
    if !file_path.exists() {
        return create_error_response(
            "404 Not Found",
            &format!("File '{}' not found", filename),
        );
    }
    match std::fs::read(&file_path) {
        Ok(content) => {
            let mut response = String::new();
            response.push_str("HTTP/1.1 200 OK\r\n");
            response.push_str("Content-Type: application/octet-stream\r\n");
            response.push_str(&format!("Content-Length: {}\r\n", content.len()));
            response
                .push_str(
                    &format!(
                        "Content-Disposition: attachment; filename=\"{}\"\r\n", filename
                    ),
                );
            response.push_str("\r\n");
            response
        }
        Err(_) => {
            create_error_response("500 Internal Server Error", "Failed to read file")
        }
    }
}
pub fn create_health_response() -> String {
    let health_json = r#"{"status": "healthy", "service": "mso-server", "timestamp": "2024-01-01T00:00:00Z"}"#;
    let mut response = String::new();
    response.push_str("HTTP/1.1 200 OK\r\n");
    response.push_str("Content-Type: application/json\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", health_json.len()));
    response.push_str("\r\n");
    response.push_str(health_json);
    response
}
pub fn create_html_response(html: String) -> String {
    let mut response = String::new();
    response.push_str("HTTP/1.1 200 OK\r\n");
    response.push_str("Content-Type: text/html\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", html.len()));
    response.push_str("\r\n");
    response.push_str(&html);
    response
}
pub fn create_error_response(status: &str, message: &str) -> String {
    let html = format!(
        "<!DOCTYPE html>\n<html><head><title>Error</title></head><body>\n<h1>{}</h1>\n<p>{}</p>\n</body></html>",
        status, message
    );
    let mut response = String::new();
    response.push_str(&format!("HTTP/1.1 {}\r\n", status));
    response.push_str("Content-Type: text/html\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", html.len()));
    response.push_str("\r\n");
    response.push_str(&html);
    response
}
pub fn find_project_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    loop {
        let manifest_path = current_dir.join("project.hlx");
        if manifest_path.exists() {
            return Ok(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(anyhow::anyhow!("No HELIX project found. Run 'helix init' first."))
}