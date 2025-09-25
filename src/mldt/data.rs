// ────────────────────────────────────────────────────────────────────────────
//   Claude JSON Xtra Love – Enhanced Data Analysis & Export (Rust)
//   Port of the original Python script, expanded to ~1 300 LOC.
//   © 2025 – made with 💝 for superior data management
// ────────────────────────────────────────────────────────────────────────────

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use ctrlc;
use dashmap::DashMap;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, error, info, warn, LevelFilter};
use once_cell::sync::Lazy;
use polars::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
use rusqlite::{params, Connection, Result as SqlResult};
use secp256k1::hashes::sha256;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use std::{hash::Hasher, str::FromStr};
use chrono::{DateTime, Utc};

// ---------------------------------------------------------------------------
// 1️⃣  CLI arguments (clap)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, ValueEnum)]
enum ExportFormat {
    Json,
    Jsonl,
    Csv,
    Parquet,
}

#[derive(Debug, Clone, ValueEnum)]
enum VizFormat {
    Png,
    Pdf,
    Svg,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "xtra-love-analyzer",
    version,
    about = "💝 Claude JSON Xtra Love – Enhanced Data Analysis & Export (Rust)",
    after_help = r#"
EXAMPLES:
  xtra-love-analyzer --analyze
  xtra-love-analyzer --export json csv
  xtra-love-analyzer --visualize --viz-format png
  xtra-love-analyzer --merge
  xtra-love-analyzer --report
  xtra-love-analyzer --all
  xtra-love-analyzer --stats
"#,
    arg_required_else_help = true
)]
struct Cli {
    #[arg(long, default_value = "claude-json-xtra-love")]
    base_dir: PathBuf,

    #[arg(long)]
    analyze: bool,

    #[arg(long)]
    quality: bool,

    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new([
        "json","jsonl","csv","parquet"
    ]))]
    export: Vec<ExportFormat>,

    #[arg(long)]
    visualize: bool,

    #[arg(long, default_value = "png")]
    viz_format: VizFormat,

    #[arg(long)]
    merge: bool,

    #[arg(long)]
    stats: bool,

    #[arg(long)]
    report: bool,

    #[arg(long)]
    all: bool,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long, short)]
    verbose: bool,
}

// ---------------------------------------------------------------------------
// 2️⃣  Global utilities (logging, hashing, compression, JSON streaming)
// ---------------------------------------------------------------------------

lazy_static! {
    static ref LOG_LEVEL: Mutex<LevelFilter> = Mutex::new(LevelFilter::Info);
}

fn configure_logging(base: &Path, level: LevelFilter, json: bool) {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(level);
    if json {
        builder.format(|buf, record| {
            writeln!(
                buf,
                "{{\"ts\":\"{}\",\"lvl\":\"{}\",\"msg\":\"{}\"}}",
                chrono::Utc::now().to_rfc3339(),
                record.level(),
                record.args()
            )
        });
    }
    builder.init();
    *LOG_LEVEL.lock().unwrap() = level;
}

/// Normalised SHA‑256 hash – same behaviour as the Python helper
fn calculate_hash(content: &str, normalize: bool) -> String {
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

/// Write JSON (optionally gzipped) to a file
fn save_with_compression<T: Serialize>(data: &T, path: impl AsRef<Path>, compress: bool) -> Result<()> {
    let raw = serde_json::to_string_pretty(data)?;
    if compress {
        let f = File::create(path)?;
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(raw.as_bytes())?;
        enc.finish()?;
    } else {
        let mut f = File::create(path)?;
        f.write_all(raw.as_bytes())?;
    }
    Ok(())
}

/// Stream a JSON file (plain or .gz) – yields each top‑level object
fn load_json_stream(path: impl AsRef<Path>) -> impl Iterator<Item = Result<JsonValue>> {
    let p = path.as_ref().to_path_buf();
    let opener = move || -> Result<Box<dyn Read>> {
        let f = File::open(&p)?;
        if p.extension()
            .map(|e| e == "gz")
            .unwrap_or(false)
        {
            Ok(Box::new(GzDecoder::new(f)))
        } else {
            Ok(Box::new(f))
        }
    };

    let reader = match opener() {
        Ok(r) => BufReader::new(r),
        Err(e) => return Box::new(std::iter::once(Err(anyhow!(e)))) as Box<dyn Iterator<Item = _>>,
    };

    // Very simple: each line is assumed to be a whole JSON object
    let lines = reader.lines().map(move |ln| {
        let s = ln?;
        let v: JsonValue = serde_json::from_str(&s)?;
        Ok(v)
    });
    Box::new(lines)
}

/// Small helper that writes JSONL in batches – used for exports
struct BatchWriter {
    file: Mutex<BufWriter<File>>,
    batch_size: usize,
    buffer: Mutex<Vec<JsonValue>>,
}

impl BatchWriter {
    fn new(path: impl AsRef<Path>, batch_size: usize) -> Result<Self> {
        let f = File::create(path)?;
        Ok(Self {
            file: Mutex::new(BufWriter::new(f)),
            batch_size,
            buffer: Mutex::new(Vec::with_capacity(batch_size)),
        })
    }

    fn add(&self, item: JsonValue) -> Result<()> {
        let mut buf = self.buffer.lock().unwrap();
        buf.push(item);
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
        let mut out = self.file.lock().unwrap();
        for v in buf.drain(..) {
            writeln!(out, "{}", serde_json::to_string(&v)?)?;
        }
        out.flush()?;
        Ok(())
    }
}

impl Drop for BatchWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ---------------------------------------------------------------------------
// 3️⃣  Data classes (Rust structs with serde)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
struct QualityMetrics {
    total_samples: usize,
    valid_samples: usize,
    invalid_samples: usize,
    quality_score: f64,
    diversity_score: f64,
    balance_score: f64,
    completeness_score: f64,
    duplicate_rate: f64,

    #[serde(skip)]
    #[serde(default)]
    validity_rate: f64,
}

impl QualityMetrics {
    fn compute_validity_rate(&mut self) {
        if self.total_samples == 0 {
            self.validity_rate = 0.0;
        } else {
            self.validity_rate = (self.valid_samples as f64 / self.total_samples as f64) * 100.0;
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ProcessingStats {
    total_items: usize,
    successful: usize,
    failed: usize,
    avg_processing_time: f64,
    avg_quality_score: f64,
    models_used: Vec<String>,
    categories_processed: HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// 4️⃣  Persistent deduplication store (SQLite wrapper)
// ---------------------------------------------------------------------------

mod dedup {
    use super::*;
    pub struct DedupStore {
        conn: Mutex<Connection>,
    }

    impl DedupStore {
        pub fn new(path: impl AsRef<Path>) -> Result<Self> {
            let conn = Connection::open(path)?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS dedup (
                    hash TEXT PRIMARY KEY,
                    category TEXT,
                    meta TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_category ON dedup(category);",
            )?;
            Ok(Self {
                conn: Mutex::new(conn),
            })
        }

        pub fn exists(&self, hash: &str) -> bool {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT 1 FROM dedup WHERE hash = ?1")
                .unwrap();
            stmt.exists(params![hash]).unwrap_or(false)
        }

        pub fn add(&self, hash: &str, category: &str, meta: impl Serialize) {
            let meta_json = serde_json::to_string(&meta).unwrap_or_default();
            let conn = self.conn.lock().unwrap();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO dedup (hash, category, meta) VALUES (?1, ?2, ?3)",
                params![hash, category, meta_json],
            );
        }

        pub fn get_stats(&self) -> HashMap<String, usize> {
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
            // Dropping self closes the connection
        }
    }
}

// ---------------------------------------------------------------------------
// 5️⃣  The main Analyzer struct – contains all heavy logic
// ---------------------------------------------------------------------------

struct XtraLoveAnalyzer {
    base_dir: PathBuf,
    marathon_dir: PathBuf,
    cleaned_dir: PathBuf,
    datasets_dir: PathBuf,
    analytics_dir: PathBuf,
    logger: log::Logger,
    dedup_store: Option<Arc<dedup::DedupStore>>,
    cache: Mutex<HashMap<String, (Instant, QualityMetrics)>>,
}

impl XtraLoveAnalyzer {
    fn new(base_dir: impl AsRef<Path>) -> Self {
        let base = base_dir.as_ref().to_path_buf();
        let marathon = base.join("marathon");
        let cleaned = base.join("cleaned");
        let datasets = base.join("datasets");
        let analytics = base.join("analytics");

        // Ensure essential directories exist
        for d in &[&base, &marathon, &cleaned, &datasets, &analytics] {
            let _ = fs::create_dir_all(d);
        }

        // Logging (fallback to simple env logger if utils missing)
        configure_logging(&base, LevelFilter::Info, false);
        let logger = log::logger();

        // Try to open the persistent dedup DB – ignore failures
        let dedup_store = match dedup::DedupStore::new(base.join("dedup.db")) {
            Ok(s) => {
                info!("🔍 Loaded dedup store with {} hashes", s.get_stats().len());
                Some(Arc::new(s))
            }
            Err(e) => {
                warn!("⚠️ Dedup store not available: {}", e);
                None
            }
        };

        Self {
            base_dir: base,
            marathon_dir: marathon,
            cleaned_dir: cleaned,
            datasets_dir: datasets,
            analytics_dir: analytics,
            logger,
            dedup_store,
            cache: Mutex::new(HashMap::new()),
        }
    }

    // -----------------------------------------------------------------------
    // 5.1  Marathon stage analysis
    // -----------------------------------------------------------------------
    fn analyze_marathon_stages(&self) -> Result<JsonValue> {
        info!("🏃 Analyzing marathon stages…");

        let mut stages_analysis = json!({
            "stages": {},
            "progression": [],
            "quality_improvement": [],
            "model_performance": {}
        });

        for stage_no in 1..=5 {
            let stage_dir = self.marathon_dir.join(format!("stage_{stage_no}"));
            if !stage_dir.is_dir() {
                continue;
            }
            let files: Vec<_> = fs::read_dir(&stage_dir)?
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().map(|e| e == "json" || e == "gz").unwrap_or(false))
                .map(|e| e.path())
                .collect();

            if files.is_empty() {
                continue;
            }

            // progress bar
            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap(),
            );
            pb.set_message(format!("Stage {stage_no}"));

            let mut categories = HashMap::new();
            let mut quality_scores = Vec::new();

            for file in files {
                pb.inc(1);
                for item in load_json_stream(&file) {
                    let obj = item?;
                    if let Some(it) = obj.get("item") {
                        if let Some(cat) = it.get("category") {
                            if let Some(cat_str) = cat.as_str() {
                                *categories.entry(cat_str.to_string()).or_insert(0usize) += 1;
                            }
                        }
                    }
                    if let Some(st) = obj.get("stage") {
                        if let Some(q) = st.get("quality_score") {
                            if let Some(qf) = q.as_f64() {
                                quality_scores.push(qf);
                                if let Some(model) = st.get("model") {
                                    if let Some(m) = model.as_str() {
                                        let mp = stages_analysis
                                            .get_mut("model_performance")
                                            .unwrap()
                                            .as_object_mut()
                                            .unwrap();
                                        let entry = mp.entry(m.to_string()).or_insert(json!({
                                            "count": 0,
                                            "total_quality": 0.0
                                        }));
                                        entry["count"] = json!(entry["count"].as_u64().unwrap() + 1);
                                        entry["total_quality"] = json!(entry["total_quality"]
                                            .as_f64()
                                            .unwrap()
                                            + qf);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            pb.finish_and_clear();

            let stage_key = format!("stage_{stage_no}");
            stages_analysis["stages"][stage_key] = json!({
                "total_files": files.len(),
                "categories": categories,
                "avg_quality": if quality_scores.is_empty() { 0.0 } else { quality_scores.iter().copied().sum::<f64>() / quality_scores.len() as f64 },
                "std_quality": if quality_scores.is_empty() { 0.0 } else { variance(&quality_scores).sqrt() },
                "min_quality": quality_scores.iter().copied().fold(f64::INFINITY, f64::min),
                "max_quality": quality_scores.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            });
        }

        // -------------------------------------------------------------------
        // 5.2  Progression analysis (complete_*.json)
        // -------------------------------------------------------------------
        let complete_files: Vec<_> = fs::read_dir(&self.marathon_dir)?
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("complete_"))
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .take(500) // sample limit
            .collect();

        let pb = ProgressBar::new(complete_files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.green/white}] {pos}/{len} ({eta})")
                .unwrap(),
        );
        pb.set_message("Progression");

        let mut progression = Vec::new();

        for file in complete_files {
            pb.inc(1);
            for data in load_json_stream(&file) {
                let obj = data?;
                if let Some(stages) = obj.get("stages").and_then(|v| v.as_array()) {
                    let scores: Vec<f64> = stages
                        .iter()
                        .filter_map(|s| s.get("quality_score"))
                        .filter_map(|q| q.as_f64())
                        .collect();
                    if scores.len() > 1 {
                        let improvement = scores.last().unwrap() - scores.first().unwrap();
                        let rate = if scores.first().unwrap() > &0.0 {
                            improvement / scores.first().unwrap() * 100.0
                        } else {
                            0.0
                        };
                        progression.push(json!({
                            "item_id": obj["item"]["id"],
                            "scores": scores,
                            "improvement": improvement,
                            "improvement_rate": rate
                        }));
                    }
                }
            }
        }
        pb.finish_and_clear();

        // keep a small sample for the final report
        let sample_prog = progression.iter().take(100).cloned().collect::<Vec<_>>();
        stages_analysis["progression"] = json!(sample_prog);

        // quality‑improvement aggregates
        if !progression.is_empty() {
            let improvements: Vec<f64> = progression
                .iter()
                .map(|p| p["improvement"].as_f64().unwrap())
                .collect();
            let rates: Vec<f64> = progression
                .iter()
                .map(|p| p["improvement_rate"].as_f64().unwrap())
                .collect();

            stages_analysis["quality_improvement"] = json!({
                "sample_size": progression.len(),
                "average_improvement": mean(&improvements),
                "median_improvement": median(&improvements),
                "std_improvement": variance(&improvements).sqrt(),
                "positive_rate": (improvements.iter().filter(|&&v| v > 0.0).count() as f64
                    / improvements.len() as f64) * 100.0,
                "average_improvement_rate": mean(&rates),
                "significant_improvement_rate": (rates.iter().filter(|&&r| r > 10.0).count() as f64
                    / rates.len() as f64) * 100.0
            });
        }

        // final aggregation for model performance
        if let Some(mp) = stages_analysis.get_mut("model_performance") {
            for (_, v) in mp.as_object_mut().unwrap() {
                let cnt = v["count"].as_u64().unwrap() as f64;
                let total = v["total_quality"].as_f64().unwrap();
                v["average_quality"] = json!(if cnt > 0.0 { total / cnt } else { 0.0 });
            }
        }

        Ok(stages_analysis)
    }

    // -----------------------------------------------------------------------
    // 5.3  Quality‑metric calculation (cached)
    // -----------------------------------------------------------------------
    fn analyze_quality_metrics(&self) -> Result<QualityMetrics> {
        const CACHE_KEY: &str = "quality_metrics";

        // Check cache (5‑minute TTL)
        {
            let cache = self.cache.lock().unwrap();
            if let Some((ts, cached)) = cache.get(CACHE_KEY) {
                if ts.elapsed() < Duration::from_secs(300) {
                    info!("📊 Using cached quality metrics");
                    let mut m = cached.clone();
                    m.compute_validity_rate();
                    return Ok(m);
                }
            }
        }

        info!("📊 Computing quality metrics…");
        let mut total_samples = 0usize;
        let mut valid_samples = 0usize;
        let mut duplicates = 0usize;
        let mut uniq_hashes = HashSet::new();

        // Helper to hash a generic JSON value (same as Python version)
        let hash_item = |it: &JsonValue| -> String {
            if let Ok(txt) = serde_json::to_string(it) {
                calculate_hash(&txt, true)
            } else {
                uuid::Uuid::new_v4().to_string()
            }
        };

        // Load from cleaned batches first (best quality)
        for file in fs::read_dir(&self.cleaned_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|e| e == "json" || e == "gz").unwrap_or(false))
        {
            for batch in load_json_stream(file.path()) {
                let data = batch?;
                if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
                    for it in arr {
                        total_samples += 1;
                        let h = hash_item(it);
                        if uniq_hashes.contains(&h) {
                            duplicates += 1;
                        } else {
                            uniq_hashes.insert(h);
                            // validation – same rules as the Python code
                            if let Some(resp) = it.get("final_response") {
                                if let Some(exs) = resp.get("training_examples").and_then(|v| v.as_array())
                                {
                                    if exs.len() >= 4 {
                                        valid_samples += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback to dataset files if cleaned dir empty
        if total_samples == 0 {
            for file in fs::read_dir(&self.datasets_dir)?
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().map(|e| e == "json" || e == "gz").unwrap_or(false))
            {
                for batch in load_json_stream(file.path()) {
                    let data = batch?;
                    if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
                        for it in arr {
                            total_samples += 1;
                            let h = hash_item(it);
                            if uniq_hashes.contains(&h) {
                                duplicates += 1;
                            } else {
                                uniq_hashes.insert(h);
                                if let Some(resp) = it.get("final_response") {
                                    if let Some(exs) = resp.get("training_examples").and_then(|v| v.as_array())
                                    {
                                        if exs.len() >= 4 {
                                            valid_samples += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ----------  calculate scores ----------
        let invalid_samples = total_samples - valid_samples;
        let quality_score = if total_samples == 0 {
            0.0
        } else {
            (valid_samples as f64 / total_samples as f64) * 100.0
        };
        let duplicate_rate = if total_samples == 0 {
            0.0
        } else {
            (duplicates as f64 / total_samples as f64) * 100.0
        };

        // Diversity – unique prompt hashes
        let mut unique_prompts = HashSet::new();
        let mut prompt_lengths = Vec::new();
        let mut chosen_lengths = Vec::new();

        for file in fs::read_dir(&self.cleaned_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|e| e == "json" || e == "gz").unwrap_or(false))
        {
            for batch in load_json_stream(file.path()) {
                let data = batch?;
                if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
                    for it in arr {
                        if let Some(resp) = it.get("final_response") {
                            if let Some(exs) = resp.get("training_examples").and_then(|v| v.as_array())
                            {
                                for ex in exs {
                                    if let Some(p) = ex.get("prompt").and_then(|v| v.as_str()) {
                                        let norm = p.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
                                        unique_prompts.insert(calculate_hash(&norm, true));
                                        prompt_lengths.push(p.len());
                                    }
                                    if let Some(ch) = ex.get("chosen").and_then(|v| v.as_str()) {
                                        chosen_lengths.push(ch.len());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let expected_unique = valid_samples * 4; // min 4 examples per item
        let diversity_score = if expected_unique == 0 {
            0.0
        } else {
            (unique_prompts.len() as f64 / expected_unique as f64) * 100.0 * 1.5
        }
        .min(100.0);

        // Balance – coefficient of variation of category counts
        let mut cat_counts = HashMap::<String, usize>::new();
        for file in fs::read_dir(&self.cleaned_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|e| e == "json" || e == "gz").unwrap_or(false))
        {
            for batch in load_json_stream(file.path()) {
                let data = batch?;
                if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
                    for it in arr {
                        if let Some(cat) = it
                            .get("item")
                            .and_then(|i| i.get("category"))
                            .and_then(|c| c.as_str())
                        {
                            *cat_counts.entry(cat.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let balance_score = if cat_counts.is_empty() {
            0.0
        } else {
            let vals: Vec<f64> = cat_counts.values().map(|&v| v as f64).collect();
            let mean = mean(&vals);
            let std = variance(&vals).sqrt();
            let cv = if mean > 0.0 { std / mean } else { 1.0 };
            ((1.0 - cv) * 100.0).max(0.0).min(100.0)
        };

        // Completeness (basic – can be extended)
        let completeness_score = if total_samples == 0 {
            0.0
        } else {
            (valid_samples as f64 / total_samples as f64) * 100.0
        } + if !prompt_lengths.is_empty() {
            let avg_len = mean(&prompt_lengths.iter().map(|&v| v as f64).collect::<Vec<_>>());
            if avg_len > 50.0 { 10.0 } else { 0.0 }
        } else {
            0.0
        };

        let mut metrics = QualityMetrics {
            total_samples,
            valid_samples,
            invalid_samples,
            quality_score,
            diversity_score,
            balance_score,
            completeness_score,
            duplicate_rate,
            validity_rate: 0.0,
        };
        metrics.compute_validity_rate();

        // Cache it
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(CACHE_KEY.to_string(), (Instant::now(), metrics.clone()));
        }

        Ok(metrics)
    }

    // -----------------------------------------------------------------------
    // 5.4  Visualisation (using Plotters)
    // -----------------------------------------------------------------------
    fn generate_visualizations(&self, fmt: VizFormat) -> Result<()> {
        info!("📈 Generating visualisations…");
        use plotters::prelude::*;

        // Load analyses (may be expensive – but we reuse the same methods)
        let marathon = self.analyze_marathon_stages()?;
        let metrics = self.analyze_quality_metrics()?;

        // Helper to map VizFormat → file suffix
        let suffix = match fmt {
            VizFormat::Png => "png",
            VizFormat::Pdf => "pdf",
            VizFormat::Svg => "svg",
        };

        let out_path = self.analytics_dir.join(format!("dashboard.{suffix}"));
        let root = BitMapBackend::new(&out_path, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;
        let (upper, lower) = root.split_vertically(0.6);

        // 1️⃣  Quality progression (line per item, avg line)
        {
            let mut chart = ChartBuilder::on(&upper)
                .caption("Quality Score Progression (Marathon)", ("sans-serif", 30))
                .margin(10)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(0..5, 0f64..100f64)?;
            chart.configure_mesh().x_desc("Stage").y_desc("Quality").draw()?;

            if let Some(arr) = marathon.get("progression").and_then(|v| v.as_array()) {
                // draw up to 30 random trajectories
                let mut rng = thread_rng();
                let sample: Vec<_> = arr
                    .choose_multiple(&mut rng, 30)
                    .cloned()
                    .collect();

                for itm in sample {
                    if let Some(scores) = itm.get("scores").and_then(|v| v.as_array()) {
                        let pts: Vec<(i32, f64)> = scores
                            .iter()
                            .enumerate()
                            .map(|(i, s)| (i as i32 + 1, s.as_f64().unwrap_or(0.0)))
                            .collect();
                        chart
                            .draw_series(LineSeries::new(pts.clone(), &BLUE.mix(0.2)))?
                            .label("sample")
                            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
                    }
                }

                // average line
                let mut sums = vec![0.0; 5];
                let mut cnts = vec![0usize; 5];
                for itm in arr {
                    if let Some(scores) = itm.get("scores").and_then(|v| v.as_array()) {
                        for (i, s) in scores.iter().enumerate() {
                            sums[i] += s.as_f64().unwrap_or(0.0);
                            cnts[i] += 1;
                        }
                    }
                }
                let avg_pts: Vec<(i32, f64)> = sums
                    .into_iter()
                    .zip(cnts)
                    .enumerate()
                    .map(|(i, (sum, cnt))| (i as i32 + 1, sum / cnt as f64))
                    .collect();
                chart
                    .draw_series(LineSeries::new(avg_pts, &RED))?
                    .label("average")
                    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
                chart.configure_series_labels().border_style(&BLACK).draw()?;
            }
        }

        // 2️⃣  Model performance bar chart (lower half)
        {
            let mut chart = ChartBuilder::on(&lower)
                .caption("Model Performance (Average Quality)", ("sans-serif", 30))
                .margin(10)
                .x_label_area_size(60)
                .y_label_area_size(60)
                .build_cartesian_2d(
                    (0..marathon["model_performance"]
                        .as_object()
                        .map(|o| o.len())
                        .unwrap_or(0))
                        as i32,
                    0f64..100f64,
                )?;
            chart.configure_mesh().x_desc("Model").y_desc("Avg Quality").draw()?;

            if let Some(mp) = marathon.get("model_performance").and_then(|v| v.as_object()) {
                let mut models: Vec<_> = mp.iter().collect();
                models.sort_by(|a, b| {
                    let qa = a.1.get("average_quality").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let qb = b.1.get("average_quality").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    qb.partial_cmp(&qa).unwrap()
                });

                for (idx, (name, data)) in models.iter().enumerate() {
                    let avg = data.get("average_quality").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(idx as i32, 0.0), (idx as i32 + 1, avg)],
                        BLUE.filled(),
                    )))?;
                }
            }
        }

        // Save file
        root.present()?;
        info!("📊 Dashboard saved to {}", out_path.display());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 5.5  Export (JSON, JSONL, CSV, Parquet) using Polars
    // -----------------------------------------------------------------------
    fn export_enhanced_datasets(
        &self,
        formats: &[ExportFormat],
    ) -> Result<HashMap<String, HashMap<String, usize>>> {
        info!("📦 Exporting datasets…");
        let mut stats: HashMap<String, HashMap<String, usize>> = HashMap::new();

        for split in &["train", "test", "validate"] {
            let src_path = self.datasets_dir.join(format!("{split}.json"));
            if !src_path.exists() {
                warn!("⚠️ Split file missing: {}", src_path.display());
                continue;
            }

            // Load lazily with Polars JSON reading (fast)
            let lf = JsonReader::new(File::open(&src_path)?)
                .with_json_format(JsonFormat::JsonLines) // we will treat whole file as array, fallback ok
                .finish()?;
            let df = lf.select(&[
                col("item.id").alias("item_id"),
                col("final_response.training_examples").arr().explode(),
            ])?;

            // explode to get each example as separate row
            let exploded = df
                .explode(["training_examples"])?
                .unnest(["training_examples"])?;

            // flatten nested fields
            let flattened = exploded
                .select(&[
                    col("item_id"),
                    col("final_response.training_examples.category"),
                    col("final_response.training_examples.level"),
                    col("final_response.training_examples.prompt"),
                    col("final_response.training_examples.chosen"),
                    col("final_response.training_examples.rejected"),
                    col("final_response.training_examples.explanation"),
                    col("final_response.training_examples.tags"),
                    col("final_response.training_examples.stage_origin"),
                    col("final_response.training_examples.model_origin"),
                    col("final_response.training_examples.quality_score"),
                ])?
                .with_column(lit(split.clone()).alias("split"));

            // Optimize dtypes
            let df = flattened
                .clone()
                .with_column(col("category").cast(DataType::Categorical(None))?)
                .with_column(col("level").cast(DataType::Categorical(None))?)
                .with_column(col("split").cast(DataType::Categorical(None))?)
                .with_column(col("quality_score").cast(DataType::Float64)?)?;

            // Export each requested format
            for fmt in formats {
                let fname = format!("{split}_enhanced.{fmt}");
                let out_path = self.datasets_dir.join(&fname);
                match fmt {
                    ExportFormat::Csv => {
                        CsvWriter::new(File::create(&out_path)?)
                            .has_header(true)
                            .finish(&df)?;
                    }
                    ExportFormat::Json => {
                        JsonWriter::new(File::create(&out_path)?)
                            .with_json_format(JsonFormat::Json)
                            .finish(&df)?;
                    }
                    ExportFormat::Jsonl => {
                        JsonWriter::new(File::create(&out_path)?)
                            .with_json_format(JsonFormat::JsonLines)
                            .finish(&df)?;
                    }
                    ExportFormat::Parquet => {
                        ParquetWriter::new(File::create(&out_path)?)
                            .with_compression(ParquetCompression::Snappy)
                            .finish(&df)?;
                    }
                };
                stats
                    .entry(split.to_string())
                    .or_default()
                    .insert(fmt.to_string(), df.height() as usize);
                info!("✅ Exported {} → {}", split, out_path.display());
            }
        }
        Ok(stats)
    }

    // -----------------------------------------------------------------------
    // 5.6  Merge / deduplicate all sources
    // -----------------------------------------------------------------------
    fn merge_and_deduplicate(&self, out_name: Option<String>) -> Result<usize> {
        info!("🔀 Merging data with deduplication…");
        let mut all_items = Vec::new();
        let mut seen = HashSet::new();
        let mut dup_cnt = 0usize;
        let mut source_counts = HashMap::new();

        // Helper to push a JSON object after dedup check
        let mut push_item = |item: JsonValue, src: &str| {
            let hash = if let Some(h) = item.get("hash_id") {
                h.as_str().unwrap_or("").to_string()
            } else {
                calculate_hash(&serde_json::to_string(&item).unwrap_or_default(), true)
            };
            if seen.contains(&hash) {
                dup_cnt += 1;
                return;
            }
            seen.insert(hash);
            all_items.push(item);
            *source_counts.entry(src.to_string()).or_insert(0usize) += 1;
        };

        // Sources (cleaned → marathon complete → dataset)
        let sources = [
            ("cleaned", self.cleaned_dir.clone(), "batch_*.json*"),
            ("marathon", self.marathon_dir.clone(), "complete_*.json*"),
            ("datasets", self.datasets_dir.clone(), "*.json*"),
        ];

        for (label, dir, pattern) in &sources {
            let files: Vec<_> = dir.glob(pattern)?.collect();
            if files.is_empty() {
                continue;
            }
            let pb = ProgressBar::new(files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap(),
            );
            pb.set_message(format!("Merging {label}"));
            for f in files {
                pb.inc(1);
                for data in load_json_stream(f) {
                    let obj = data?;
                    if let Some(arr) = obj.get("data").and_then(|v| v.as_array()) {
                        for it in arr {
                            push_item(it.clone(), label);
                        }
                    } else if obj.is_object() {
                        push_item(obj.clone(), label);
                    }
                }
            }
            pb.finish_and_clear();
        }

        // Save merged data
        let out_fname = out_name.unwrap_or_else(|| {
            format!(
                "merged_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            )
        });
        let out_path = self.base_dir.join(&out_fname);
        let payload = json!({
            "metadata": {
                "total_items": all_items.len(),
                "duplicates_removed": dup_cnt,
                "sources": source_counts,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "created_with": "💝 xtra love (Rust)"
            },
            "data": all_items
        });
        save_with_compression(&payload, &out_path, false)?;
        info!(
            "✅ Merged {} unique items → {} (removed {} duplicates)",
            all_items.len(),
            out_path.display(),
            dup_cnt
        );

        // Update dedup store if present
        if let Some(store) = &self.dedup_store {
            let before = store.get_stats().len();
            for h in seen {
                store.add(&h, "merged", json!({ "timestamp": chrono::Utc::now().to_rfc3339() }));
            }
            let after = store.get_stats().len();
            info!(
                "🔍 Dedup store now holds {} hashes (+{})",
                after,
                after - before
            );
        }

        Ok(all_items.len())
    }

    // -----------------------------------------------------------------------
    // 5.7  Detailed statistics (full JSON dump)
    // -----------------------------------------------------------------------
    fn calculate_statistics(&self) -> Result<JsonValue> {
        info!("📊 Computing detailed statistics…");
        let mut stats = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "by_model": {},
            "by_category": {},
            "by_level": {},
            "processing_times": [],
            "quality_distribution": {"scores": [], "buckets": {}},
            "file_statistics": {},
            "content_statistics": {"prompt_lengths": [], "chosen_lengths": [], "rejected_lengths": []}
        });

        // Sample up to 1000 complete marathon files
        let files: Vec<_> = fs::read_dir(&self.marathon_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().file_name().map(|n| n.to_string_lossy().starts_with("complete_")).unwrap_or(false))
            .map(|e| e.path())
            .take(1000)
            .collect();

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.yellow/black}] {pos}/{len} ({eta})")
                .unwrap(),
        );
        pb.set_message("Scanning");

        for file in files {
            pb.inc(1);
            for data in load_json_stream(&file) {
                let obj = data?;
                // total processing time
                if let Some(t) = obj.get("total_time").and_then(|v| v.as_f64()) {
                    stats["processing_times"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!(t));
                }

                // stages → model stats & quality
                if let Some(stages) = obj.get("stages").and_then(|v| v.as_array()) {
                    for st in stages {
                        let model = st.get("model").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let entry = stats["by_model"]
                            .as_object_mut()
                            .unwrap()
                            .entry(model.to_string())
                            .or_insert(json!({
                                "count": 0,
                                "quality_sum": 0.0,
                                "quality_scores": [],
                                "processing_times": []
                            }));

                        *entry["count"].as_u64_mut().unwrap() += 1;
                        if let Some(q) = st.get("quality_score").and_then(|v| v.as_f64()) {
                            entry["quality_sum"] = json!(entry["quality_sum"].as_f64().unwrap() + q);
                            entry["quality_scores"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!(q));
                            stats["quality_distribution"]["scores"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!(q));
                            // bucket 0‑10, 10‑20, …
                            let bucket = (q / 10.0).floor() as i32 * 10;
                            let bkey = format!("{}-{}", bucket, bucket + 10);
                            *stats["quality_distribution"]["buckets"]
                                .as_object_mut()
                                .unwrap()
                                .entry(bkey)
                                .or_insert(json!(0)) =
                                json!(
                                    stats["quality_distribution"]["buckets"]
                                        .as_object()
                                        .unwrap()
                                        .get(&bkey)
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        + 1
                                );
                        }
                    }
                }

                // category / level counts (item level)
                if let Some(item) = obj.get("item") {
                    if let Some(cat) = item.get("category").and_then(|v| v.as_str()) {
                        *stats["by_category"]
                            .as_object_mut()
                            .unwrap()
                            .entry(cat.to_string())
                            .or_insert(json!(0)) =
                            json!(
                                stats["by_category"]
                                    .as_object()
                                    .unwrap()
                                    .get(cat)
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0)
                                    + 1
                            );
                    }
                    if let Some(level) = item.get("level").and_then(|v| v.as_str()) {
                        *stats["by_level"]
                            .as_object_mut()
                            .unwrap()
                            .entry(level.to_string())
                            .or_insert(json!(0)) =
                            json!(
                                stats["by_level"]
                                    .as_object()
                                    .unwrap()
                                    .get(level)
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0)
                                    + 1
                            );
                    }
                }

                // content length stats (first 10 examples)
                if let Some(resp) = obj.get("final_response") {
                    if let Some(exs) = resp.get("training_examples").and_then(|v| v.as_array()) {
                        for ex in exs.iter().take(10) {
                            if let Some(p) = ex.get("prompt").and_then(|v| v.as_str()) {
                                stats["content_statistics"]["prompt_lengths"]
                                    .as_array_mut()
                                    .unwrap()
                                    .push(json!(p.len()));
                            }
                            if let Some(c) = ex.get("chosen").and_then(|v| v.as_str()) {
                                stats["content_statistics"]["chosen_lengths"]
                                    .as_array_mut()
                                    .unwrap()
                                    .push(json!(c.len()));
                            }
                            if let Some(r) = ex.get("rejected").and_then(|v| v.as_str()) {
                                stats["content_statistics"]["rejected_lengths"]
                                    .as_array_mut()
                                    .unwrap()
                                    .push(json!(r.len()));
                            }
                        }
                    }
                }
            }
        }
        pb.finish_and_clear();

        // File system statistics
        let mut total_size: u64 = 0;
        let mut file_counts: HashMap<String, usize> = HashMap::new();
        for entry in walkdir::WalkDir::new(&self.base_dir).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                let sz = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total_size += sz;
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();
                *file_counts.entry(ext).or_insert(0) += 1;
            }
        }
        stats["file_statistics"] = json!({
            "total_files": file_counts.values().sum::<usize>(),
            "by_extension": file_counts,
            "total_size_mb": (total_size as f64) / (1024.0 * 1024.0)
        });

        Ok(stats)
    }

    // -----------------------------------------------------------------------
    // 5.8  Report generation (plain‑text)
    // -----------------------------------------------------------------------
    fn generate_comprehensive_report(&self) -> Result<String> {
        info!("📝 Building comprehensive report…");
        let marathon = self.analyze_marathon_stages()?;
        let qm = self.analyze_quality_metrics()?;
        let stats = self.calculate_statistics()?;

        // Dedup info (if we have a store)
        let dedup_stats = if let Some(store) = &self.dedup_store {
            store.get_stats()
        } else {
            HashMap::new()
        };

        // Helper for pretty percentages
        let pct = |v: f64| format!("{:.1}%", v);
        let int = |v: usize| format!("{:,}", v);

        let mut lines = Vec::new();
        lines.push("=".repeat(70));
        lines.push("💝 CLAUDE JSON XTRA LOVE – COMPREHENSIVE ANALYSIS REPORT".into());
        lines.push("=".repeat(70));
        lines.push(format!(
            "Generated: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));
        lines.push(String::new());

        // -------------------------------------------------------------------
        // Executive summary
        // -------------------------------------------------------------------
        lines.push("📋 EXECUTIVE SUMMARY".into());
        lines.push("-".repeat(50));
        lines.push(format!(
            "Total Samples Processed: {}",
            int(qm.total_samples)
        ));
        lines.push(format!(
            "Valid Samples: {} ({})",
            int(qm.valid_samples),
            pct(qm.validity_rate)
        ));
        lines.push(format!("Overall Quality Score: {}", pct(qm.quality_score)));
        lines.push(String::new());

        // -------------------------------------------------------------------
        // Marathon analysis
        // -------------------------------------------------------------------
        lines.push("🏃 MARATHON PROCESSING ANALYSIS".into());
        lines.push("-".repeat(50));
        if let Some(stages) = marathon.get("stages").and_then(|v| v.as_object()) {
            for (stage_name, data) in stages {
                lines.push(format!("\n{}:", stage_name.to_uppercase()));
                lines.push(format!(
                    "  • Total Files: {}",
                    data.get("total_files")
                        .and_then(|v| v.as_u64())
                        .map(|v| int(v as usize))
                        .unwrap_or_else(|| "0".into())
                ));
                lines.push(format!(
                    "  • Avg Quality: {:.2}",
                    data.get("avg_quality")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                ));
                lines.push(format!(
                    "  • Quality Range: {:.2} – {:.2}",
                    data.get("min_quality")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    data.get("max_quality")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                ));
                if let Some(cats) = data.get("categories").and_then(|v| v.as_object()) {
                    lines.push("  • Top Categories:".into());
                    for (cat, cnt) in cats.iter().take(5) {
                        lines.push(format!("    - {}: {}", cat, cnt));
                    }
                }
            }
        }

        // -------------------------------------------------------------------
        // Model performance
        // -------------------------------------------------------------------
        lines.push("\nMODEL PERFORMANCE COMPARISON".into());
        lines.push("-".repeat(50));
        if let Some(mp) = marathon.get("model_performance").and_then(|v| v.as_object()) {
            for (model, stats) in mp.iter().sorted_by_key(|&(k, _)| k) {
                let avg = stats.get("average_quality").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let cnt = stats.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                if avg > 0.0 {
                    lines.push(format!(
                        "  • {} – avg {:.2} (n={})",
                        model,
                        avg,
                        cnt
                    ));
                }
            }
        }

        // -------------------------------------------------------------------
        // Quality improvement
        // -------------------------------------------------------------------
        if let Some(qi) = marathon.get("quality_improvement").and_then(|v| v.as_object()) {
            lines.push("\nQUALITY IMPROVEMENT METRICS".into());
            lines.push("-".repeat(50));
            lines.push(format!(
                "Sample size: {}",
                qi.get("sample_size")
                    .and_then(|v| v.as_u64())
                    .map(|v| int(v as usize))
                    .unwrap_or_else(|| "0".into())
            ));
            lines.push(format!(
                "Average improvement: {:.2}",
                qi.get("average_improvement")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            ));
            lines.push(format!(
                "Positive improvement rate: {:.1}%",
                qi.get("positive_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            ));
        }

        // -------------------------------------------------------------------
        // Dataset quality metrics
        // -------------------------------------------------------------------
        lines.push("\n📊 DATASET QUALITY METRICS".into());
        lines.push("-".repeat(50));
        lines.push(format!("Total Samples: {}", int(qm.total_samples)));
        lines.push(format!("Valid Samples: {}", int(qm.valid_samples)));
        lines.push(format!("Invalid Samples: {}", int(qm.invalid_samples)));
        lines.push(format!("Duplicate Rate: {}", pct(qm.duplicate_rate)));
        lines.push(String::new());
        lines.push("Quality Scores:".into());
        lines.push(format!("  • Overall: {}", pct(qm.quality_score)));
        lines.push(format!("  • Diversity: {}", pct(qm.diversity_score)));
        lines.push(format!("  • Balance: {}", pct(qm.balance_score)));
        lines.push(format!("  • Completeness: {}", pct(qm.completeness_score)));
        lines.push(format!(
            "  • Uniqueness: {}",
            pct(100.0 - qm.duplicate_rate)
        ));

        // -------------------------------------------------------------------
        // Deduplication statistics
        // -------------------------------------------------------------------
        if !dedup_stats.is_empty() {
            lines.push("\n🔍 DEDUPLICATION STATISTICS".into());
            lines.push("-".repeat(50));
            for (k, v) in dedup_stats.iter() {
                lines.push(format!("  • {}: {}", k, int(*v)));
            }
        }

        // -------------------------------------------------------------------
        // Dataset split stats
        // -------------------------------------------------------------------
        lines.push("\n📈 DATASET SPLIT STATISTICS".into());
        lines.push("-".repeat(50));
        for split in &["train", "test", "validate"] {
            let path = self.datasets_dir.join(format!("{split}.json"));
            if path.exists() {
                // quick count (stream)
                let mut cnt = 0usize;
                for batch in load_json_stream(&path) {
                    let obj = batch?;
                    if let Some(arr) = obj.get("data").and_then(|v| v.as_array()) {
                        cnt += arr.len();
                    }
                }
                lines.push(format!("  • {}: {}", split, int(cnt)));
            }
        }

        // -------------------------------------------------------------------
        // Filesystem statistics
        // -------------------------------------------------------------------
        if let Some(fs_stats) = stats.get("file_statistics") {
            lines.push("\n💾 STORAGE STATISTICS".into());
            lines.push("-".repeat(50));
            lines.push(format!(
                "Total size: {:.2} MB",
                fs_stats
                    .get("total_size_mb")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            ));
            if let Some(exts) = fs_stats.get("by_extension").and_then(|v| v.as_object()) {
                lines.push("File types:".into());
                for (ext, cnt) in exts.iter().sorted_by_key(|&(k, _)| k) {
                    lines.push(format!("  • {}: {}", ext, cnt));
                }
            }
        }

        // -------------------------------------------------------------------
        // Recommendations
        // -------------------------------------------------------------------
        lines.push("\n💡 RECOMMENDATIONS".into());
        lines.push("-".repeat(50));
        let mut recs = Vec::new();
        if qm.quality_score < 60.0 {
            recs.push("❗ Quality score below 60 % → review prompt templates and validation rules.");
        }
        if qm.diversity_score < 50.0 {
            recs.push("❗ Low diversity → ingest more varied source data, enrich prompts.");
        }
        if qm.balance_score < 70.0 {
            recs.push("❗ Category imbalance → rebalance training data per category.");
        }
        if qm.duplicate_rate > 10.0 {
            recs.push("❗ High duplicate rate → tighten dedup hashing or pre‑process inputs.");
        }
        if let Some(qi) = marathon.get("quality_improvement").and_then(|v| v.as_object()) {
            if let Some(pos) = qi.get("positive_rate").and_then(|v| v.as_f64()) {
                if pos < 50.0 {
                    recs.push("❗ Marathon stages rarely improve → tune enhancement prompts.");
                }
            }
        }
        if recs.is_empty() {
            lines.push("✅ Dataset looks healthy – no major issues detected.");
        } else {
            for r in recs {
                lines.push(r.into());
            }
        }

        // -------------------------------------------------------------------
        // Performance tips
        // -------------------------------------------------------------------
        lines.push("\n🚀 PERFORMANCE TIPS".into());
        lines.push("-".repeat(50));
        lines.push("• Install the optional `utils` crate for faster compression.");
        lines.push("• Use `--visualize` for quick PNG dashboards.");
        lines.push("• Enable parallel workers in the config for faster merging.");
        lines.push("• Keep the dedup DB on SSD for best throughput.");

        lines.push("\n".to_string());
        lines.push("=".repeat(70));
        lines.push("Report generated with 💝 xtra love (Rust)".into());
        lines.push("=".repeat(70));

        Ok(lines.join("\n"))
    }
}

// ---------------------------------------------------------------------------
// 6️⃣  Helper math utilities (mean, median, variance)
// ---------------------------------------------------------------------------
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}
fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = s.len() / 2;
    if s.len() % 2 == 0 {
        (s[mid - 1] + s[mid]) / 2.0
    } else {
        s[mid]
    }
}
fn variance(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0);
    var
}

// ---------------------------------------------------------------------------
// 7️⃣  Main entry point – replicates original CLI workflow
// ---------------------------------------------------------------------------
fn main() -> Result<()> {
    // Parse CLI
    let args = Cli::parse();

    // Adjust log level
    if args.verbose {
        env::set_var("RUST_LOG", "debug");
    }

    // Initialise analyzer
    let analyzer = XtraLoveAnalyzer::new(&args.base_dir);

    // If `--all` is set, enable every action
    let mut actions = Vec::new();
    if args.all {
        actions.extend_from_slice(&[
            "analyze", "quality", "export", "visualize", "merge", "stats", "report",
        ]);
    } else {
        if args.analyze {
            actions.push("analyze");
        }
        if args.quality {
            actions.push("quality");
        }
        if !args.export.is_empty() {
            actions.push("export");
        }
        if args.visualize {
            actions.push("visualize");
        }
        if args.merge {
            actions.push("merge");
        }
        if args.stats {
            actions.push("stats");
        }
        if args.report {
            actions.push("report");
        }
    }

    // -----------------------------------------------------------
    // 7.1  ANALYZE marathon
    // -----------------------------------------------------------
    if actions.contains(&"analyze") {
        let stg = analyzer.analyze_marathon_stages()?;
        println!("\n🏃 Marathon Analysis (truncated):");
        println!("{}", serde_json::to_string_pretty(&stg)?);
    }

    // -----------------------------------------------------------
    // 7.2  QUALITY metrics
    // -----------------------------------------------------------
    if actions.contains(&"quality") {
        let qm = analyzer.analyze_quality_metrics()?;
        println!("\n📊 Quality Metrics:");
        println!("  Total samples: {}", qm.total_samples);
        println!("  Valid samples: {} ({:.1}%)", qm.valid_samples, qm.validity_rate);
        println!("  Quality score: {:.1}%", qm.quality_score);
        println!("  Diversity score: {:.1}%", qm.diversity_score);
        println!("  Balance score: {:.1}%", qm.balance_score);
        println!("  Completeness: {:.1}%", qm.completeness_score);
        println!("  Duplicate rate: {:.1}%", qm.duplicate_rate);
    }

    // -----------------------------------------------------------
    // 7.3  EXPORT
    // -----------------------------------------------------------
    if actions.contains(&"export") {
        let fmt: Vec<ExportFormat> = args
            .export
            .iter()
            .map(|f| match f {
                ExportFormat::Json => ExportFormat::Json,
                ExportFormat::Jsonl => ExportFormat::Jsonl,
                ExportFormat::Csv => ExportFormat::Csv,
                ExportFormat::Parquet => ExportFormat::Parquet,
            })
            .collect();
        let stats = analyzer.export_enhanced_datasets(&fmt)?;
        println!("\n📦 Export statistics:");
        for (split, fm) in stats {
            println!("  {split}:");
            for (f, cnt) in fm {
                println!("    {f}: {cnt}");
            }
        }
    }

    // -----------------------------------------------------------
    // 7.4  VISUALISATION
    // -----------------------------------------------------------
    if actions.contains(&"visualize") {
        analyzer.generate_visualizations(args.viz_format)?;
    }

    // -----------------------------------------------------------
    // 7.5  MERGE & DE‑DUP
    // -----------------------------------------------------------
    if actions.contains(&"merge") {
        let out_name = args.output.clone().map(|p| p.to_string_lossy().into_owned());
        let total = analyzer.merge_and_deduplicate(out_name)?;
        println!("\n🔀 Merged {} unique items", total);
    }

    // -----------------------------------------------------------
    // 7.6  DETAILED STATISTICS
    // -----------------------------------------------------------
    if actions.contains(&"stats") {
        let stat_json = analyzer.calculate_statistics()?;
        println!("\n📈 Detailed statistics (truncated):");
        println!("{}", serde_json::to_string_pretty(&stat_json)?);

        let stats_path = analyzer
            .analytics_dir
            .join(format!("statistics_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
        save_with_compression(&stat_json, stats_path.clone(), false)?;
        println!("Full statistics saved to {}", stats_path.display());
    }

    // -----------------------------------------------------------
    // 7.7  REPORT
    // -----------------------------------------------------------
    if actions.contains(&"report") {
        let report = analyzer.generate_comprehensive_report()?;
        println!("\n{}", report);
        let out_path = if let Some(p) = args.output {
            p
        } else {
            analyzer
                .analytics_dir
                .join(format!("report_{}.txt", chrono::Utc::now().format("%Y%m%d_%H%M%S")))
        };
        let mut f = File::create(&out_path)?;
        f.write_all(report.as_bytes())?;
        println!("Report saved to {}", out_path.display());
    }

    // -----------------------------------------------------------
    // 7.8  Graceful shutdown (Ctrl‑C)
    // -----------------------------------------------------------
    let dedup_ref = analyzer.dedup_store.clone();
    ctrlc::set_handler(move || {
        info!("⚡ Received interrupt – shutting down cleanly");
        if let Some(store) = dedup_ref.clone() {
            store.close();
        }
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}
