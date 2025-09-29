#[cfg(test)]
use criterion::*;
#[cfg(test)]
use crate::parse;
#[cfg(test)]
use crate::mds::loader::BinaryLoader;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use tempfile::TempDir;
#[cfg(test)]
const SMALL_hlx: &str = r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
}
"#;
#[cfg(test)]
const MEDIUM_hlx: &str = r#"
project "benchmark" {
    version = "1.0.0"
    author = "tester"
}

agent "analyzer" {
    model = "claude-3"
    role = "Code Analyzer"
    temperature = 0.5
    max_tokens = 2000

    capabilities [
        "code-review"
        "bug-detection"
        "performance-analysis"
    ]
}

workflow "review-process" {
    trigger = "manual"

    step "analyze" {
        agent = "analyzer"
        task = "Review the code"
        timeout = 5m
    }

    step "report" {
        agent = "analyzer"
        task = "Generate report"
        timeout = 2m
    }
}

crew "review-team" {
    agents ["analyzer"]
    process = "sequential"
}
"#;
#[cfg(test)]
fn generate_large_hlx(agents: usize, workflows: usize) -> String {
    let mut hlx = String::from("project \"large\" { version = \"1.0.0\" }\n\n");
    for i in 0..agents {
        hlx.push_str(
            &format!(
                r#"
agent "agent_{}" {{
    model = "gpt-4"
    role = "Agent {}"
    temperature = 0.7
    capabilities ["task-{}", "skill-{}"]
}}
"#,
                i, i, i, i
            ),
        );
    }
    for i in 0..workflows {
        hlx.push_str(
            &format!(
                r#"
workflow "workflow_{}" {{
    trigger = "manual"
    step "step_{}" {{
        agent = "agent_{}"
        task = "Execute task {}"
    }}
}}
"#,
                i, i, i % agents, i
            ),
        );
    }
    hlx
}
#[cfg(test)]
fn benchmark_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    group
        .bench_function(
            "small",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(SMALL_hlx));
                });
            },
        );
    group
        .bench_function(
            "medium",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(MEDIUM_hlx));
                });
            },
        );
    let large_hlx = generate_large_hlx(50, 100);
    group
        .bench_function(
            "large",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(&large_hlx));
                });
            },
        );
    group.finish();
}
// Commented out - Compiler API not available
// fn benchmark_compilation(c: &mut Criterion) {
//     let mut group = c.benchmark_group("compilation");
//     // Compiler API not implemented
//     group.finish();
// }
#[cfg(test)]
fn benchmark_binary_vs_text_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("loading");
    let temp_dir = TempDir::new().unwrap();
    let text_path = temp_dir.path().join("test.hlxbb");
    let binary_path = temp_dir.path().join("test.hlxb");
    fs::write(&text_path, MEDIUM_hlx).unwrap();
    // Note: Compilation API not available, using placeholder
    // For now, just test parsing and loading existing binaries
    // let binary = ... compilation code ...
    // let serializer = crate::mds::serializer::BinarySerializer::new(true);
    // serializer.write_to_file(&binary, &binary_path).unwrap();
    group
        .bench_function(
            "text_loading",
            |b| {
                b.iter(|| {
                    let content = fs::read_to_string(&text_path).unwrap();
                    let _ = crate::parse_and_validate(black_box(&content));
                });
            },
        );
    let loader = BinaryLoader::new();
    group
        .bench_function(
            "binary_loading",
            |b| {
                b.iter(|| {
                    let _ = loader.load_file(black_box(&binary_path));
                });
            },
        );
    let mmap_loader = BinaryLoader::new().with_mmap(true);
    group
        .bench_function(
            "mmap_loading",
            |b| {
                b.iter(|| {
                    let _ = mmap_loader.load_file(black_box(&binary_path));
                });
            },
        );
    group.finish();
}
#[cfg(test)]
fn benchmark_optimization_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization");
    let large_hlx = generate_large_hlx(30, 60);
        for level in 0..=3 {
            let opt_level = match level {
                0 => crate::mds::compile::OptimizationLevel::Zero,
                1 => crate::mds::compile::OptimizationLevel::One,
                2 => crate::mds::compile::OptimizationLevel::Two,
                _ => crate::mds::compile::OptimizationLevel::Three,
            };
        group
            .bench_with_input(
                BenchmarkId::from_parameter(level),
                &opt_level,
                |b, &opt_level| {
                    let compiler = crate::mds::compile::Compiler::new(opt_level);
                    b.iter(|| {
                        let _ = compiler.compile_source(black_box(&large_hlx), None);
                    });
                },
            );
    }
    group.finish();
}
#[cfg(test)]
fn benchmark_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");
    let mut hlx_with_duplicates = String::new();
    for i in 0..100 {
        hlx_with_duplicates
            .push_str(
                &format!(
                    r#"
agent "agent_{}" {{
    model = "gpt-4"  // Same model for all
    role = "Assistant"  // Same role for all
    temperature = 0.7  // Same temperature
    capabilities ["coding", "testing", "debugging"]  // Same capabilities
}}
"#,
                    i
                ),
            );
    }
    let opt_compiler = crate::mds::compile::Compiler::new(crate::mds::compile::OptimizationLevel::Two);
    group
        .bench_function(
            "with_deduplication",
            |b| {
                b.iter(|| {
                    let _ = opt_compiler
                        .compile_source(black_box(&hlx_with_duplicates), None);
                });
            },
        );
    let no_opt_compiler = crate::mds::compile::Compiler::new(crate::mds::compile::OptimizationLevel::Zero);
    group
        .bench_function(
            "without_deduplication",
            |b| {
                b.iter(|| {
                    let _ = no_opt_compiler
                        .compile_source(black_box(&hlx_with_duplicates), None);
                });
            },
        );
    group.finish();
}
#[cfg(test)]
criterion_group!(
    benches, benchmark_parsing, benchmark_compilation, benchmark_binary_vs_text_loading,
    benchmark_optimization_levels, benchmark_string_interning
);
#[cfg(test)]
criterion_main!(benches);