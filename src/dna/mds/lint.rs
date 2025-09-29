use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};

pub fn lint_files(files: Vec<PathBuf>, verbose: bool) -> Result<()> {
    if files.is_empty() {
        lint_project(verbose)
    } else {
        lint_specific_files(files, verbose)
    }
}
pub fn lint_project(verbose: bool) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("🔍 Linting HELIX project:");
        println!("  Project: {}", project_dir.display());
    }
    let mut helix_files = Vec::new();
    find_helix_files(&project_dir, &mut helix_files)?;
    if helix_files.is_empty() {
        println!("ℹ️  No HELIX files found to lint.");
        return Ok(());
    }
    println!("📋 Found {} HELIX files to lint", helix_files.len());
    let mut total_issues = 0;
    let mut files_with_issues = 0;
    for file in helix_files {
        match lint_single_file(&file, verbose) {
            Ok(issues) => {
                if !issues.is_empty() {
                    files_with_issues += 1;
                    total_issues += issues.len();
                    if !verbose {
                        println!("❌ {} issues in {}", issues.len(), file.display());
                    }
                } else if verbose {
                    println!("✅ No issues in {}", file.display());
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to lint {}: {}", file.display(), e);
            }
        }
    }
    if total_issues == 0 {
        println!("✅ No linting issues found!");
    } else {
        println!("\n📊 Linting Results:");
        println!("  Total issues: {}", total_issues);
        println!("  Files with issues: {}", files_with_issues);
        std::process::exit(1);
    }
    Ok(())
}
pub fn lint_specific_files(files: Vec<PathBuf>, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔍 Linting specific files:");
        println!("  Files: {}", files.len());
    }
    let mut total_issues = 0;
    let mut files_with_issues = 0;
    for file in files {
        if !file.exists() {
            eprintln!("❌ File not found: {}", file.display());
            continue;
        }
        if !file.extension().map_or(false, |ext| ext == "hlx") {
            eprintln!("⚠️  Skipping non-HELIX file: {}", file.display());
            continue;
        }
        match lint_single_file(&file, verbose) {
            Ok(issues) => {
                if !issues.is_empty() {
                    files_with_issues += 1;
                    total_issues += issues.len();
                    if !verbose {
                        println!("❌ {} issues in {}", issues.len(), file.display());
                    }
                } else if verbose {
                    println!("✅ No issues in {}", file.display());
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to lint {}: {}", file.display(), e);
            }
        }
    }
    if total_issues == 0 {
        println!("✅ No linting issues found!");
    } else {
        println!("\n📊 Linting Results:");
        println!("  Total issues: {}", total_issues);
        println!("  Files with issues: {}", files_with_issues);
        std::process::exit(1);
    }
    Ok(())
}
#[derive(Debug)]
struct LintIssue {
    line: usize,
    column: usize,
    severity: LintSeverity,
    message: String,
    rule: String,
}
#[derive(Debug)]
enum LintSeverity {
    Error,
    Warning,
    #[allow(dead_code)]
    Info,
}
pub fn lint_single_file(file: &PathBuf, verbose: bool) -> Result<Vec<LintIssue>> {
    let content = fs::read_to_string(file).context("Failed to read file")?;
    let mut issues = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;
        issues.extend(check_line_issues(line, line_num));
    }
    issues.extend(check_file_issues(&content, file));
    if verbose && !issues.is_empty() {
        println!("  Issues in {}:", file.display());
        for issue in &issues {
            println!(
                "    {}:{}:{}: {}: {} ({})", file.display(), issue.line, issue.column,
                format!("{:?}", issue.severity) .to_lowercase(), issue.message, issue
                .rule
            );
        }
    }
    Ok(issues)
}
pub fn check_line_issues(line: &str, line_num: usize) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    if line.ends_with(' ') || line.ends_with('\t') {
        issues
            .push(LintIssue {
                line: line_num,
                column: line.len(),
                severity: LintSeverity::Warning,
                message: "Trailing whitespace".to_string(),
                rule: "no-trailing-whitespace".to_string(),
            });
    }
    if line.contains('\t') && line.contains(' ') {
        issues
            .push(LintIssue {
                line: line_num,
                column: 1,
                severity: LintSeverity::Error,
                message: "Mixed tabs and spaces".to_string(),
                rule: "no-mixed-indentation".to_string(),
            });
    }
    if line.len() > 100 {
        issues
            .push(LintIssue {
                line: line_num,
                column: 101,
                severity: LintSeverity::Warning,
                message: "Line too long (over 100 characters)".to_string(),
                rule: "line-length".to_string(),
            });
    }
    if line.contains("  ") && line.trim().starts_with('{') {
        let indent = line.len() - line.trim_start().len();
        if indent % 2 != 0 {
            issues
                .push(LintIssue {
                    line: line_num,
                    column: 1,
                    severity: LintSeverity::Warning,
                    message: "Inconsistent indentation".to_string(),
                    rule: "indentation".to_string(),
                });
        }
    }
    issues
}
pub fn check_file_issues(content: &str, _file: &PathBuf) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    if !content.ends_with('\n') {
        issues
            .push(LintIssue {
                line: content.lines().count(),
                column: 1,
                severity: LintSeverity::Warning,
                message: "Missing newline at end of file".to_string(),
                rule: "newline-at-eof".to_string(),
            });
    }
    if content.starts_with('\u{FEFF}') {
        issues
            .push(LintIssue {
                line: 1,
                column: 1,
                severity: LintSeverity::Error,
                message: "Byte order mark (BOM) detected".to_string(),
                rule: "no-bom".to_string(),
            });
    }
    if content.contains("\r\n") {
        issues
            .push(LintIssue {
                line: 1,
                column: 1,
                severity: LintSeverity::Warning,
                message: "CRLF line endings detected (use LF)".to_string(),
                rule: "line-endings".to_string(),
            });
    }
    issues
}
pub fn find_helix_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "hlx" {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name == "target" || dir_name == "lib" {
                    continue;
                }
            }
            find_helix_files(&path, files)?;
        }
    }
    Ok(())
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

