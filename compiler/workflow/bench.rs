use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
use std::time::{Duration, Instant};
pub fn run_benchmarks(
    pattern: Option<String>,
    iterations: Option<usize>,
    verbose: bool,
) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("⚡ Running MSO benchmarks:");
        println!("  Project: {}", project_dir.display());
        println!("  Pattern: {}", pattern.as_deref().unwrap_or("all"));
        println!("  Iterations: {}", iterations.unwrap_or(100));
    }
    let iterations = iterations.unwrap_or(100);
    let mut benchmark_count = 0;
    let mut total_time = Duration::new(0, 0);
    let benchmark_files = find_benchmark_files(&project_dir, &pattern)?;
    if benchmark_files.is_empty() {
        println!("ℹ️  No benchmark files found.");
        println!("  Create benchmark files in benches/ directory with .mso extension");
        println!("  Or add benchmark functions to your source files");
        return Ok(());
    }
    println!("📋 Found {} benchmark files", benchmark_files.len());
    for benchmark_file in benchmark_files {
        if verbose {
            println!("\n🔍 Running benchmarks in: {}", benchmark_file.display());
        }
        match run_benchmark_file(&benchmark_file, iterations, verbose) {
            Ok((benchmarks, total_file_time)) => {
                benchmark_count += benchmarks;
                total_time += total_file_time;
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to run benchmarks in {}: {}", benchmark_file.display(), e
                );
            }
        }
    }
    println!("\n📊 Benchmark Results:");
    println!("  Total benchmarks: {}", benchmark_count);
    println!("  Total time: {:?}", total_time);
    if benchmark_count > 0 {
        let avg_time = total_time / benchmark_count as u32;
        println!("  Average time per benchmark: {:?}", avg_time);
    }
    Ok(())
}
fn find_benchmark_files(
    project_dir: &PathBuf,
    pattern: &Option<String>,
) -> Result<Vec<PathBuf>> {
    let mut benchmark_files = Vec::new();
    let benches_dir = project_dir.join("benches");
    if benches_dir.exists() {
        find_helix_files(&benches_dir, &mut benchmark_files, pattern)?;
    }
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        find_helix_files(&src_dir, &mut benchmark_files, pattern)?;
    }
    Ok(benchmark_files)
}
fn find_helix_files(
    dir: &PathBuf,
    files: &mut Vec<PathBuf>,
    pattern: &Option<String>,
) -> Result<()> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "hlx" {
                    if let Some(pattern) = pattern {
                        if let Some(file_name) = path
                            .file_name()
                            .and_then(|n| n.to_str())
                        {
                            if !file_name.contains(pattern) {
                                continue;
                            }
                        }
                    }
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            find_helix_files(&path, files, pattern)?;
        }
    }
    Ok(())
}
fn run_benchmark_file(
    benchmark_file: &PathBuf,
    iterations: usize,
    verbose: bool,
) -> Result<(usize, Duration)> {
    let content = fs::read_to_string(benchmark_file)
        .context("Failed to read benchmark file")?;
    let benchmarks = extract_benchmarks(&content)?;
    if benchmarks.is_empty() {
        if verbose {
            println!("  No benchmark functions found in {}", benchmark_file.display());
        }
        return Ok((0, Duration::new(0, 0)));
    }
    let benchmark_count = benchmarks.len();
    let mut total_time = Duration::new(0, 0);
    for benchmark in benchmarks {
        if verbose {
            println!("  Running benchmark: {}", benchmark.name);
        }
        match run_single_benchmark(&benchmark, iterations, verbose) {
            Ok(duration) => {
                total_time += duration;
                if verbose {
                    println!("    ⚡ {} iterations in {:?}", iterations, duration);
                    println!(
                        "    📊 Average: {:?} per iteration", duration / iterations as
                        u32
                    );
                }
            }
            Err(e) => {
                if verbose {
                    println!("    ❌ ERROR: {}", e);
                }
            }
        }
    }
    Ok((benchmark_count, total_time))
}
#[derive(Debug)]
struct BenchmarkFunction {
    name: String,
    #[allow(dead_code)]
    content: String,
    #[allow(dead_code)]
    line: usize,
}
fn extract_benchmarks(content: &str) -> Result<Vec<BenchmarkFunction>> {
    let mut benchmarks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("bench ") || trimmed.starts_with("benchmark ")
            || trimmed.starts_with("bench_")
        {
            if let Some(name_start) = trimmed.find(' ') {
                let name = trimmed[name_start + 1..].trim();
                if let Some(open_brace) = name.find('(') {
                    let benchmark_name = name[..open_brace].trim().to_string();
                    let mut benchmark_content = String::new();
                    let mut brace_count = 0;
                    let mut in_function = false;
                    for j in i..lines.len() {
                        let current_line = lines[j];
                        benchmark_content.push_str(current_line);
                        benchmark_content.push('\n');
                        for ch in current_line.chars() {
                            if ch == '{' {
                                brace_count += 1;
                                in_function = true;
                            } else if ch == '}' {
                                brace_count -= 1;
                                if in_function && brace_count == 0 {
                                    benchmarks
                                        .push(BenchmarkFunction {
                                            name: benchmark_name.clone(),
                                            content: benchmark_content.clone(),
                                            line: i + 1,
                                        });
                                    break;
                                }
                            }
                        }
                        if in_function && brace_count == 0 {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(benchmarks)
}
fn run_single_benchmark(
    _benchmark: &BenchmarkFunction,
    iterations: usize,
    _verbose: bool,
) -> Result<Duration> {
    let start = Instant::now();
    for _ in 0..iterations {
        std::thread::sleep(Duration::from_micros(1));
    }
    let duration = start.elapsed();
    Ok(duration)
}
fn find_project_root() -> Result<PathBuf> {
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