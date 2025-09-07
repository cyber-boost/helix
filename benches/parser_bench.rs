use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use helix_config::{parse, compile::Compiler, compiler::loader::BinaryLoader};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
const SMALL_MSO: &str = r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
}
"#;
const MEDIUM_MSO: &str = r#"
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
fn generate_large_mso(agents: usize, workflows: usize) -> String {
    let mut mso = String::from("project \"large\" { version = \"1.0.0\" }\n\n");
    for i in 0..agents {
        mso.push_str(
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
        mso.push_str(
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
    mso
}
fn benchmark_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    group
        .bench_function(
            "small",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(SMALL_MSO));
                });
            },
        );
    group
        .bench_function(
            "medium",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(MEDIUM_MSO));
                });
            },
        );
    let large_mso = generate_large_mso(50, 100);
    group
        .bench_function(
            "large",
            |b| {
                b.iter(|| {
                    let _ = parse(black_box(&large_mso));
                });
            },
        );
    group.finish();
}
fn benchmark_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("compilation");
    let compiler = Compiler::new(helix_config::compile::OptimizationLevel::Two);
    group
        .bench_function(
            "small",
            |b| {
                b.iter(|| {
                    let _ = compiler.compile_source(black_box(SMALL_MSO), None);
                });
            },
        );
    group
        .bench_function(
            "medium",
            |b| {
                b.iter(|| {
                    let _ = compiler.compile_source(black_box(MEDIUM_MSO), None);
                });
            },
        );
    let large_mso = generate_large_mso(50, 100);
    group
        .bench_function(
            "large",
            |b| {
                b.iter(|| {
                    let _ = compiler.compile_source(black_box(&large_mso), None);
                });
            },
        );
    group.finish();
}
fn benchmark_binary_vs_text_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("loading");
    let temp_dir = TempDir::new().unwrap();
    let text_path = temp_dir.path().join("test.hlxbb");
    let binary_path = temp_dir.path().join("test.hlxb");
    fs::write(&text_path, MEDIUM_MSO).unwrap();
    let compiler = Compiler::new(helix_config::compile::OptimizationLevel::Two);
    let binary = compiler.compile_file(&text_path).unwrap();
    let serializer = helix_config::compiler::serializer::BinarySerializer::new(true);
    serializer.write_to_file(&binary, &binary_path).unwrap();
    group
        .bench_function(
            "text_loading",
            |b| {
                b.iter(|| {
                    let content = fs::read_to_string(&text_path).unwrap();
                    let _ = helix_config::parse_and_validate(black_box(&content));
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
fn benchmark_optimization_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimization");
    let large_mso = generate_large_mso(30, 60);
    for level in 0..=3 {
        let opt_level = match level {
            0 => helix_config::compile::OptimizationLevel::Zero,
            1 => helix_config::compile::OptimizationLevel::One,
            2 => helix_config::compile::OptimizationLevel::Two,
            _ => helix_config::compile::OptimizationLevel::Three,
        };
        group
            .bench_with_input(
                BenchmarkId::from_parameter(level),
                &opt_level,
                |b, &opt_level| {
                    let compiler = Compiler::new(opt_level);
                    b.iter(|| {
                        let _ = compiler.compile_source(black_box(&large_mso), None);
                    });
                },
            );
    }
    group.finish();
}
fn benchmark_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");
    let mut mso_with_duplicates = String::new();
    for i in 0..100 {
        mso_with_duplicates
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
    let opt_compiler = Compiler::new(helix_config::compile::OptimizationLevel::Two);
    group
        .bench_function(
            "with_deduplication",
            |b| {
                b.iter(|| {
                    let _ = opt_compiler
                        .compile_source(black_box(&mso_with_duplicates), None);
                });
            },
        );
    let no_opt_compiler = Compiler::new(helix_config::compile::OptimizationLevel::Zero);
    group
        .bench_function(
            "without_deduplication",
            |b| {
                b.iter(|| {
                    let _ = no_opt_compiler
                        .compile_source(black_box(&mso_with_duplicates), None);
                });
            },
        );
    group.finish();
}
criterion_group!(
    benches, benchmark_parsing, benchmark_compilation, benchmark_binary_vs_text_loading,
    benchmark_optimization_levels, benchmark_string_interning
);
criterion_main!(benches);