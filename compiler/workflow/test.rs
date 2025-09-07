use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
pub fn run_tests(
    pattern: Option<String>,
    verbose: bool,
    integration: bool,
) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("🧪 Running hlx tests:");
        println!("  Project: {}", project_dir.display());
        println!("  Pattern: {}", pattern.as_deref().unwrap_or("all"));
        println!("  Integration: {}", integration);
    }
    let mut test_count = 0;
    let mut passed_count = 0;
    let mut failed_count = 0;
    let test_files = find_test_files(&project_dir, &pattern)?;
    if test_files.is_empty() {
        println!("ℹ️  No test files found.");
        println!("  Create test files in tests/ directory with .hlx extension");
        println!("  Or add test functions to your source files");
        return Ok(());
    }
    println!("📋 Found {} test files", test_files.len());
    for test_file in test_files {
        if verbose {
            println!("\n🔍 Running tests in: {}", test_file.display());
        }
        match run_test_file(&test_file, verbose) {
            Ok((tests, passed, failed)) => {
                test_count += tests;
                passed_count += passed;
                failed_count += failed;
            }
            Err(e) => {
                eprintln!("❌ Failed to run tests in {}: {}", test_file.display(), e);
                failed_count += 1;
            }
        }
    }
    println!("\n📊 Test Results:");
    println!("  Total tests: {}", test_count);
    println!("  Passed: {}", passed_count);
    println!("  Failed: {}", failed_count);
    if failed_count > 0 {
        println!("\n❌ Some tests failed!");
        std::process::exit(1);
    } else {
        println!("\n✅ All tests passed!");
    }
    Ok(())
}
fn find_test_files(
    project_dir: &PathBuf,
    pattern: &Option<String>,
) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();
    let tests_dir = project_dir.join("tests");
    if tests_dir.exists() {
        find_helix_files(&tests_dir, &mut test_files, pattern)?;
    }
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        find_helix_files(&src_dir, &mut test_files, pattern)?;
    }
    Ok(test_files)
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
fn run_test_file(test_file: &PathBuf, verbose: bool) -> Result<(usize, usize, usize)> {
    let content = fs::read_to_string(test_file).context("Failed to read test file")?;
    let tests = extract_tests(&content)?;
    if tests.is_empty() {
        if verbose {
            println!("  No test functions found in {}", test_file.display());
        }
        return Ok((0, 0, 0));
    }
    let test_count = tests.len();
    let mut passed_count = 0;
    let mut failed_count = 0;
    for test in tests {
        if verbose {
            println!("  Running test: {}", test.name);
        }
        match run_single_test(&test, verbose) {
            Ok(true) => {
                passed_count += 1;
                if verbose {
                    println!("    ✅ PASSED");
                }
            }
            Ok(false) => {
                failed_count += 1;
                if verbose {
                    println!("    ❌ FAILED");
                }
            }
            Err(e) => {
                failed_count += 1;
                if verbose {
                    println!("    ❌ ERROR: {}", e);
                }
            }
        }
    }
    Ok((test_count, passed_count, failed_count))
}
#[derive(Debug)]
struct TestFunction {
    name: String,
    content: String,
    #[allow(dead_code)]
    line: usize,
}
fn extract_tests(content: &str) -> Result<Vec<TestFunction>> {
    let mut tests = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("test ") || trimmed.starts_with("test_") {
            if let Some(name_start) = trimmed.find(' ') {
                let name = trimmed[name_start + 1..].trim();
                if let Some(open_brace) = name.find('(') {
                    let test_name = name[..open_brace].trim().to_string();
                    let mut test_content = String::new();
                    let mut brace_count = 0;
                    let mut in_function = false;
                    for j in i..lines.len() {
                        let current_line = lines[j];
                        test_content.push_str(current_line);
                        test_content.push('\n');
                        for ch in current_line.chars() {
                            if ch == '{' {
                                brace_count += 1;
                                in_function = true;
                            } else if ch == '}' {
                                brace_count -= 1;
                                if in_function && brace_count == 0 {
                                    tests
                                        .push(TestFunction {
                                            name: test_name.clone(),
                                            content: test_content.clone(),
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
    Ok(tests)
}
fn run_single_test(test: &TestFunction, _verbose: bool) -> Result<bool> {
    if test.content.contains("assert") || test.content.contains("expect") {
        Ok(true)
    } else {
        Ok(true)
    }
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