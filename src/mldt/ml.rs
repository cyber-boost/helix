// =========  src/main.rs  ====================================================
//  Claude JSON Xtra Love – Marathon Edition (Enhanced)
//  Rust translation (≈1 200 LOC).  Uses async Tokio, Clap, Serde, Rayon, etc.
//  ---------------------------------------------------------------
//  The code is deliberately split into logical modules (inside the same file)
//  to keep the answer self‑contained while staying readable.
// ===========================================================================

#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tokio_stream::StreamExt;

// ---------------------------------------------------------------------------
// 1️⃣  CLI arguments (Python's argparse)
// ---------------------------------------------------------------------------

#[derive(Parser, Debug, Clone)]
#[command(
    name = "xtralove",
    version,
    about = "Claude JSON Xtra Love – Marathon Edition (Enhanced)",
    after_help = r#"
Marathon Mode: each item is processed through several models sequentially,
with each model building on the previous output.

EXAMPLES:
  xtralove                         # standard marathon processing
  xtralove --skip 100             # skip the first 100 items
  xtralove --resume-from-line 500 # resume from line 500
  xtralove --auto-clean 5        # auto‑clean every 5 items
  xtralove --config custom.json   # use a custom config
  xtralove --dry-run              # dry‑run, no model calls
  xtralove --stats                # show live statistics
"#
)]
struct Cli {
    #[arg(long, default_value = "config.json")]
    config: PathBuf,

    #[arg(long, default_value_t = 0)]
    skip: usize,

    #[arg(long, name = "resume-from-line", default_value_t = 0)]
    resume_from_line: usize,

    #[arg(long)]
    auto_clean: Option<usize>,

    #[arg(long, name = "quality-threshold")]
    quality_threshold: Option<f64>,

    #[arg(long, short)]
    verbose: bool,

    #[arg(long, short)]
    test: bool,

    #[arg(long, short)]
    dry_run: bool,

    #[arg(long, short)]
    stats: bool,

    #[arg(long, short = 'D')]
    no_dedup: bool,
}

// ---------------------------------------------------------------------------
// 2️⃣  Global utilities (original utils.py)
// ---------------------------------------------------------------------------

static LOG_LEVEL: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("INFO".into()));

fn configure_logging(base_dir: &Path, level: &str, json_logs: bool) {
    let mut env_filter = env_logger::Env::default().default_filter_or(level);
    if json_logs {
        // Simple JSON‑style line: {"ts":"...","lvl":"INFO","msg":"…"}
        env::set_var("RUST_LOG", level);
        env_logger::Builder::from_env(env_filter)
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{{\"ts\":\"{}\",\"lvl\":\"{}\",\"msg\":\"{}\"}}",
                    chrono::Utc::now().to_rfc3339(),
                    record.level(),
                    record.args()
                )
            })
            .init();
    } else {
        env_logger::Builder::from_env(env_filter).init();
    }
    *LOG_LEVEL.lock().unwrap() = level.into();
}

/// Simple SHA‑256 hash, optionally normalised (lower‑casing & whitespace collapsing)
fn calculate_hash(content: &str, normalize: bool) -> String {
    use sha2::{Digest, Sha256};
    let txt = if normalize {
        let re = Regex::new(r"\s+").unwrap();
        re.replace_all(&content.to_lowercase(), " ").to_string()
    } else {
        content.to_string()
    };
    let mut hasher = Sha256::new();
    hasher.update(txt.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn json_dumps<T: Serialize>(value: &T, indent: Option<usize>) -> Result<String> {
    if let Some(spaces) = indent {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
}

fn json_loads<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T> {
    Ok(serde_json::from_str(s)?)
}

// ---------------------------------------------------------------------------
// 3️⃣  Configuration handling (ConfigManager)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
struct Config {
    ollama: OllamaConfig,
    processing: ProcessingConfig,
    output: OutputConfig,
    prompt: PromptConfig,
    input: InputConfig,
    // optional sections …
    #[serde(default)]
    marathon: MarathonConfig,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    dataset: DatasetConfig,
    #[serde(default)]
    logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct OllamaConfig {
    host: String,
    #[serde(default)]
    auth_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ProcessingConfig {
    marathon_sequence: Vec<String>,
    batch_size: usize,
    checkpoint_interval: usize,
    parallel_workers: usize,
    max_retries: usize,
    retry_delay: u64,
    #[serde(default)]
    auto_clean_interval: usize,
    #[serde(default)]
    timeout_per_model: u64,
    #[serde(default)]
    pass_full_context: bool,
    #[serde(default)]
    aggregate_responses: bool,
    #[serde(default)]
    best_response: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct OutputConfig {
    base_directory: String,
    #[serde(default)]
    compression: CompressionConfig,
    #[serde(default)]
    formats: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct CompressionConfig {
    enable: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct PromptConfig {
    template_file: String,
    #[serde(default)]
    prompt_replace_one: String,
    #[serde(default)]
    prompt_replace_two: String,
    #[serde(default)]
    prompt_replace_three: String,
    #[serde(default)]
    prompt_replace_four: String,
    #[serde(default)]
    prompt_replace_five: String,
    #[serde(default)]
    dynamic_replacements: DynamicReplacements,
    #[serde(default)]
    enhancement_prompts: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct DynamicReplacements {
    enable: bool,
    per_category: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct MarathonConfig {
    #[serde(default)]
    pass_full_context: bool,
    #[serde(default)]
    aggregate_responses: bool,
    #[serde(default)]
    best_response: bool,
    #[serde(default)]
    enhancement_prompts: HashMap<String, String>,
    // stage_X_min_quality can be added via serde(flatten) if needed
    #[serde(flatten)]
    extra: HashMap<String, JsonValue>,
}

#[derive(Debug, Deserialize, Clone)]
struct InputConfig {
    source_file: String,
    processing_keys: Vec<String>,
    #[serde(default)]
    min_length: usize,
    #[serde(default)]
    max_length: usize,
    #[serde(default)]
    skip_empty: bool,
    #[serde(default)]
    test_limit: usize,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct DatasetConfig {
    #[serde(default)]
    quality_threshold: f64,
    #[serde(default)]
    split_ratios: SplitRatios,
    #[serde(default)]
    validation: ValidationConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct SplitRatios {
    #[serde(default = "default_train")]
    train: f64,
    #[serde(default = "default_test")]
    test: f64,
    #[serde(default = "default_validate")]
    validate: f64,
}
fn default_train() -> f64 { 0.8 }
fn default_test() -> f64 { 0.1 }
fn default_validate() -> f64 { 0.1 }

#[derive(Debug, Deserialize, Clone, Default)]
struct ValidationConfig {
    #[serde(default = "default_min_examples")]
    min_examples_per_response: usize,
    #[serde(default = "default_required_fields")]
    required_fields: Vec<String>,
}
fn default_min_examples() -> usize { 4 }
fn default_required_fields() -> Vec<String> {
    vec![
        "level".into(),
        "category".into(),
        "prompt".into(),
        "chosen".into(),
        "rejected".into(),
    ]
}

#[derive(Debug, Deserialize, Clone, Default)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default)]
    json_logs: bool,
}
fn default_log_level() -> String { "INFO".into() }

/// A thin wrapper that loads the config JSON once and caches it (singleton‑like)
#[derive(Clone)]
struct ConfigManager {
    config: Arc<Config>,
    prompt_template: Arc<String>,
}

impl ConfigManager {
    fn new(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Reading config file {}", path.display()))?;
        let config: Config = json_loads(&raw)?;
        // Load prompt template
        let tmpl_path = Path::new(&config.prompt.template_file);
        let tmpl = fs::read_to_string(tmpl_path).with_context(|| {
            format!("Reading prompt template {}", tmpl_path.display())
        })?;
        Ok(Self {
            config: Arc::new(config),
            prompt_template: Arc::new(tmpl),
        })
    }

    /// Dynamically generate a prompt string based on the template and replacements
    fn render_prompt(&self, content: &str, category: Option<&str>) -> String {
        // base replacements
        let mut repl = HashMap::new();
        repl.insert("replace-by-key".to_string(), content.to_string());
        repl.insert(
            "prompt_replace_one".to_string(),
            self.config.prompt.prompt_replace_one.clone(),
        );
        repl.insert(
            "prompt_replace_two".to_string(),
            self.config.prompt.prompt_replace_two.clone(),
        );
        repl.insert(
            "prompt_replace_three".to_string(),
            self.config.prompt.prompt_replace_three.clone(),
        );
        repl.insert(
            "prompt_replace_four".to_string(),
            self.config.prompt.prompt_replace_four.clone(),
        );
        repl.insert(
            "prompt_replace_five".to_string(),
            self.config.prompt.prompt_replace_five.clone(),
        );

        // category‑specific overrides
        if let Some(cat) = category {
            if self
                .config
                .prompt
                .dynamic_replacements
                .enable
            {
                if let Some(cat_map) = self
                    .config
                    .prompt
                    .dynamic_replacements
                    .per_category
                    .get(cat)
                {
                    for (k, v) in cat_map {
                        repl.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        // Very small template engine: {{key}}
        let mut out = self.prompt_template.as_str().to_string();
        for (k, v) in repl {
            let placeholder = format!("{{{{{}}}}}", k);
            out = out.replace(&placeholder, &v);
        }
        out
    }

    /// Prompt used for later marathon stages (adds previous response + enhancement)
    fn marathon_prompt(
        &self,
        content: &str,
        stage: usize,
        previous: Option<&JsonValue>,
    ) -> String {
        let base = self.render_prompt(content, None);
        if stage == 1 || previous.is_none() {
            return base;
        }
        let enh_key = format!("stage_{stage}");
        let enh = self
            .config
            .marathon
            .enhancement_prompts
            .get(&enh_key)
            .cloned()
            .unwrap_or_else(|| {
                "Improve upon the previous response with more depth and quality.".into()
            });
        let prev_json = json_dumps(previous.unwrap(), Some(2)).unwrap_or_default();
        format!(
            "{base}\n\n=== MARATHON MODE STAGE {stage} ===\nPrevious model's response to enhance:\n{prev_json}\n\nEnhancement Instructions: {enh}\n\nProvide an improved version that builds upon the previous response."
        )
    }
}

// ---------------------------------------------------------------------------
// 4️⃣  Core data structures (ProcessingItem, MarathonResponse, …)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProcessingItem {
    id: String,
    content: String,
    category: String,
    line_number: usize,
    source_key: String,
    #[serde(default)]
    processed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MarathonResponse {
    stage: usize,
    model: String,
    response: JsonValue,
    timestamp: String,
    #[serde(default)]
    quality_score: f64,
    #[serde(default)]
    enhancement: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MarathonResult {
    item: ProcessingItem,
    stages: Vec<MarathonResponse>,
    final_response: JsonValue,
    total_time: f64,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    hash_id: Option<String>,
}

impl MarathonResult {
    fn new(item: ProcessingItem, stages: Vec<MarathonResponse>, final_resp: JsonValue, total: f64) -> Self {
        let hash = {
            let src = format!(
                "{}_{}_{}",
                item.content, item.category, stages.len()
            );
            calculate_hash(&src, true)
        };
        let success = !stages.is_empty();
        Self {
            item,
            stages,
            final_response: final_resp,
            total_time: total,
            success,
            hash_id: Some(hash),
        }
    }
}

// ---------------------------------------------------------------------------
// 5️⃣  Persistent deduplication store (very small SQLite wrapper)
// ---------------------------------------------------------------------------

mod dedup {
    use super::*;
    use rusqlite::{params, Connection, Result as SqlResult};

    pub struct DedupStore {
        conn: Mutex<Connection>,
    }

    impl DedupStore {
        pub fn new(path: impl AsRef<Path>) -> Self {
            let conn = Connection::open(path).expect("Failed to open dedup DB");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS dedup (
                    hash TEXT PRIMARY KEY,
                    category TEXT,
                    meta TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_category ON dedup(category);",
            )
            .expect("Failed to init dedup schema");
            Self {
                conn: Mutex::new(conn),
            }
        }

        pub fn exists(&self, hash: &str) -> bool {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT 1 FROM dedup WHERE hash = ?1")
                .unwrap();
            stmt.exists(params![hash]).unwrap_or(false)
        }

        pub fn add(&self, hash: &str, category: &str, meta: impl Serialize) {
            let meta_json = json_dumps(&meta, None).unwrap();
            let conn = self.conn.lock().unwrap();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO dedup (hash, category, meta) VALUES (?1, ?2, ?3)",
                params![hash, category, meta_json],
            );
        }

        pub fn stats(&self) -> HashMap<String, usize> {
            let conn = self.conn.lock().unwrap();
            let mut map = HashMap::new();
            let mut stmt = conn
                .prepare("SELECT category, COUNT(*) FROM dedup GROUP BY category")
                .unwrap();
            let rows = stmt
                .query_map([], |row| {
                    let cat: String = row.get(0)?;
                    let cnt: usize = row.get(1)?;
                    Ok((cat, cnt))
                })
                .unwrap();
            for r in rows {
                if let Ok((cat, cnt)) = r {
                    map.insert(cat, cnt);
                }
            }
            map
        }

        pub fn close(self) {
            // Drop connection automatically.
        }
    }
}

// ---------------------------------------------------------------------------
// 6️⃣  Helper I/O (compression, batch writer, JSON‑stream loader)
// ---------------------------------------------------------------------------

fn save_with_compression<T: Serialize>(data: &T, path: impl AsRef<Path>, compress: bool) -> Result<()> {
    let raw = json_dumps(data, Some(2))?;
    if compress {
        // Simple gzip wrapper (flate2)
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let file = File::create(&path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(raw.as_bytes())?;
        encoder.finish()?;
    } else {
        let mut file = File::create(&path)?;
        file.write_all(raw.as_bytes())?;
    }
    Ok(())
}

/// Load a huge JSON file line‑by‑line (each line may be a JSON object)
fn load_json_stream(path: impl AsRef<Path>) -> impl Iterator<Item = JsonValue> {
    let file = File::open(path).expect("Failed to open source file");
    let br = BufReader::new(file);
    br.lines()
        .filter_map(|ln| ln.ok())
        .filter_map(|ln| json_loads::<JsonValue>(&ln).ok())
}

// Very small batch writer (used for JSONL export)
struct BatchWriter {
    file: Mutex<File>,
    batch_size: usize,
    buffer: Mutex<Vec<JsonValue>>,
    fmt: String, // "jsonl" or "json"
}

impl BatchWriter {
    fn new(path: impl AsRef<Path>, batch_size: usize, fmt: &str) -> Result<Self> {
        let f = File::create(path)?;
        Ok(Self {
            file: Mutex::new(f),
            batch_size,
            buffer: Mutex::new(Vec::with_capacity(batch_size)),
            fmt: fmt.to_string(),
        })
    }

    fn add(&self, item: impl Serialize) -> Result<()> {
        let json = serde_json::to_value(item)?;
        let mut buf = self.buffer.lock().unwrap();
        buf.push(json);
        if buf.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        let mut buf = self.buffer.lock().unwrap();
        if buf.is_empty() {
            return Ok(());
        }
        let mut file = self.file.lock().unwrap();
        match self.fmt.as_str() {
            "jsonl" => {
                for v in buf.drain(..) {
                    writeln!(file, "{}", json_dumps(&v, None)?)?;
                }
            }
            "json" => {
                // Write a single JSON array for the whole batch
                let arr = JsonValue::Array(buf.clone());
                writeln!(file, "{}", json_dumps(&arr, Some(2))?)?;
                buf.clear();
            }
            _ => {}
        }
        Ok(())
    }
}

impl Drop for BatchWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ---------------------------------------------------------------------------
// 7️⃣  Ollama client wrapper (streaming chat)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct OllamaClient {
    http: HttpClient,
    host: String,
    default_headers: HeaderMap,
}

impl OllamaClient {
    fn new(host: &str, token: Option<String>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        if let Some(tok) = token {
            let val = HeaderValue::from_str(&format!("Bearer {tok}")).unwrap();
            headers.insert(AUTHORIZATION, val);
        }
        let client = HttpClient::builder()
            .default_headers(headers.clone())
            .build()?;
        Ok(Self {
            http: client,
            host: host.trim_end_matches('/').to_string(),
            default_headers: headers,
        })
    }

    /// Call Ollama's `/api/chat` endpoint with streaming turned on.
    async fn chat_stream(
        &self,
        model: &str,
        prompt: &str,
    ) -> Result<impl futures::Stream<Item = Result<String, reqwest::Error>>> {
        #[derive(Serialize)]
        struct ReqBody<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            stream: bool,
        }
        #[derive(Serialize)]
        struct Message<'a> {
            role: &'a str,
            content: &'a str,
        }

        let url = format!("{}/api/chat", self.host);
        let body = ReqBody {
            model,
            messages: vec![Message {
                role: "user",
                content: prompt,
            }],
            stream: true,
        };
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .bytes_stream()
            .map(|b| b.map(|bytes| String::from_utf8_lossy(&bytes).to_string()));
        Ok(resp)
    }
}

// ---------------------------------------------------------------------------
// 8️⃣  MarathonProcessor – core business logic
// ---------------------------------------------------------------------------

struct MarathonProcessor {
    cfg: Arc<Config>,
    client: OllamaClient,
    max_retries: usize,
    retry_delay: Duration,
    logger: log::Logger,
}

impl MarathonProcessor {
    fn new(cfg: Arc<Config>, client: OllamaClient) -> Self {
        let logger = log::logger();
        Self {
            cfg,
            client,
            max_retries: cfg.processing.max_retries,
            retry_delay: Duration::from_secs(cfg.processing.retry_delay),
            logger,
        }
    }

    /// Public entry point – called from the orchestrator
    async fn process_item(
        &self,
        item: ProcessingItem,
        cfg_mgr: &ConfigManager,
    ) -> Result<MarathonResult> {
        let start = Instant::now();
        let mut stages = Vec::new();
        let mut previous_response: Option<JsonValue> = None;

        for (idx, model) in self.cfg.processing.marathon_sequence.iter().enumerate() {
            let stage_no = idx + 1;
            // Build prompt
            let prompt = if stage_no == 1 {
                cfg_mgr.render_prompt(&item.content, Some(&item.category))
            } else {
                cfg_mgr.marathon_prompt(&item.content, stage_no, previous_response.as_ref())
            };

            // Model call with retry
            let mut attempt = 0;
            let mut raw_response: Option<JsonValue> = None;
            while attempt < self.max_retries {
                match self.call_model(model, &prompt).await {
                    Ok(resp) => {
                        raw_response = Some(resp);
                        break;
                    }
                    Err(e) => {
                        attempt += 1;
                        log::warn!(
                            "Stage {} attempt {} failed for {}: {}",
                            stage_no,
                            attempt,
                            model,
                            e
                        );
                        if attempt < self.max_retries {
                            tokio::time::sleep(self.retry_delay * attempt as u32).await;
                        }
                    }
                }
            }

            let response = match raw_response {
                Some(r) => r,
                None => {
                    log::error!("Stage {} failed after {} retries", stage_no, self.max_retries);
                    continue; // go to next model
                }
            };

            // Quality scoring
            let quality = self.calculate_quality(&response);

            // Optional fast‑fail based on config
            let min_q_key = format!("stage_{stage_no}_min_quality");
            let min_quality = self
                .cfg
                .marathon
                .extra
                .get(&min_q_key)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if quality < min_quality && stage_no < self.cfg.processing.marathon_sequence.len() {
                log::info!(
                    "Stage {stage_no} quality {quality:.1} < {min_quality}, skipping to next model"
                );
                continue;
            }

            // Record stage
            let stage = MarathonResponse {
                stage: stage_no,
                model: model.clone(),
                response: response.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                quality_score: quality,
                enhancement: format!("Stage {stage_no} enhancement"),
            };
            previous_response = Some(response.clone());
            stages.push(stage);
        }

        // Final aggregation
        let final_resp = if self.cfg.marathon.aggregate_responses {
            self.aggregate_responses(&stages)
        } else if self.cfg.marathon.best_response {
            stages
                .iter()
                .max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap())
                .map(|s| s.response.clone())
                .unwrap_or_else(|| json!({}))
        } else {
            stages.last().map(|s| s.response.clone()).unwrap_or_else(|| json!({}))
        };

        let total_secs = start.elapsed().as_secs_f64();
        Ok(MarathonResult::new(item, stages, final_resp, total_secs))
    }

    async fn call_model(&self, model: &str, prompt: &str) -> Result<JsonValue> {
        let mut full = String::new();
        let mut stream = self.client.chat_stream(model, prompt).await?;
        while let Some(chunk) = stream.next().await {
            let txt = chunk?;
            // Ollama returns raw text fragments; we just concatenate.
            full.push_str(&txt);
        }

        // Strip markdown fences if present
        let cleaned = if let Some(start) = full.find("```json") {
            let after = start + 7;
            let end = full[after..].find("```").map(|e| after + e).unwrap_or(full.len());
            full[after..end].trim()
        } else if let Some(start) = full.find("```") {
            let after = start + 3;
            let end = full[after..].find("```").map(|e| after + e).unwrap_or(full.len());
            full[after..end].trim()
        } else {
            full.trim()
        };

        // Parse JSON – if it fails we keep the raw response under a special key.
        match json_loads::<JsonValue>(cleaned) {
            Ok(j) => Ok(j),
            Err(_) => Ok(json!({
                "raw_response": cleaned,
                "parse_error": true
            })),
        }
    }

    fn calculate_quality(&self, resp: &JsonValue) -> f64 {
        // Mirrors the Python scoring logic (simplified a bit)
        if resp.get("parse_error").and_then(|v| v.as_bool()) == Some(true) {
            return 0.0;
        }
        let mut score = 0.0;
        if let Some(arr) = resp.get("training_examples").and_then(|v| v.as_array()) {
            score += 20.0; // presence
            let cnt = arr.len();
            if cnt >= 4 {
                score += 10.0;
            }
            if cnt >= 8 {
                score += 10.0;
            }
            if cnt >= 16 {
                score += 10.0;
            }

            // Field & content checks (first 10 examples)
            let mut field_score = 0.0;
            for ex in arr.iter().take(10) {
                let has_req = ["level", "category", "prompt", "chosen", "rejected"]
                    .iter()
                    .all(|k| ex.get(*k).map(|v| !v.is_null()).unwrap_or(false));
                if has_req {
                    field_score += 3.0;
                    if let Some(p) = ex.get("prompt").and_then(|v| v.as_str()) {
                        if p.len() > 20 {
                            field_score += 1.0;
                        }
                    }
                    if let Some(c) = ex.get("chosen").and_then(|v| v.as_str()) {
                        if c.len() > 30 {
                            field_score += 1.0;
                        }
                    }
                    if let Some(r) = ex.get("rejected").and_then(|v| v.as_str()) {
                        if r.len() > 30 {
                            field_score += 1.0;
                        }
                    }
                    // optional fields bonus
                    if ex.get("explanation").is_some() || ex.get("tags").is_some() {
                        field_score += 1.0;
                    }
                }
            }
            score += field_score.min(30.0);
            // Diversity
            let uniq: HashSet<_> = arr
                .iter()
                .filter_map(|e| e.get("prompt"))
                .map(|v| v.as_str().unwrap_or("").to_lowercase())
                .collect();
            let diversity = (uniq.len() as f64 / cnt as f64) * 10.0;
            score += diversity.min(10.0);
        }
        score.min(100.0)
    }

    fn aggregate_responses(&self, stages: &[MarathonResponse]) -> JsonValue {
        let mut agg = json!({
            "metadata": {
                "stages_completed": stages.len(),
                "models_used": stages.iter().map(|s| s.model.clone()).collect::<Vec<_>>(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            },
            "training_examples": []
        });

        let mut seen = HashSet::new();
        for stage in stages {
            if let Some(arr) = stage.response.get("training_examples").and_then(|v| v.as_array()) {
                for ex in arr {
                    let prompt = ex
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let norm = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
                    let h = calculate_hash(&norm, true);
                    if seen.insert(h) {
                        // Clone the example and enrich it
                        let mut enriched = ex.clone();
                        if let Some(obj) = enriched.as_object_mut() {
                            obj.insert(
                                "stage_origin".to_string(),
                                json!(stage.stage),
                            );
                            obj.insert(
                                "model_origin".to_string(),
                                json!(stage.model.clone()),
                            );
                            obj.insert(
                                "quality_score".to_string(),
                                json!(stage.quality_score),
                            );
                        }
                        if let Some(te) = agg.get_mut("training_examples") {
                            if let Some(arr_mut) = te.as_array_mut() {
                                arr_mut.push(enriched);
                            }
                        }
                    }
                }
            }
        }

        // Sort by quality
        if let Some(te) = agg.get_mut("training_examples") {
            if let Some(arr) = te.as_array_mut() {
                arr.sort_by(|a, b| {
                    let qa = a
                        .get("quality_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let qb = b
                        .get("quality_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    qb.partial_cmp(&qa).unwrap()
                });
            }
        }

        // Statistics
        let stats = json!({
            "total_examples": agg["training_examples"].as_array().map(|a| a.len()).unwrap_or(0),
            "average_quality": stages.iter().map(|s| s.quality_score).sum::<f64>() / stages.len() as f64,
            "best_quality": stages.iter().map(|s| s.quality_score).fold(0.0f64, f64::max),
            "unique_examples": seen.len()
        });
        agg.as_object_mut()
            .unwrap()
            .insert("statistics".to_string(), stats);
        agg
    }
}

// ---------------------------------------------------------------------------
// 9️⃣  DataManager – filesystem layout, dedup, batch writes, auto‑clean
// ---------------------------------------------------------------------------

struct DataManager {
    cfg: Arc<Config>,
    base_dir: PathBuf,
    dedup: Arc<dedup::DedupStore>,
    // counters
    batch_counter: AtomicUsize,
    auto_clean_counter: AtomicUsize,
    // in‑memory collection of results (for final export & stats)
    results: Mutex<Vec<Arc<MarathonResult>>>,
    logger: log::Logger,
}

impl DataManager {
    fn new(cfg: Arc<Config>) -> Result<Self> {
        let base_dir = PathBuf::from(&cfg.output.base_directory);
        fs::create_dir_all(&base_dir)?;
        // create sub‑dirs
        let subdirs = [
            "marathon",
            "checkpoints",
            "datasets",
            "cleaned",
            "analytics",
        ];
        for s in subdirs.iter() {
            fs::create_dir_all(base_dir.join(s))?;
        }
        // stage dirs
        for i in 1..=5 {
            fs::create_dir_all(base_dir.join("marathon").join(format!("stage_{i}")))?;
        }
        // category dirs
        for cat in cfg.categories.iter() {
            fs::create_dir_all(base_dir.join("by_category").join(cat))?;
        }

        let dedup_path = base_dir.join("dedup.db");
        let dedup = Arc::new(dedup::DedupStore::new(dedup_path));

        configure_logging(
            &base_dir,
            cfg.logging.level.as_str(),
            cfg.logging.json_logs,
        );

        Ok(Self {
            cfg,
            base_dir,
            dedup,
            batch_counter: AtomicUsize::new(0),
            auto_clean_counter: AtomicUsize::new(0),
            results: Mutex::new(Vec::new()),
            logger: log::logger(),
        })
    }

    fn save_result(&self, result: Arc<MarathonResult>) -> Result<bool> {
        // dedup check
        let hash = result
            .hash_id
            .as_ref()
            .ok_or_else(|| anyhow!("Result missing hash_id"))?;
        if self.dedup.exists(hash) {
            log::debug!("Duplicate result {}", hash);
            return Ok(false);
        }

        // record in dedup
        self.dedup.add(
            hash,
            &result.item.category,
            json!({
                "item_id": result.item.id,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        );

        // write each stage
        for (i, stage) in result.stages.iter().enumerate() {
            let stage_dir = self.base_dir.join("marathon").join(format!("stage_{}", i + 1));
            let fname = format!(
                "{}_{}_{}.json",
                result.item.category, result.item.line_number, hash
            );
            let file_path = stage_dir.join(fname);
            let payload = json!({
                "item": result.item,
                "stage": stage,
                "hash_id": hash
            });
            save_with_compression(
                &payload,
                file_path,
                self.cfg.output.compression.enable,
            )?;
        }

        // aggregate file
        let agg_path = self.base_dir.join("marathon").join(format!("complete_{}.json", hash));
        save_with_compression(&*result, agg_path, self.cfg.output.compression.enable)?;

        // category directory
        let cat_dir = self.base_dir.join("by_category").join(&result.item.category);
        let cat_path = cat_dir.join(format!("{}_{}.json", result.item.line_number, hash));
        save_with_compression(&*result, cat_path, self.cfg.output.compression.enable)?;

        // keep in memory for final export / stats
        self.results.lock().unwrap().push(result.clone());

        // counters
        self.batch_counter.fetch_add(1, Ordering::SeqCst);
        self.auto_clean_counter.fetch_add(1, Ordering::SeqCst);

        // auto‑clean trigger
        let auto_int = self.cfg.processing.auto_clean_interval;
        if auto_int > 0 && self.auto_clean_counter.load(Ordering::SeqCst) >= auto_int {
            self.auto_clean(None)?;
            self.auto_clean_counter.store(0, Ordering::SeqCst);
        }

        Ok(true)
    }

    fn auto_clean(&self, quality_threshold: Option<f64>) -> Result<()> {
        let thresh = quality_threshold.unwrap_or_else(|| self.cfg.dataset.quality_threshold);
        let mut cleaned = Vec::new();
        let mut reasons: HashMap<String, usize> = HashMap::new();

        let results_snapshot = {
            let guard = self.results.lock().unwrap();
            guard.clone()
        };

        for r in results_snapshot.iter().rev().take(self.auto_clean_counter.load(Ordering::SeqCst)) {
            // duplicate guard (should be impossible)
            if self.dedup.exists(r.hash_id.as_ref().unwrap()) {
                *reasons.entry("duplicate".into()).or_default() += 1;
                continue;
            }
            if !r.success {
                *reasons.entry("failed".into()).or_default() += 1;
                continue;
            }
            if r.final_response.is_null() {
                *reasons.entry("no_response".into()).or_default() += 1;
                continue;
            }
            let avg_q: f64 = if r.stages.is_empty() {
                0.0
            } else {
                r.stages.iter().map(|s| s.quality_score).sum::<f64>() / r.stages.len() as f64
            };
            if avg_q < thresh {
                *reasons
                    .entry(format!("below_quality_{:.0}", thresh))
                    .or_default() += 1;
                continue;
            }
            if self.validate_response(&r.final_response) {
                cleaned.push(r.clone());
            } else {
                *reasons.entry("invalid_structure".into()).or_default() += 1;
            }
        }

        if !cleaned.is_empty() {
            let batch_id = self.batch_counter.load(Ordering::SeqCst);
            let out_path = self
                .base_dir
                .join("cleaned")
                .join(format!("batch_{:06}.json", batch_id));
            let payload = json!({
                "batch": batch_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "count": cleaned.len(),
                "quality_threshold": thresh,
                "data": cleaned
            });
            save_with_compression(&payload, out_path, self.cfg.output.compression.enable)?;
            log::info!("Cleaned {} results (threshold {})", cleaned.len(), thresh);
        }

        if !reasons.is_empty() {
            log::info!("Cleaning reasons: {:?}", reasons);
        }
        Ok(())
    }

    fn validate_response(&self, resp: &JsonValue) -> bool {
        if let Some(arr) = resp.get("training_examples").and_then(|v| v.as_array()) {
            let min_ex = self.cfg.dataset.validation.min_examples_per_response;
            if arr.len() < min_ex {
                return false;
            }
            let required = &self.cfg.dataset.validation.required_fields;
            for ex in arr.iter() {
                if !required.iter().all(|k| ex.get(k).is_some()) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn save_checkpoint(&self, last_line: usize) -> Result<()> {
        let ckpt_id = self.checkpoint_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let path = self.base_dir.join("checkpoints").join(format!("checkpoint_{:04}.json", ckpt_id));
        let data = json!({
            "checkpoint": ckpt_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "total_processed": self.results.lock().unwrap().len(),
            "batch_counter": self.batch_counter.load(Ordering::SeqCst),
            "last_processed_line": last_line,
            "dedup_stats": self.dedup.stats(),
            "state": {
                "successful": self.results.lock().unwrap().iter().filter(|r| r.success).count(),
                "failed": self.results.lock().unwrap().iter().filter(|r| !r.success).count(),
                "categories": self
                    .results
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|r| r.item.category.clone())
                    .collect::<HashSet<_>>()
            }
        });
        save_with_compression(&data, path, false)?;
        log::info!("Checkpoint {} saved (line {})", ckpt_id, last_line);
        Ok(())
    }

    fn final_export(&self) -> Result<()> {
        log::info!("Creating final exports…");
        let all = self.results.lock().unwrap();
        let successful: Vec<_> = all.iter().filter(|r| r.success).cloned().collect();

        if successful.is_empty() {
            log::warn!("No successful results to export");
            return Ok(());
        }

        // split
        let mut rng = rand::thread_rng();
        let mut shuffled = successful.clone();
        shuffled.shuffle(&mut rng);
        let total = shuffled.len() as f64;
        let split = &self.cfg.dataset.split_ratios;
        let train_sz = (total * split.train).round() as usize;
        let test_sz = (total * split.test).round() as usize;
        let train = &shuffled[..train_sz];
        let test = &shuffled[train_sz..train_sz + test_sz];
        let validate = &shuffled[train_sz + test_sz..];

        for (name, slice) in &[("train", train), ("test", test), ("validate", validate)] {
            if slice.is_empty() {
                log::warn!("No data for {} split", name);
                continue;
            }
            // JSON
            let json_path = self.base_dir.join("datasets").join(format!("{name}.json"));
            let payload = json!({
                "split": name,
                "count": slice.len(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "data": slice
            });
            save_with_compression(
                &payload,
                json_path,
                self.cfg.output.compression.enable,
            )?;

            // JSONL (optional)
            if self.cfg.output.formats.iter().any(|f| f == "jsonl") {
                let jsonl_path = self.base_dir.join("datasets").join(format!("{name}.jsonl"));
                let writer = BatchWriter::new(jsonl_path, 100, "jsonl")?;
                for r in *slice {
                    writer.add(&r)?;
                }
                // Drop writer → flush
            }
            log::info!("Exported {} ({}) samples", name, slice.len());
        }

        // summary report
        let report_path = self.base_dir.join("analytics").join("summary_report.json");
        let mut by_cat: HashMap<String, usize> = HashMap::new();
        let mut by_stage: HashMap<String, usize> = HashMap::new();
        let mut quality_scores = Vec::new();
        let mut processing_times = Vec::new();

        for r in all.iter() {
            *by_cat.entry(r.item.category.clone()).or_default() += 1;
            processing_times.push(r.total_time);
            for s in &r.stages {
                *by_stage
                    .entry(format!("stage_{}", s.stage))
                    .or_default() += 1;
                if s.quality_score > 0.0 {
                    quality_scores.push(s.quality_score);
                }
            }
        }

        let report = json!({
            "generated": chrono::Utc::now().to_rfc3339(),
            "created_with": "💝 xtra love (Rust rewrite)",
            "statistics": {
                "total_processed": all.len(),
                "successful": all.iter().filter(|r| r.success).count(),
                "failed": all.iter().filter(|r| !r.success).count(),
                "success_rate": (all.iter().filter(|r| r.success).count() as f64
                                 / all.len().max(1) as f64) * 100.0
            },
            "dedup_stats": self.dedup.stats(),
            "by_category": by_cat,
            "by_stage": by_stage,
            "quality_scores": quality_scores,
            "processing_times": processing_times
        });
        save_with_compression(&report, report_path, false)?;
        log::info!("Summary report saved");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 10️⃣  Orchestrator – coordinate everything, handle signals, stats, etc.
// ---------------------------------------------------------------------------

struct Orchestrator {
    args: Cli,
    cfg_mgr: ConfigManager,
    data_mgr: DataManager,
    processor: MarathonProcessor,
    // runtime control
    last_line: AtomicUsize,
    processed_set: DashMap<String, ()>,
}

impl Orchestrator {
    async fn new(args: Cli) -> Result<Self> {
        let cfg_mgr = ConfigManager::new(&args.config)?;
        let cfg_arc = cfg_mgr.config.clone();
        let data_mgr = DataManager::new(cfg_arc.clone())?;
        let client = OllamaClient::new(
            &cfg_arc.ollama.host,
            cfg_arc.ollama.auth_token.clone(),
        )?;
        let processor = MarathonProcessor::new(cfg_arc.clone(), client);
        Ok(Self {
            args,
            cfg_mgr,
            data_mgr,
            processor,
            last_line: AtomicUsize::new(0),
            processed_set: DashMap::new(),
        })
    }

    /// Load items from the source file (stream‑wise) applying skip / resume / test limits.
    fn load_items(&self) -> Result<Vec<ProcessingItem>> {
        let src_path = Path::new(&self.cfg_mgr.config.input.source_file);
        if !src_path.exists() {
            return Err(anyhow!("Source file not found: {}", src_path.display()));
        }

        let mut items = Vec::new();
        let proc_keys = &self.cfg_mgr.config.input.processing_keys;
        let min_len = self.cfg_mgr.config.input.min_length;
        let max_len = self.cfg_mgr.config.input.max_length;
        let skip_empty = self.cfg_mgr.config.input.skip_empty;

        for (idx, json_val) in load_json_stream(src_path).enumerate() {
            // For each top‑level key we care about
            for key in proc_keys {
                if let Some(val) = json_val.get(key) {
                    self.extract_items(
                        val,
                        key,
                        &mut items,
                        idx,
                        min_len,
                        max_len,
                        skip_empty,
                    );
                }
            }

            // Test mode early exit
            if self.args.test && items.len() >= 5 {
                break;
            }
        }

        // Apply `--skip` and `--resume-from-line`
        let mut start = 0usize;
        if self.args.resume_from_line > 0 {
            start = self.args.resume_from_line;
        }
        if self.args.skip > 0 {
            start = start.saturating_add(self.args.skip);
        }
        let final_items = if start < items.len() {
            items[start..].to_vec()
        } else {
            Vec::new()
        };
        log::info!("Loaded {} items to process", final_items.len());
        Ok(final_items)
    }

    fn extract_items(
        &self,
        data: &JsonValue,
        key: &str,
        out: &mut Vec<ProcessingItem>,
        line_no: usize,
        min_len: usize,
        max_len: usize,
        skip_empty: bool,
    ) {
        match data {
            JsonValue::Array(arr) => {
                for (i, it) in arr.iter().enumerate() {
                    let content = it.to_string().trim().to_string();
                    if (skip_empty && content.is_empty())
                        || content.len() < min_len
                        || content.len() > max_len
                    {
                        continue;
                    }
                    let category = self.determine_category(&content);
                    out.push(ProcessingItem {
                        id: format!("{key}_{:06}", out.len()),
                        content,
                        category,
                        line_number: line_no + i,
                        source_key: key.to_string(),
                        processed: false,
                        skip_reason: None,
                    });
                }
            }
            JsonValue::Object(map) => {
                for (cat, val) in map.iter() {
                    if let JsonValue::Array(arr) = val {
                        for (i, it) in arr.iter().enumerate() {
                            let content = it.to_string().trim().to_string();
                            if (skip_empty && content.is_empty())
                                || content.len() < min_len
                                || content.len() > max_len
                            {
                                continue;
                            }
                            out.push(ProcessingItem {
                                id: format!("{key}_{cat}_{:06}", out.len()),
                                content,
                                category: cat.clone(),
                                line_number: line_no + i,
                                source_key: key.to_string(),
                                processed: false,
                                skip_reason: None,
                            });
                        }
                    }
                }
            }
            _ => {
                // plain value
                let content = data.to_string().trim().to_string();
                if !(skip_empty && content.is_empty())
                    && content.len() >= min_len
                    && content.len() <= max_len
                {
                    let category = self.determine_category(&content);
                    out.push(ProcessingItem {
                        id: format!("direct_{:06}", out.len()),
                        content,
                        category,
                        line_number: line_no,
                        source_key: "direct".into(),
                        processed: false,
                        skip_reason: None,
                    });
                }
            }
        }
    }

    fn determine_category(&self, content: &str) -> String {
        let lc = content.to_lowercase();
        // explicit categories from config first
        for cat in &self.cfg_mgr.config.categories {
            if lc.contains(&cat.to_lowercase()) {
                return cat.clone();
            }
        }
        // fallback heuristics
        let heuristics: &[(&str, &[&str])] = &[
            ("personal", &["personal", "self", "individual", "private"]),
            ("business", &["business", "company", "corporate", "work"]),
            ("development", &["code", "develop", "programming", "software"]),
            ("health", &["health", "medical", "wellness", "fitness"]),
            ("education", &["learn", "study", "teach", "school"]),
            ("technology", &["tech", "digital", "computer", "internet"]),
        ];
        for (cat, words) in heuristics {
            if words.iter().any(|w| lc.contains(*w)) {
                return (*cat).into();
            }
        }
        "general".into()
    }

    async fn run(&self) -> Result<()> {
        // Load items
        let items = self.load_items()?;
        if items.is_empty() {
            log::warn!("No items to process – exiting");
            return Ok(());
        }

        // Dry‑run handling
        if self.args.dry_run {
            log::info!("DRY‑RUN – would process {} items", items.len());
            for (i, it) in items.iter().take(10).enumerate() {
                log::info!("  [{i}] {} – {}", it.id, &it.content[..std::cmp::min(50, it.content.len())]);
            }
            return Ok(());
        }

        // Set up a semaphore to honor `parallel_workers`
        let semaphore = Arc::new(Semaphore::new(
            self.cfg_mgr.config.processing.parallel_workers,
        ));

        // Stats
        let total = items.len();
        let mut processed = 0usize;
        let mut successes = 0usize;
        let mut failures = 0usize;
        let start_time = Instant::now();

        // Use a thread pool from rayon for simplicity (the actual model calls are async)
        let items_arc = Arc::new(items);
        items_arc.par_iter().enumerate().for_each(|(idx, item)| {
            let permit = semaphore.clone().acquire_owned();
            // we block here – the async call is then executed inside a Tokio runtime
            let _permit = futures::executor::block_on(permit);
            let item_clone = item.clone();
            let orchestrator = self.clone();

            let fut = async move {
                let res = orchestrator
                    .processor
                    .process_item(item_clone.clone(), &orchestrator.cfg_mgr)
                    .await;
                match res {
                    Ok(mr) => {
                        let saved = orchestrator
                            .data_mgr
                            .save_result(Arc::new(mr.clone()))
                            .unwrap_or(false);
                        if saved {
                            orchestrator
                                .processed_set
                                .insert(item_clone.id.clone(), ());
                            orchestrator.last_line.store(item_clone.line_number, Ordering::SeqCst);
                            if mr.success {
                                successes += 1;
                                log::info!("✅ {} success", mr.item.id);
                            } else {
                                failures += 1;
                                log::warn!("⚠️ {} failed", mr.item.id);
                            }
                        }
                    }
                    Err(e) => {
                        failures += 1;
                        log::error!("❌ {} processing error: {}", item_clone.id, e);
                    }
                }
                processed += 1;

                // Live stats (if requested)
                if orchestrator.args.stats || orchestrator.args.verbose {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = processed as f64 / elapsed;
                    let eta = if rate > 0.0 {
                        (total - processed) as f64 / rate
                    } else {
                        0.0
                    };
                    log::info!(
                        "Live: {processed}/{total} ({:.1}%) | ok:{successes} err:{failures} | {:.2} it/s | ETA {:.1}m",
                        processed as f64 / total as f64 * 100.0,
                        rate,
                        eta / 60.0
                    );
                }

                // Checkpoint every `checkpoint_interval` items
                if orchestrator.cfg_mgr.config.processing.checkpoint_interval > 0
                    && processed % orchestrator.cfg_mgr.config.processing.checkpoint_interval == 0
                {
                    let _ = orchestrator
                        .data_mgr
                        .save_checkpoint(orchestrator.last_line.load(Ordering::SeqCst));
                }
            };
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(fut);
        });

        // final export / cleanup
        self.data_mgr.final_export()?;
        log::info!(
            "Finished – processed {} items ({} ok, {} err) in {:.2} min ({:.2} it/s)",
            processed,
            successes,
            failures,
            start_time.elapsed().as_secs_f64() / 60.0,
            processed as f64 / start_time.elapsed().as_secs_f64()
        );
        Ok(())
    }
}

// Implement Clone for Orchestrator so that the rayon closure can capture it.
impl Clone for Orchestrator {
    fn clone(&self) -> Self {
        Self {
            args: self.args.clone(),
            cfg_mgr: self.cfg_mgr.clone(),
            data_mgr: self.data_mgr.clone(),
            processor: self.processor.clone(),
            last_line: AtomicUsize::new(self.last_line.load(Ordering::SeqCst)),
            processed_set: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// 11️⃣  Main entry point (mirrors the Python `if __name__ == "__main__"`)
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI
    let args = Cli::parse();

    // Adjust log level if verbose
    if args.verbose {
        env::set_var("RUST_LOG", "debug");
    }

    // Override auto‑clean interval on‑the‑fly (mirrors the Python temp‑config trick)
    if let Some(new_int) = args.auto_clean {
        // Load, modify, write temporary config – then point ConfigManager at it.
        let mut cfg_json: JsonValue = {
            let raw = fs::read_to_string(&args.config)?;
            json_loads(&raw)?
        };
        if let Some(proc) = cfg_json.get_mut("processing").and_then(|v| v.as_object_mut()) {
            proc.insert(
                "auto_clean_interval".into(),
                json!(new_int),
            );
        }
        let tmp_path = args.config.with_extension("tmp.json");
        fs::write(&tmp_path, json_dumps(&cfg_json, Some(2))?)?;
        // Swap path for the rest of the program
        let manager = ConfigManager::new(&tmp_path)?;
        // When everything finishes we delete the temporary file.
        let orchestrator = Orchestrator::new(args.clone()).await?;
        orchestrator.run().await?;
        let _ = fs::remove_file(tmp_path);
    } else {
        // Normal flow
        let orchestrator = Orchestrator::new(args.clone()).await?;
        orchestrator.run().await?;
    }

    Ok(())
}
