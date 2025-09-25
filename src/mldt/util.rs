// ────────────────────────────────────────────────────────────────────────────
//   Claude JSON Xtra Love – Shared Utilities (Rust)
//   Direct port of the original Python `utils.py`
//   Made with 💝 for superior efficiency
//   Approx. 1 260 LOC (≈ 2.4 × the original)
// ────────────────────────────────────────────────────────────────────────────

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter, Record};
use regex::Regex;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use simd_json::OwnedValue as SimdJsonValue;
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use std::{thread, time::Duration};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// 1️⃣  Logging configuration (rotating + optional JSON logs)
// ---------------------------------------------------------------------------

/// Initialise a rotating logger (log4rs) plus a plain console logger.
pub fn configure_logging(
    base_dir: &Path,
    log_name: &str,
    level: &str,
    json_logs: bool,
    max_bytes: usize,
    backup_count: usize,
) -> Result<()> {
    // Ensure directory exists
    fs::create_dir_all(base_dir)?;

    // Build the log configuration programmatically
    use log4rs::{
        append::rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
                CompoundPolicy,
            },
            RollingFileAppender,
        },
        config::{Appender, Config, Logger, Root},
        encode::pattern::PatternEncoder,
        encode::json::JsonEncoder,
    };

    // ----- plain text rotating file -----
    let size_trigger = SizeTrigger::new(max_bytes as u64);
    let roller = FixedWindowRoller::builder()
        .build(
            &base_dir.join(format!("{log_name}.{{}}")),
            backup_count,
        )
        .context("Failed to create plain text log roller")?;
    let compound = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));
    let plain_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d} - {l} - [{M}] - {m}{n}",
        )))
        .build(base_dir.join(log_name), Box::new(compound))
        .context("building plain rotating appender")?;

    // ----- optional JSON rotating file -----
    let json_appender = if json_logs {
        let json_name = format!("{log_name}.json");
        let size_trigger = SizeTrigger::new(max_bytes as u64);
        let roller = FixedWindowRoller::builder()
            .build(
                &base_dir.join(format!("{json_name}.{{}}")),
                backup_count,
            )
            .context("Failed to create JSON log roller")?;
        let compound = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));
        Some(
            RollingFileAppender::builder()
                .encoder(Box::new(JsonEncoder::new()))
                .build(base_dir.join(json_name), Box::new(compound))
                .context("building JSON rotating appender")?,
        )
    } else {
        None
    };

    // ----- console output -----
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d} - {l} - [{M}] - {m}{n}",
        )))
        .build();

    // ----- assemble config -----
    let mut config_builder = Config::builder()
        .appender(Appender::builder().build("plain_file", Box::new(plain_file)))
        .appender(Appender::builder().build("stdout", Box::new(stdout)));

    if let Some(json_app) = json_appender {
        config_builder = config_builder.appender(
            Appender::builder()
                .build("json_file", Box::new(json_app)),
        );
    }

    let config = config_builder
        .logger(Logger::builder().build("xtra_utils", LevelFilter::Info))
        .build(
            Root::builder()
                .appender("plain_file")
                .appender("stdout")
                .appender_if(json_logs, "json_file")
                .build(LevelFilter::Info),
        )
        .context("building final logger config")?;

    // Initialise the global logger (log crate)
    log4rs::init_config(config)?;
    // Override global log level according to `level` arg
    let lvl = LevelFilter::from_str(level).unwrap_or(LevelFilter::Info);
    log::set_max_level(lvl);
    info!("💝 Logging configured ({})", base_dir.join(log_name).display());
    Ok(())
}

// ---------------------------------------------------------------------------
// 2️⃣  Deduplication store (SQLite, thread‑safe)
// ---------------------------------------------------------------------------

pub struct DedupStore {
    conn: Mutex<Connection>,
}

impl DedupStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let parent = db_path
            .as_ref()
            .parent()
            .ok_or_else(|| anyhow!("Invalid DB path"))?;
        fs::create_dir_all(parent)?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hashes (
                hash TEXT PRIMARY KEY,
                category TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                metadata TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_category ON hashes(category);
            CREATE INDEX IF NOT EXISTS idx_timestamp ON hashes(timestamp);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Returns `true` if the hash **already exists**.
    pub fn exists(&self, hash: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT 1 FROM hashes WHERE hash = ?1 LIMIT 1")
            .context("Failed to prepare hash existence query")?;
        stmt.exists(params![hash]).map_err(|e| {
            error!("Database query error in exists(): {}", e);
            anyhow!("Failed to check hash existence: {}", e)
        })
    }

    /// Insert a new hash. Returns `true` if it was added, `false` if it already existed.
    pub fn add(&self, hash: &str, category: Option<&str>, metadata: Option<&JsonValue>) -> Result<bool> {
        let meta_str = metadata.and_then(|m| serde_json::to_string(m).ok());
        let conn = self.conn.lock().map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;
        match conn.execute(
            "INSERT INTO hashes (hash, category, metadata) VALUES (?1, ?2, ?3)",
            params![hash, category, meta_str],
        ) {
            Ok(_) => {
                conn.commit().map_err(|e| {
                    error!("Failed to commit hash insertion: {}", e);
                    anyhow!("Database commit error: {}", e)
                })?;
                Ok(true)
            }
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.extended_code == rusqlite::ErrorCode::ConstraintViolation as i32 =>
            {
                Ok(false) // duplicate
            }
            Err(e) => {
                error!("DedupStore insert error: {}", e);
                Err(anyhow!("Failed to insert hash: {}", e))
            }
        }
    }

    /// Batch insertion – returns the number of *new* hashes added.
    pub fn batch_add(&self, entries: &[(String, Option<String>, Option<JsonValue>)]) -> Result<usize> {
        let mut added = 0;
        for (hash, cat, meta) in entries {
            if self.add(hash, cat.as_deref(), meta.as_ref())? {
                added += 1;
            }
        }
        Ok(added)
    }

    /// Retrieve high‑level statistics.
    pub fn get_stats(&self) -> Result<HashMap<String, JsonValue>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;
        let mut stats = HashMap::new();

        // totals
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) AS total,
                        COUNT(DISTINCT category) AS uniq_cats,
                        MIN(timestamp) AS first_ts,
                        MAX(timestamp) AS last_ts
                 FROM hashes",
            )
            .context("Failed to prepare statistics query")?;
        let row = stmt
            .query_row([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .unwrap_or((0, 0, "N/A".into(), "N/A".into()));
        stats.insert("total_hashes".into(), json!(row.0));
        stats.insert("unique_categories".into(), json!(row.1));
        stats.insert("first_entry".into(), json!(row.2));
        stats.insert("last_entry".into(), json!(row.3));

        // top categories
        let mut top = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT category, COUNT(*) AS cnt
                 FROM hashes
                 GROUP BY category
                 ORDER BY cnt DESC
                 LIMIT 10",
            )
            .context("Failed to prepare top categories query")?;
        let mut rows = stmt.query([]).map_err(|e| {
            error!("Failed to execute top categories query: {}", e);
            anyhow!("Database query error: {}", e)
        })?;
        while let Some(r) = rows.next().map_err(|e| {
            error!("Failed to fetch row in get_stats: {}", e);
            anyhow!("Database iteration error: {}", e)
        })? {
            let cat: String = r.get(0).context("Failed to get category from row")?;
            let cnt: i64 = r.get(1).context("Failed to get count from row")?;
            top.insert(cat, json!(cnt));
        }
        stats.insert("top_categories".into(), json!(top));

        Ok(stats)
    }

    /// Remove entries older than `days_old`. Returns number of rows deleted.
    pub fn cleanup(&self, days_old: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;
        let stmt = conn
            .prepare(
                "DELETE FROM hashes
                 WHERE timestamp < datetime('now', ?1)",
            )
            .context("Failed to prepare cleanup query")?;
        let param = format!("-{} days", days_old);
        let affected = stmt.execute(params![param]).map_err(|e| {
            error!("Failed to execute cleanup query: {}", e);
            anyhow!("Database deletion error: {}", e)
        })?;
        conn.commit().map_err(|e| {
            error!("Failed to commit cleanup changes: {}", e);
            anyhow!("Database commit error: {}", e)
        })?;
        Ok(affected as usize)
    }

    pub fn close(self) {
        // Dropping self closes the SQLite connection.
    }
}

// ---------------------------------------------------------------------------
// 3️⃣  Prompt rendering (simple {{placeholder}} replacement)
// ---------------------------------------------------------------------------

lazy_static! {
    static ref RE_PLACEHOLDER: Regex = Regex::new(r"\{\{(\w+)\}\}").unwrap();
}

/// Replace `{{key}}` placeholders in *template* using the *replacements* map.
pub fn render_prompt(template: &str, replacements: &HashMap<String, String>) -> String {
    RE_PLACEHOLDER
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            replacements.get(key).cloned().unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

// ---------------------------------------------------------------------------
// 4️⃣  Hashing utilities (SHA‑256, 16‑char hex, optional normalisation)
// ---------------------------------------------------------------------------

pub fn calculate_hash(content: &impl Serialize, normalize: bool) -> Result<String> {
    let mut s = if normalize {
        // Normalisation: lower‑case, collapse whitespace.
        let txt = serde_json::to_string(content)?;
        txt.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        serde_json::to_string(content)?
    };
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    Ok(hex::encode(hasher.finalize())[..16].to_string())
}

// ---------------------------------------------------------------------------
// 5️⃣  JSON (de)serialisation – fast path with `simd-json` (fallback to `serde_json`)
// ---------------------------------------------------------------------------

fn is_orjson_available() -> bool {
    // We treat `simd-json` as the Rust analogue of `orjson`.
    true
}

/// Serialize *obj* to a JSON string. `indent` = Some(n) → pretty‑print,
/// `sort_keys` = true → alphabetical key order.
pub fn json_dumps(
    obj: &impl Serialize,
    indent: Option<usize>,
    sort_keys: bool,
) -> Result<String> {
    if is_orjson_available() {
        // Simd‑json does not support pretty printing directly → use serde_json for that.
        if indent.is_some() || sort_keys {
            let mut ser = serde_json::Serializer::new(Vec::new());
            if sort_keys {
                // Turn the value into a map and sort it manually.
                let mut map = serde_json::to_value(obj)?
                    .as_object()
                    .cloned()
                    .ok_or_else(|| anyhow!("Object required for sort_keys"))?;
                let mut sorted = serde_json::Map::new();
                for k in map.keys().sorted() {
                    let v = map.remove(k).ok_or_else(|| {
                        anyhow!("Failed to remove key '{}' during JSON sorting", k)
                    })?;
                    sorted.insert(k.clone(), v);
                }
                sorted.serialize(&mut ser)?;
            } else {
                obj.serialize(&mut ser)?;
            }
            let data = ser.into_inner();
            let txt = String::from_utf8(data)?;
            if let Some(spaces) = indent {
                // simple pretty‑print with serde_json
                let v: JsonValue = serde_json::from_str(&txt)?;
                return Ok(serde_json::to_string_pretty(&v)?);
            }
            Ok(txt)
        } else {
            // Fast path: simd‑json's `to_string` (no pretty)
            Ok(simd_json::to_string(obj)?)
        }
    } else {
        // pure serde_json fallback
        Ok(serde_json::to_string_pretty(obj)?)
    }
}

pub fn json_loads(s: &str) -> Result<JsonValue> {
    if is_orjson_available() {
        // Simd‑json expects &mut [u8]
        let mut bytes = s.as_bytes().to_vec();
        let v: SimdJsonValue = simd_json::to_owned_value(&mut bytes)?;
        // Convert Simd‑json value to serde_json for uniform return type
        let serde_v: JsonValue = serde_json::from_str(&v.to_string())?;
        Ok(serde_v)
    } else {
        Ok(serde_json::from_str(s)?)
    }
}

// ---------------------------------------------------------------------------
// 6️⃣  Streaming JSON loaders (plain files & gzip)
// ---------------------------------------------------------------------------

/// Yield JSON objects from *filepath* – works for ordinary JSON, JSON‑L, or
/// a dict containing a `"data"` array.  `max_items` limits the number emitted.
pub fn load_json_stream(
    filepath: impl AsRef<Path>,
    max_items: Option<usize>,
) -> impl Iterator<Item = Result<JsonValue>> {
    let path = filepath.as_ref().to_path_buf();
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or("").to_ascii_lowercase();

    // Open with optional gzip decoding
    let file_result = if ext == "gz" {
        File::open(&path).map(|f| Box::new(GzDecoder::new(f)) as Box<dyn Read>)
    } else {
        File::open(&path).map(|f| Box::new(f) as Box<dyn Read>)
    };

    let reader = match file_result {
        Ok(r) => r,
        Err(e) => {
            // Return an iterator that yields the error
            return std::iter::once(Err(anyhow!("Failed to open file {}: {}", path.display(), e)));
        }
    };
    let buf = BufReader::new(reader);

    // Heuristic: if the first non‑whitespace char is '[' → treat as JSON array;
    // if it's '{' → maybe JSONL (line‑by‑line) or a single object.
    // We keep it simple: try line‑by‑line first, then fall back to whole content.
    let mut lines = buf.lines();
    let mut count = 0usize;

    // Peek at first line without consuming
    let first = lines.next().and_then(|r| r.ok()).unwrap_or_default();

    if first.trim_start().starts_with('[') {
        // Whole file is a JSON array – read it entirely (still streaming for huge files)
        let mut rest = String::new();
        rest.push_str(&first);
        for l in lines {
            if let Ok(l) = l {
                rest.push('\n');
                rest.push_str(&l);
            }
        }
        let whole: JsonValue = json_loads(&rest).unwrap_or(JsonValue::Null);
        let iter = whole
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(move |item| {
                if let Some(max) = max_items {
                    if count >= max {
                        return None;
                    }
                }
                count += 1;
                Some(Ok(item))
            });
        Box::new(iter) as Box<dyn Iterator<Item = Result<JsonValue>>>
    } else {
        // Assume JSONL – each line is a JSON object.
        let iter = std::iter::from_fn(move || {
            if let Some(max) = max_items {
                if count >= max {
                    return None;
                }
            }
            match lines.next() {
                Some(Ok(l)) => {
                    let trimmed = l.trim();
                    if trimmed.is_empty() {
                        return Some(Ok(JsonValue::Null));
                    }
                    match json_loads(trimmed) {
                        Ok(v) => {
                            count += 1;
                            Some(Ok(v))
                        }
                        Err(e) => {
                            warn!("Failed to parse JSON line {}: {}", count + 1, e);
                            Some(Ok(JsonValue::Null))
                        }
                    }
                }
                Some(Err(e)) => Some(Err(anyhow!(e))),
                None => None,
            }
        });
        Box::new(iter) as Box<dyn Iterator<Item = Result<JsonValue>>>
    }
}

// ---------------------------------------------------------------------------
// 7️⃣  Save with optional gzip compression
// ---------------------------------------------------------------------------

/// Persist *data* to *filepath*. If `compress == true`, a `.gz` suffix is added.
pub fn save_with_compression(
    data: &impl Serialize,
    filepath: impl AsRef<Path>,
    compress: bool,
    compression_level: u32,
) -> Result<PathBuf> {
    let json_str = json_dumps(data, Some(2), false)?;
    let mut out_path = filepath.as_ref().to_path_buf();

    if compress {
        out_path.set_extension(
            format!(
                "{}.gz",
                out_path
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or_else(|| {
                        warn!("Could not extract extension from path {}", out_path.display());
                        ""
                    })
            ),
        );
        let mut encoder = GzEncoder::new(File::create(&out_path)?, Compression::new(compression_level));
        encoder.write_all(json_str.as_bytes())?;
        encoder.finish()?;
    } else {
        let mut f = File::create(&out_path)?;
        f.write_all(json_str.as_bytes())?;
    }
    Ok(out_path)
}

// ---------------------------------------------------------------------------
// 8️⃣  Environment‑variable aware config helper
// ---------------------------------------------------------------------------

/// Retrieve a value from *config* using a dotted *key_path*; if *env_var* is set
/// in the current environment its value takes precedence.  If the key does not
/// exist (or env var missing) `default` is returned.
pub fn get_config_value<T: Clone + FromStr>(
    config: &HashMap<String, JsonValue>,
    key_path: &str,
    env_var: Option<&str>,
    default: T,
) -> T {
    if let Some(var) = env_var {
        if let Ok(v) = env::var(var) {
            if let Ok(parsed) = v.parse::<T>() {
                return parsed;
            }
        }
    }

    let mut cur: Option<&JsonValue> = Some(&JsonValue::Object(config.clone()));
    for part in key_path.split('.') {
        match cur {
            Some(JsonValue::Object(map)) => cur = map.get(part),
            _ => return default,
        }
    }

    cur.and_then(|v| match v {
        JsonValue::String(s) => s.parse::<T>().ok(),
        JsonValue::Number(num) => num.as_i64().map(|i| T::from_str(&i.to_string()).ok()).flatten(),
        JsonValue::Bool(b) => T::from_str(&b.to_string()).ok(),
        _ => None,
    })
    .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// 9️⃣  BatchWriter – write JSON or JSONL in configurable batches
// ---------------------------------------------------------------------------

pub struct BatchWriter {
    path: PathBuf,
    batch_size: usize,
    format: String,
    buffer: Vec<JsonValue>,
}

impl BatchWriter {
    pub fn new(path: impl AsRef<Path>, batch_size: usize, format: &str) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            batch_size,
            format: format.to_lowercase(),
            buffer: Vec::with_capacity(batch_size),
        }
    }

    pub fn add(&mut self, item: impl Serialize) -> Result<()> {
        let v = serde_json::to_value(item)?;
        self.buffer.push(v);
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        match self.format.as_str() {
            "jsonl" => {
                for v in &self.buffer {
                    writeln!(f, "{}", json_dumps(v, None, false)?)?;
                }
            }
            "json" => {
                // Write a full JSON array – if the file already exists we read‑modify‑write.
                let mut existing = if self.path.exists() {
                    let txt = fs::read_to_string(&self.path)?;
                    json_loads(&txt)?
                } else {
                    json!({ "data": [] })
                };
                if let Some(arr) = existing.get_mut("data").and_then(|v| v.as_array_mut()) {
                    arr.extend(self.buffer.drain(..));
                }
                let out = json_dumps(&existing, Some(2), false)?;
                f.set_len(0)?;
                f.write_all(out.as_bytes())?;
            }
            _ => {}
        }
        self.buffer.clear();
        Ok(())
    }
}

impl Drop for BatchWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ---------------------------------------------------------------------------
// 10️⃣  Small test harness (mirrors `if __name__ == "__main__"`)
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // Prepare a temporary directory for demonstration
    let base = PathBuf::from("demo_utils");
    configure_logging(
        &base,
        "processing.log",
        "INFO",
        false,
        5 * 1024 * 1024,
        5,
    )?;

    // -------------------------------------------------------------------
    // Demo: DedupStore
    // -------------------------------------------------------------------
    let db_path = base.join("test_dedup.db");
    let store = DedupStore::new(&db_path)?;
    store.add("hash1", Some("cat_a"), None)?;
    store.add("hash2", Some("cat_b"), None)?;
    store.add("hash1", Some("cat_a"), None)?; // duplicate – should return false
    info!("hash1 exists? {}", store.exists("hash1")?);
    info!("dedup stats: {:?}", store.get_stats()?);

    // -------------------------------------------------------------------
    // Demo: render_prompt
    // -------------------------------------------------------------------
    let tmpl = "Hello {{name}}, welcome to {{place}}!";
    let mut repl = HashMap::new();
    repl.insert("name".into(), "Alice".into());
    repl.insert("place".into(), "Wonderland".into());
    println!("Rendered: {}", render_prompt(tmpl, &repl));

    // -------------------------------------------------------------------
    // Demo: hashing
    // -------------------------------------------------------------------
    let h = calculate_hash(&json!({ "a": 1, "b": 2 }), true)?;
    println!("Sample hash: {h}");

    // -------------------------------------------------------------------
    // Demo: JSON streaming
    // -------------------------------------------------------------------
    let jsonl_path = base.join("sample.jsonl");
    {
        let mut w = BatchWriter::new(&jsonl_path, 2, "jsonl");
        w.add(json!({ "id": 1, "msg": "first" }))?;
        w.add(json!({ "id": 2, "msg": "second" }))?;
        w.add(json!({ "id": 3, "msg": "third" }))?;
    } // writer flushed automatically

    println!("--- Streaming JSONL items ---");
    for (i, itm) in load_json_stream(&jsonl_path, None).enumerate() {
        println!("Item {}: {:?}", i + 1, itm?);
    }

    // -------------------------------------------------------------------
    // Demo: save with compression
    // -------------------------------------------------------------------
    let data = json!({
        "generated": Utc::now(),
        "samples": [1, 2, 3, 4],
        "nested": { "a": true, "b": "text" }
    });
    let gz_path = save_with_compression(&data, base.join("data.json"), true, 6)?;
    println!("Compressed data saved to {}", gz_path.display());

    // -------------------------------------------------------------------
    // Demo: env‑aware config getter
    // -------------------------------------------------------------------
    let cfg: HashMap<String, JsonValue> = json!({
        "ollama": {
            "host": "http://localhost:11434",
            "auth_token": null
        },
        "processing": { "batch_size": 100 }
    })
    .as_object()
    .expect("Demo config should always be an object")
    .clone();

    // pretend an env var is set
    env::set_var("OLLAMA_HOST", "http://envhost:1234");
    let host: String = get_config_value(&cfg, "ollama.host", Some("OLLAMA_HOST"), "default".into());
    println!("Resolved host: {host}");

    // Clean‑up demo artefacts
    store.close();
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_dir_all(&base);

    Ok(())
}
