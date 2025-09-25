use std::collections::HashMap;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use chrono;
#[cfg(feature = "compiler")]
use crate::compiler::{Compiler, OptimizationLevel};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub domain: String,
    pub root_directory: PathBuf,
    pub auto_convert: bool,
    pub cache_timeout: u64,
    pub max_file_size: u64,
    pub allowed_extensions: Vec<String>,
    pub cors_enabled: bool,
    pub verbose: bool,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 4592,
            domain: "localhost".to_string(),
            root_directory: PathBuf::from("."),
            auto_convert: true,
            cache_timeout: 300,
            max_file_size: 10 * 1024 * 1024,
            allowed_extensions: vec!["hlx".to_string(), "hlxb".to_string()],
            cors_enabled: true,
            verbose: false,
        }
    }
}
#[derive(Debug, Clone)]
struct CacheEntry {
    content: Vec<u8>,
    content_type: String,
    last_modified: SystemTime,
    etag: String,
}
pub struct HelixServer {
    config: ServerConfig,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}
impl HelixServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn start(&self) -> Result<()> {
        let bind_address = format!("0.0.0.0:{}", self.config.port);
        let display_address = format!("{}:{}", self.config.domain, self.config.port);
        let listener = TcpListener::bind(&bind_address)
            .context(format!("Failed to bind to address {}", bind_address))?;
        println!("🚀 HELIX Configuration Server started!");
        println!("  🌐 Listening on: http://{}", display_address);
        println!("  📁 Root Directory: {}", self.config.root_directory.display());
        println!("  🔄 Auto-convert .hlx to .hlxb: {}", self.config.auto_convert);
        println!("  📋 Available endpoints:");
        println!("    GET / - Server information and file listing");
        println!("    GET /<filename> - Serve .hlx or .hlxb file");
        println!("    GET /<domain>.<hlxb> - Domain-based file serving");
        println!("    GET /health - Health check");
        println!("    GET /config - Server configuration");
        println!("  Press Ctrl+C to stop");
        let cache_clone = Arc::clone(&self.cache);
        let cache_timeout = self.config.cache_timeout;
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60));
                Self::cleanup_cache(&cache_clone, cache_timeout);
            }
        });
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let server = self.clone();
                    thread::spawn(move || {
                        if let Err(e) = server.handle_connection(stream) {
                            eprintln!("❌ Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Failed to accept connection: {}", e);
                }
            }
        }
        Ok(())
    }
    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = [0; 8192];
        let size = stream.read(&mut buffer).context("Failed to read from stream")?;
        let request = String::from_utf8_lossy(&buffer[..size]);
        let response = self.handle_request(&request)?;
        stream.write_all(response.as_bytes()).context("Failed to write response")?;
        Ok(())
    }
    fn handle_request(&self, request: &str) -> Result<String> {
        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return Ok(self.create_error_response("400 Bad Request", "Empty request"));
        }
        let request_line = lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(
                self.create_error_response("400 Bad Request", "Invalid request line"),
            );
        }
        let method = parts[0];
        let path = parts[1];
        match method {
            "GET" => self.handle_get_request(path),
            "OPTIONS" => Ok(self.create_options_response()),
            _ => {
                Ok(
                    self
                        .create_error_response(
                            "405 Method Not Allowed",
                            "Only GET and OPTIONS requests are supported",
                        ),
                )
            }
        }
    }
    fn handle_get_request(&self, path: &str) -> Result<String> {
        match path {
            "/" => Ok(self.create_info_response()),
            "/health" => Ok(self.create_health_response()),
            "/config" => Ok(self.create_config_response()),
            "/files" => Ok(self.list_available_files()),
            "/files-html" => {
                let file_list = self.list_available_files();
                Ok(self.create_html_response(&file_list))
            }
            path => self.serve_file(path),
        }
    }
    fn serve_file(&self, path: &str) -> Result<String> {
        let filename = path.trim_start_matches('/');
        if filename.ends_with(".hlxb") && filename.contains('.')
            && !filename.contains('/')
        {
            let parts: Vec<&str> = filename.split('.').collect();
            if parts.len() >= 2 && parts[parts.len() - 1] == "hlxb" {
                return self.serve_domain_file(filename);
            }
        }
        if let Some(cached) = self.get_cached_file(filename) {
            return Ok(
                self
                    .create_file_response(
                        &cached.content,
                        &cached.content_type,
                        &cached.etag,
                        true,
                    ),
            );
        }
        let file_path = self.config.root_directory.join(filename);
        if !file_path.exists() {
            let file_path = self.find_file_with_extension(filename)?;
            return self.serve_file_from_path(&file_path);
        }
        self.serve_file_from_path(&file_path)
    }
    fn serve_domain_file(&self, domain_filename: &str) -> Result<String> {
        let parts: Vec<&str> = domain_filename.split('.').collect();
        if parts.len() < 2 {
            return Ok(
                self
                    .create_error_response(
                        "400 Bad Request",
                        "Invalid domain file format",
                    ),
            );
        }
        let domain = parts[0];
        let extension = parts[parts.len() - 1];
        if extension != "hlxb" {
            return Ok(
                self
                    .create_error_response(
                        "400 Bad Request",
                        "Domain files must have .hlxb extension",
                    ),
            );
        }
        let filename = format!("{}.hlxb", domain);
        let file_path = self.config.root_directory.join(&filename);
        if file_path.exists() {
            return self.serve_file_from_path(&file_path);
        }
        if self.config.auto_convert {
            let hlx_filename = format!("{}.hlx", domain);
            let hlx_path = self.config.root_directory.join(&hlx_filename);
            if hlx_path.exists() {
                return self.convert_and_serve_file(&hlx_path, &file_path);
            }
        }
        Ok(
            self
                .create_error_response(
                    "404 Not Found",
                    &format!("Domain configuration '{}' not found", domain),
                ),
        )
    }
    fn find_file_with_extension(&self, base_filename: &str) -> Result<PathBuf> {
        for ext in &self.config.allowed_extensions {
            let filename = format!("{}.{}", base_filename, ext);
            let file_path = self.config.root_directory.join(&filename);
            if file_path.exists() {
                return Ok(file_path);
            }
        }
        Err(
            anyhow::anyhow!(
                "File '{}' not found with any allowed extension", base_filename
            ),
        )
    }
    fn serve_file_from_path(&self, file_path: &Path) -> Result<String> {
        let metadata = fs::metadata(file_path)?;
        if metadata.len() > self.config.max_file_size {
            return Ok(
                self
                    .create_error_response(
                        "413 Payload Too Large",
                        "File exceeds maximum size",
                    ),
            );
        }
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content_type = match extension {
            "hlx" => "application/x-helix",
            "hlxb" => "application/x-helix-binary",
            _ => "application/octet-stream",
        };
        let content = fs::read(file_path)?;
        let (final_content, final_content_type) = if extension == "hlx"
            && self.config.auto_convert
        {
            println!("🔄 Converting HLX to HLXB: {}", file_path.display());
            let hlxb_content = self.convert_hlx_to_hlxb(&content)?;
            println!(
                "✅ Conversion complete: {} bytes -> {} bytes", content.len(),
                hlxb_content.len()
            );
            (hlxb_content, "application/x-helix-binary".to_string())
        } else {
            (content, content_type.to_string())
        };
        let etag = self.generate_etag(&final_content);
        self.cache_file(
            file_path.to_string_lossy().as_ref(),
            &final_content,
            &final_content_type,
            etag.clone(),
        );
        Ok(self.create_file_response(&final_content, &final_content_type, &etag, false))
    }
    #[cfg(feature = "compiler")]
    fn convert_hlx_to_hlxb(&self, hlx_content: &[u8]) -> Result<Vec<u8>> {
        let hlx_source = String::from_utf8_lossy(hlx_content);
        let compiler = Compiler::new(OptimizationLevel::Two);
        let binary = compiler
            .compile_source(&hlx_source, None)
            .map_err(|e| anyhow::anyhow!("Compilation failed: {:?}", e))?;
        let hlxb_data = bincode::serialize(&binary)
            .map_err(|e| anyhow::anyhow!("Binary serialization failed: {:?}", e))?;
        Ok(hlxb_data)
    }
    #[cfg(not(feature = "compiler"))]
    fn convert_hlx_to_hlxb(&self, hlx_content: &[u8]) -> Result<Vec<u8>> {
        const MAGIC_BYTES: [u8; 4] = *b"HLXB";
        const BINARY_VERSION: u32 = 1;
        let mut result = Vec::new();
        result.extend_from_slice(&MAGIC_BYTES);
        result.extend_from_slice(&BINARY_VERSION.to_le_bytes());
        result.extend_from_slice(&(hlx_content.len() as u32).to_le_bytes());
        result.extend_from_slice(&(0u32).to_le_bytes());
        result.extend_from_slice(hlx_content);
        Ok(result)
    }
    fn convert_and_serve_file(
        &self,
        hlx_path: &Path,
        hlxb_path: &Path,
    ) -> Result<String> {
        let hlx_content = fs::read(hlx_path)?;
        let hlxb_content = self.convert_hlx_to_hlxb(&hlx_content)?;
        if let Err(e) = fs::write(hlxb_path, &hlxb_content) {
            if self.config.verbose {
                eprintln!("⚠️  Warning: Failed to save converted file: {}", e);
            }
        }
        let etag = self.generate_etag(&hlxb_content);
        self.cache_file(
            hlxb_path.to_string_lossy().as_ref(),
            &hlxb_content,
            "application/x-helix-binary",
            etag.clone(),
        );
        Ok(
            self
                .create_file_response(
                    &hlxb_content,
                    "application/x-helix-binary",
                    &etag,
                    false,
                ),
        )
    }
    fn generate_etag(&self, content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("\"{:x}\"", hasher.finish())
    }
    fn cache_file(&self, key: &str, content: &[u8], content_type: &str, etag: String) {
        let entry = CacheEntry {
            content: content.to_vec(),
            content_type: content_type.to_string(),
            last_modified: SystemTime::now(),
            etag,
        };
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key.to_string(), entry);
        }
    }
    fn get_cached_file(&self, key: &str) -> Option<CacheEntry> {
        if let Ok(cache) = self.cache.lock() { cache.get(key).cloned() } else { None }
    }
    fn cleanup_cache(cache: &Arc<Mutex<HashMap<String, CacheEntry>>>, timeout: u64) {
        if let Ok(mut cache) = cache.lock() {
            let now = SystemTime::now();
            cache
                .retain(|_, entry| {
                    now.duration_since(entry.last_modified)
                        .map(|duration| duration.as_secs() < timeout)
                        .unwrap_or(false)
                });
        }
    }
    fn create_info_response(&self) -> String {
        let files = self.list_available_files_json();
        let server_info = serde_json::json!(
            { "server" : { "name" : "HELIX Configuration Server", "domain" : self.config
            .domain, "port" : self.config.port, "root_directory" : self.config
            .root_directory.display().to_string(), "auto_convert" : self.config
            .auto_convert, "cache_timeout" : self.config.cache_timeout, "max_file_size" :
            self.config.max_file_size, "cors_enabled" : self.config.cors_enabled },
            "endpoints" : { "root" : "GET / - Server information (JSON)", "health" :
            "GET /health - Health check (JSON)", "config" :
            "GET /config - Server configuration (JSON)", "files" :
            "GET /filename.hlx - Serve HLX file", "binary_files" :
            "GET /filename.hlxb - Serve HLXB file", "domain_files" :
            "GET /domain.hlxb - Domain-based configuration" }, "available_files" : files
            }
        );
        let json = server_info.to_string();
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Content-Type: application/json\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", json.len()));
        if self.config.cors_enabled {
            response.push_str("Access-Control-Allow-Origin: *\r\n");
        }
        response.push_str("\r\n");
        response.push_str(&json);
        response
    }
    fn list_available_files_json(&self) -> Vec<serde_json::Value> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.config.root_directory) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if self
                        .config
                        .allowed_extensions
                        .iter()
                        .any(|ext| filename.ends_with(ext))
                    {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let extension = filename.rsplit('.').next().unwrap_or("");
                        let name_without_ext = filename
                            .trim_end_matches(&format!(".{}", extension));
                        files
                            .push(
                                serde_json::json!(
                                    { "name" : filename, "name_without_extension" :
                                    name_without_ext, "extension" : extension, "size" : size,
                                    "url" : format!("/{}", filename) }
                                ),
                            );
                    }
                }
            }
        }
        files
    }
    fn list_available_files(&self) -> String {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.config.root_directory) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if self
                        .config
                        .allowed_extensions
                        .iter()
                        .any(|ext| filename.ends_with(ext))
                    {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        files
                            .push(
                                format!(
                                    r#"<div class="file-item">📄 <a href="/{}">{}</a> ({} bytes)</div>"#,
                                    filename, filename, size
                                ),
                            );
                    }
                }
            }
        }
        if files.is_empty() {
            r#"<em>No .hlx or .hlxb files found</em>"#.to_string()
        } else {
            files.join("\n")
        }
    }
    fn create_health_response(&self) -> String {
        let health = serde_json::json!(
            { "status" : "healthy", "service" : "helix-config-server", "domain" : self
            .config.domain, "port" : self.config.port, "timestamp" : chrono::Utc::now()
            .to_rfc3339(), "uptime" : "unknown" }
        );
        let json = health.to_string();
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Content-Type: application/json\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", json.len()));
        response.push_str("\r\n");
        response.push_str(&json);
        response
    }
    fn create_config_response(&self) -> String {
        let config = serde_json::to_string_pretty(&self.config).unwrap_or_default();
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Content-Type: application/json\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", config.len()));
        response.push_str("\r\n");
        response.push_str(&config);
        response
    }
    fn create_file_response(
        &self,
        content: &[u8],
        content_type: &str,
        etag: &str,
        cached: bool,
    ) -> String {
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str(&format!("Content-Type: {}\r\n", content_type));
        response.push_str(&format!("Content-Length: {}\r\n", content.len()));
        response.push_str(&format!("ETag: {}\r\n", etag));
        response.push_str("Cache-Control: public, max-age=300\r\n");
        if self.config.cors_enabled {
            response.push_str("Access-Control-Allow-Origin: *\r\n");
            response.push_str("Access-Control-Allow-Methods: GET, OPTIONS\r\n");
            response.push_str("Access-Control-Allow-Headers: *\r\n");
        }
        if cached {
            response.push_str("X-Cache: HIT\r\n");
        } else {
            response.push_str("X-Cache: MISS\r\n");
        }
        response.push_str("\r\n");
        response.push_str(&String::from_utf8_lossy(content));
        response
    }
    fn create_options_response(&self) -> String {
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Access-Control-Allow-Origin: *\r\n");
        response.push_str("Access-Control-Allow-Methods: GET, OPTIONS\r\n");
        response.push_str("Access-Control-Allow-Headers: *\r\n");
        response.push_str("Content-Length: 0\r\n");
        response.push_str("\r\n");
        response
    }
    fn create_html_response(&self, html: &str) -> String {
        let mut response = String::new();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Content-Type: text/html\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", html.len()));
        if self.config.cors_enabled {
            response.push_str("Access-Control-Allow-Origin: *\r\n");
        }
        response.push_str("\r\n");
        response.push_str(html);
        response
    }
    fn create_error_response(&self, status: &str, message: &str) -> String {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Error - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; text-align: center; margin: 50px; }}
        .error {{ color: #d32f2f; background: #ffebee; padding: 20px; border-radius: 8px; }}
    </style>
</head>
<body>
    <div class="error">
        <h1>{}</h1>
        <p>{}</p>
    </div>
</body>
</html>"#,
            status, status, message
        );
        let mut response = String::new();
        response.push_str(&format!("HTTP/1.1 {}\r\n", status));
        response.push_str("Content-Type: text/html\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", html.len()));
        if self.config.cors_enabled {
            response.push_str("Access-Control-Allow-Origin: *\r\n");
        }
        response.push_str("\r\n");
        response.push_str(&html);
        response
    }
}
impl Clone for HelixServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: Arc::clone(&self.cache),
        }
    }
}
pub fn start_server(config: ServerConfig) -> Result<()> {
    let server = HelixServer::new(config);
    server.start()
}
pub fn start_default_server() -> Result<()> {
    let config = ServerConfig::default();
    start_server(config)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 4592);
        assert_eq!(config.domain, "localhost");
        assert!(config.auto_convert);
    }
    #[test]
    fn test_convert_hlx_to_hlxb() {
        let server = HelixServer::new(ServerConfig::default());
        let hlx_content = b"agent 'test' { model = 'gpt-4' }";
        let hlxb_content = server.convert_hlx_to_hlxb(hlx_content).unwrap();
        assert_eq!(& hlxb_content[0..4], b"HLXB");
        #[cfg(feature = "compiler")]
        {
            assert!(hlxb_content.len() > 4);
        }
        #[cfg(not(feature = "compiler"))]
        {
            let size = u32::from_le_bytes([
                hlxb_content[8],
                hlxb_content[9],
                hlxb_content[10],
                hlxb_content[11],
            ]);
            assert_eq!(size as usize, hlx_content.len());
            assert_eq!(& hlxb_content[16..], hlx_content);
        }
    }
    #[test]
    fn test_generate_etag() {
        let server = HelixServer::new(ServerConfig::default());
        let content = b"test content";
        let etag1 = server.generate_etag(content);
        let etag2 = server.generate_etag(content);
        assert_eq!(etag1, etag2);
        let etag3 = server.generate_etag(b"different content");
        assert_ne!(etag1, etag3);
    }
}