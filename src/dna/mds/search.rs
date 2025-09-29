// src/compiler/search.rs - Unified search engine for .emb embeddings
// Compatible with both emb.rs (basic) and ind.rs (intelligent) spider outputs
// Integrated with Helix CLI for searching configuration files and embeddings

use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Unified search result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: Option<f32>,
    pub content: Option<String>,
    pub metadata: serde_json::Value,
}

/// Loaded embedding with search capabilities
#[derive(Debug, Clone)]
pub struct LoadedEmbedding {
    pub id: String,
    pub content: Option<String>,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
}

/// Load embeddings from a .emb directory
pub async fn load_embeddings(emb_path: &Path) -> Result<Vec<LoadedEmbedding>> {
    let ndjson_path = emb_path.join("embeddings.ndjson");

    if !ndjson_path.exists() {
        return Err(anyhow::anyhow!("Embeddings file not found: {}", ndjson_path.display()));
    }

    let content = tokio::fs::read_to_string(&ndjson_path).await?;
    let mut embeddings = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let doc: serde_json::Value = serde_json::from_str(line)?;

        // Extract the embedding vector (handle both raw and base64 formats)
        let embedding = if let Some(raw_embedding) = doc.get("embedding") {
            if raw_embedding.is_array() {
                // Raw float array
                let arr: Vec<f32> = raw_embedding.as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                arr
            } else if raw_embedding.is_string() {
                // Base64 encoded
                decode_base64_embedding(raw_embedding.as_str().unwrap())?
            } else {
                continue; // Skip invalid embeddings
            }
        } else {
            continue; // Skip documents without embeddings
        };

        let loaded = LoadedEmbedding {
            id: doc.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),

            content: doc.get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),

            embedding,

            metadata: doc.get("metadata")
                .cloned()
                .unwrap_or(serde_json::json!({})),
        };

        embeddings.push(loaded);
    }

    Ok(embeddings)
}

/// Decode base64-encoded embedding back to Vec<f32>
fn decode_base64_embedding(b64_str: &str) -> Result<Vec<f32>> {
    use base64::{Engine as _, engine::general_purpose};

    let bytes = general_purpose::STANDARD.decode(b64_str)?;
    let mut embedding = Vec::new();

    // Convert bytes back to f32 values (little-endian)
    for chunk in bytes.chunks_exact(4) {
        let bytes_array: [u8; 4] = chunk.try_into().unwrap();
        let float = f32::from_le_bytes(bytes_array);
        embedding.push(float);
    }

    Ok(embedding)
}

/// Search embeddings using various strategies
pub async fn search_embeddings(
    embeddings: &[LoadedEmbedding],
    query: &str,
    search_type: &str,
    limit: usize,
    threshold: f32,
) -> Result<Vec<SearchResult>> {
    let results = match search_type {
        "semantic" => semantic_search(embeddings, query, limit, threshold).await?,
        "keyword" => keyword_search(embeddings, query, limit).await?,
        "tag" => tag_search(embeddings, query, limit).await?,
        "regex" => regex_search(embeddings, query, limit).await?,
        "natural" => natural_language_search(embeddings, query, limit, threshold).await?,
        _ => semantic_search(embeddings, query, limit, threshold).await?,
    };

    Ok(results)
}

/// Semantic similarity search using embeddings
async fn semantic_search(
    embeddings: &[LoadedEmbedding],
    query: &str,
    limit: usize,
    _threshold: f32,
) -> Result<Vec<SearchResult>> {
    // For semantic search, we need to either:
    // 1. Generate a query embedding (requires ML model)
    // 2. Use keyword fallback for now

    // Simple keyword-based semantic approximation
    keyword_search(embeddings, query, limit).await
}

/// Keyword-based search
async fn keyword_search(
    embeddings: &[LoadedEmbedding],
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let keywords: Vec<String> = query.to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut scored_results: Vec<(f32, SearchResult)> = Vec::new();

    for embedding in embeddings {
        let mut score = 0.0;
        let mut matches = 0;

        // Score based on content matching
        if let Some(content) = &embedding.content {
            let content_lower = content.to_lowercase();

            for keyword in &keywords {
                if content_lower.contains(keyword) {
                    score += 1.0;
                    matches += 1;
                }
            }

            // Boost score for multiple keyword matches
            if matches > 1 {
                score *= 1.0 + (matches as f32 * 0.1);
            }
        }

        // Score based on metadata matching
        if let Some(file_path) = embedding.metadata.get("file_path") {
            if let Some(path_str) = file_path.as_str() {
                let path_lower = path_str.to_lowercase();

                for keyword in &keywords {
                    if path_lower.contains(keyword) {
                        score += 0.5; // Lower weight for path matches
                    }
                }
            }
        }

        if score > 0.0 {
            let result = SearchResult {
                id: embedding.id.clone(),
                score: Some(score),
                content: embedding.content.clone(),
                metadata: embedding.metadata.clone(),
            };

            scored_results.push((score, result));
        }
    }

    // Sort by score descending and take top results
    scored_results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let results: Vec<SearchResult> = scored_results
        .into_iter()
        .take(limit)
        .map(|(_, result)| result)
        .collect();

    Ok(results)
}

/// Tag-based search (for ind.rs enhanced embeddings)
async fn tag_search(
    embeddings: &[LoadedEmbedding],
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let tag = query.strip_prefix("tag:").unwrap_or(query);

    let mut results = Vec::new();

    for embedding in embeddings {
        // Check if this embedding has the requested tag
        if let Some(auto_tags) = embedding.metadata.get("auto_tags") {
            if let Some(tags_array) = auto_tags.as_array() {
                let has_tag = tags_array.iter().any(|tag_value| {
                    tag_value.as_str()
                        .map(|s| s == tag)
                        .unwrap_or(false)
                });

                if has_tag {
                    let result = SearchResult {
                        id: embedding.id.clone(),
                        score: Some(1.0), // Tag matches are exact
                        content: embedding.content.clone(),
                        metadata: embedding.metadata.clone(),
                    };

                    results.push(result);

                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Regex-based search
async fn regex_search(
    embeddings: &[LoadedEmbedding],
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let pattern = query.trim_start_matches('/').trim_end_matches('/');
    let regex = regex::Regex::new(pattern)?;

    let mut results = Vec::new();

    for embedding in embeddings {
        let mut matches = false;

        // Search in content
        if let Some(content) = &embedding.content {
            if regex.is_match(content) {
                matches = true;
            }
        }

        // Search in metadata
        if let Some(file_path) = embedding.metadata.get("file_path") {
            if let Some(path_str) = file_path.as_str() {
                if regex.is_match(path_str) {
                    matches = true;
                }
            }
        }

        if matches {
            let result = SearchResult {
                id: embedding.id.clone(),
                score: Some(1.0), // Regex matches are exact
                content: embedding.content.clone(),
                metadata: embedding.metadata.clone(),
            };

            results.push(result);

            if results.len() >= limit {
                break;
            }
        }
    }

    Ok(results)
}

/// Natural language search (combines semantic + keyword)
async fn natural_language_search(
    embeddings: &[LoadedEmbedding],
    query: &str,
    limit: usize,
    threshold: f32,
) -> Result<Vec<SearchResult>> {
    // For now, combine semantic and keyword search
    let mut semantic_results = semantic_search(embeddings, query, limit * 2, threshold).await?;
    let keyword_results = keyword_search(embeddings, query, limit).await?;

    // Merge and deduplicate results
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for result in semantic_results.drain(..).chain(keyword_results) {
        if seen.insert(result.id.clone()) {
            merged.push(result);
            if merged.len() >= limit {
                break;
            }
        }
    }

    Ok(merged)
}

/// Calculate cosine similarity between two embedding vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a * norm_b > 0.0 {
        dot_product / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Get statistics about loaded embeddings
pub async fn get_embedding_stats(embeddings: &[LoadedEmbedding]) -> HashMap<String, serde_json::Value> {
    let mut stats = HashMap::new();

    stats.insert("total_documents".to_string(), embeddings.len().into());
    stats.insert("has_embeddings".to_string(),
        embeddings.iter().any(|e| !e.embedding.is_empty()).into());

    // Count by file type
    let mut file_types = HashMap::new();
    for embedding in embeddings {
        if let Some(file_type) = embedding.metadata.get("file_type") {
            if let Some(type_str) = file_type.as_str() {
                *file_types.entry(type_str.to_string()).or_insert(0) += 1;
            }
        }
    }
    stats.insert("file_types".to_string(), serde_json::to_value(file_types).unwrap());

    // Count by category (for ind.rs enhanced embeddings)
    let mut categories = HashMap::new();
    for embedding in embeddings {
        if let Some(category) = embedding.metadata.get("category") {
            if let Some(cat_str) = category.as_str() {
                *categories.entry(cat_str.to_string()).or_insert(0) += 1;
            }
        }
    }
    stats.insert("categories".to_string(), serde_json::to_value(categories).unwrap());

    // Average embedding dimension
    let avg_dim = if !embeddings.is_empty() {
        embeddings.iter().map(|e| e.embedding.len()).sum::<usize>() / embeddings.len()
    } else {
        0
    };
    stats.insert("avg_embedding_dimension".to_string(), avg_dim.into());

    stats
}

/// Auto-find .emb directories on the system
pub async fn find_embedding_directories() -> Result<Vec<PathBuf>> {
    let mut emb_dirs = Vec::new();

    // Common locations to search
    let search_paths = vec![
        "/opt/embeddings",
        "/var/embeddings",
        "/usr/local/embeddings",
        "/home/embeddings",
        "./embeddings",
        "../embeddings",
    ];

    // Also search in common project directories
    for path in &search_paths {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() && path_buf.is_dir() {
            // Look for .emb directories
            if let Ok(entries) = std::fs::read_dir(&path_buf) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.extension().map_or(false, |ext| ext == "emb") {
                        emb_dirs.push(path);
                    }
                }
            }
        }
    }

    // Also check current directory recursively
    find_emb_dirs_recursive(".", &mut emb_dirs)?;

    Ok(emb_dirs)
}

fn find_emb_dirs_recursive(dir: &str, emb_dirs: &mut Vec<PathBuf>) -> Result<()> {
    let path = Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if entry_path.extension().map_or(false, |ext| ext == "emb") {
                    emb_dirs.push(entry_path);
                } else {
                    // Recurse into subdirectories (but avoid deep recursion)
                    if entry_path.components().count() < 10 {
                        let _ = find_emb_dirs_recursive(entry_path.to_str().unwrap_or(""), emb_dirs);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Search Helix configuration files directly
pub async fn search_hlx_files(query: &str, search_type: &str, limit: usize) -> Result<Vec<SearchResult>> {
    use walkdir::WalkDir;

    let mut results = Vec::new();
    let keywords: Vec<String> = query.to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    // Find all .hlx files
    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == "hlx" {
                    // Read and search the file
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        let mut score = 0.0;
                        let mut matches = 0;
                        let content_lower = content.to_lowercase();

                        for keyword in &keywords {
                            if content_lower.contains(keyword) {
                                score += 1.0;
                                matches += 1;
                            }
                        }

                        if score > 0.0 {
                            // Get a preview of the content around the match
                            let preview = get_content_preview(&content, &keywords);

                            let result = SearchResult {
                                id: entry.path().display().to_string(),
                                score: Some(score),
                                content: Some(preview),
                                metadata: serde_json::json!({
                                    "file_path": entry.path().display().to_string(),
                                    "file_type": "hlx",
                                    "size": content.len()
                                }),
                            };

                            results.push(result);

                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by score
    results.sort_by(|a, b| b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

/// Get a preview of content around keyword matches
fn get_content_preview(content: &str, keywords: &[String]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut preview_lines = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        for keyword in keywords {
            if line_lower.contains(keyword) {
                // Add context lines
                let start = i.saturating_sub(1);
                let end = (i + 2).min(lines.len());

                for j in start..end {
                    if j == i {
                        preview_lines.push(format!("▶ {}", lines[j]));
                    } else {
                        preview_lines.push(format!("  {}", lines[j]));
                    }
                }
                break; // Only add once per line
            }
        }

        if preview_lines.len() >= 6 { // Limit preview size
            break;
        }
    }

    preview_lines.join("\n")
}

/// Main search command implementation
pub async fn search_command(
    query: String,
    search_type: String,
    limit: usize,
    threshold: f32,
    embeddings_path: Option<PathBuf>,
    auto_find: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔍 Searching for: '{}'", query);
        println!("📊 Search type: {}", search_type);
        println!("📏 Limit: {}", limit);
        println!("🎯 Threshold: {}", threshold);
        println!();
    }

    let mut all_results = Vec::new();

    // Search embeddings if specified or auto-find enabled
    if let Some(emb_path) = embeddings_path {
        if verbose {
            println!("📁 Loading embeddings from: {}", emb_path.display());
        }

        match load_embeddings(&emb_path).await {
            Ok(embeddings) => {
                if verbose {
                    println!("✅ Loaded {} embeddings", embeddings.len());
                }

                match search_embeddings(&embeddings, &query, &search_type, limit, threshold).await {
                    Ok(results) => {
                        if verbose {
                            println!("🎯 Found {} results in embeddings", results.len());
                        }
                        all_results.extend(results);
                    }
                    Err(e) => {
                        if verbose {
                            println!("⚠️  Failed to search embeddings: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                if verbose {
                    println!("⚠️  Failed to load embeddings: {}", e);
                }
            }
        }
    } else if auto_find {
        if verbose {
            println!("🔎 Auto-finding embedding directories...");
        }

        match find_embedding_directories().await {
            Ok(emb_dirs) => {
                if verbose {
                    println!("📂 Found {} embedding directories", emb_dirs.len());
                    for dir in &emb_dirs {
                        println!("  • {}", dir.display());
                    }
                }

                for emb_dir in emb_dirs {
                    if verbose {
                        println!("📁 Searching in: {}", emb_dir.display());
                    }

                    match load_embeddings(&emb_dir).await {
                        Ok(embeddings) => {
                            match search_embeddings(&embeddings, &query, &search_type, limit, threshold).await {
                                Ok(results) => {
                                    if verbose {
                                        println!("  ✅ Found {} results", results.len());
                                    }
                                    all_results.extend(results);
                                }
                                Err(e) => {
                                    if verbose {
                                        println!("  ⚠️  Search failed: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if verbose {
                                println!("  ⚠️  Load failed: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if verbose {
                    println!("⚠️  Auto-find failed: {}", e);
                }
            }
        }
    }

    // Always search Helix files
    if verbose {
        println!("📄 Searching Helix configuration files...");
    }

    match search_hlx_files(&query, &search_type, limit).await {
        Ok(results) => {
            if verbose {
                println!("📋 Found {} results in Helix files", results.len());
            }
            all_results.extend(results);
        }
        Err(e) => {
            if verbose {
                println!("⚠️  Failed to search Helix files: {}", e);
            }
        }
    }

    // Sort and deduplicate results
    all_results.sort_by(|a, b| {
        b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut seen = std::collections::HashSet::new();
    let results: Vec<SearchResult> = all_results
        .into_iter()
        .filter(|r| seen.insert(r.id.clone()))
        .take(limit)
        .collect();

    // Display results
    if results.is_empty() {
        println!("❌ No results found for query: '{}'", query);
    } else {
        println!("🎯 Found {} results:", results.len());
        println!();

        for (i, result) in results.iter().enumerate() {
            println!("{}. {}", i + 1, result.id);

            if let Some(score) = result.score {
                println!("   📊 Score: {:.2}", score);
            }

            if let Some(content) = &result.content {
                println!("   📝 Content:");
                for line in content.lines().take(3) {
                    println!("     {}", line);
                }
            }

            if let Some(file_path) = result.metadata.get("file_path") {
                if let Some(path_str) = file_path.as_str() {
                    println!("   📁 Path: {}", path_str);
                }
            }

            println!();
        }
    }

    Ok(())
}
