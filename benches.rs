use std::time::{Duration, Instant};
use std::collections::HashMap;
pub struct Benchmark {
    name: String,
    iterations: usize,
    warmup: usize,
}
impl Benchmark {
    pub fn new(name: &str) -> Self {
        Benchmark {
            name: name.to_string(),
            iterations: 1000,
            warmup: 100,
        }
    }
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }
    pub fn with_warmup(mut self, warmup: usize) -> Self {
        self.warmup = warmup;
        self
    }
    pub fn run<F>(&self, mut f: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        for _ in 0..self.warmup {
            f();
        }
        let mut timings = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed();
            timings.push(elapsed);
        }
        timings.sort();
        let total: Duration = timings.iter().sum();
        let mean = total / self.iterations as u32;
        let median = timings[self.iterations / 2];
        let min = timings[0];
        let max = timings[timings.len() - 1];
        let p95 = timings[self.iterations * 95 / 100];
        let p99 = timings[self.iterations * 99 / 100];
        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            mean,
            median,
            min,
            max,
            p95,
            p99,
        }
    }
}
#[derive(Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub mean: Duration,
    pub median: Duration,
    pub min: Duration,
    pub max: Duration,
    pub p95: Duration,
    pub p99: Duration,
}
impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Benchmark: {}", self.name)?;
        writeln!(f, "  Iterations: {}", self.iterations)?;
        writeln!(f, "  Mean:       {:?}", self.mean)?;
        writeln!(f, "  Median:     {:?}", self.median)?;
        writeln!(f, "  Min:        {:?}", self.min)?;
        writeln!(f, "  Max:        {:?}", self.max)?;
        writeln!(f, "  95th %ile:  {:?}", self.p95)?;
        writeln!(f, "  99th %ile:  {:?}", self.p99)?;
        Ok(())
    }
}
pub fn generate_small_config() -> String {
    r#"
    agent "assistant" {
        model = "gpt-4"
        temperature = 0.7
        max_tokens = 100000
    }
    "#
        .to_string()
}
pub fn generate_medium_config() -> String {
    let mut config = String::new();
    config
        .push_str(
            r#"
project "test-system" {
    version = "1.0.0"
    author = "Test"
    description = "Benchmark test configuration"
}
"#,
        );
    for i in 0..10 {
        config
            .push_str(
                &format!(
                    r#"
agent "agent-{}" {{
    model = "gpt-4"
    role = "Agent {}"
    temperature = 0.7
    max_tokens = 100000
    
    capabilities [
        "capability-1"
        "capability-2"
        "capability-3"
    ]
    
    backstory {{
        Experience in domain {}
        Specialized knowledge
        Problem solving skills
    }}
}}
"#,
                    i, i, i
                ),
            );
    }
    for i in 0..5 {
        config
            .push_str(
                &format!(
                    r#"
workflow "workflow-{}" {{
    trigger = "manual"
    
    step "step-1" {{
        agent = "agent-{}"
        task = "Process data"
        timeout = 30m
    }}
    
    step "step-2" {{
        agent = "agent-{}"
        task = "Analyze results"
        timeout = 15m
        depends_on = ["step-1"]
    }}
    
    pipeline {{
        step-1 -> step-2
    }}
}}
"#,
                    i, i, (i + 1) % 10
                ),
            );
    }
    config
}
pub fn generate_large_config() -> String {
    let mut config = String::new();
    config
        .push_str(
            r#"
project "large-system" {
    version = "2.0.0"
    author = "Benchmark"
    description = "Large benchmark configuration"
}
"#,
        );
    config
        .push_str(
            r#"
memory {
    provider = "postgres"
    connection = "postgresql://localhost/benchmark"
    
    embeddings {
        model = "text-embedding-3-small"
        dimensions = 1536
        batch_size = 100
    }
    
    cache_size = 10000
    persistence = true
}
"#,
        );
    for i in 0..100 {
        config
            .push_str(
                &format!(
                    r#"
agent "agent-{:03}" {{
    model = "{}"
    role = "Specialist {}"
    temperature = {}
    max_tokens = {}
    
    capabilities [
        "skill-{}"
        "skill-{}"
        "skill-{}"
        "skill-{}"
        "skill-{}"
    ]
    
    tools [
        "tool-{}"
        "tool-{}"
        "tool-{}"
    ]
    
    backstory {{
        {} years of experience
        Expert in domain {}
        Published {} papers
        Led {} projects
    }}
}}
"#,
                    i, if i % 2 == 0 { "gpt-4" } else { "claude-3-opus" }, i, 0.5 + (i as
                    f32 * 0.01), 50000 + (i * 1000), i * 2, i * 2 + 1, i * 2 + 2, i * 2 +
                    3, i * 2 + 4, i * 3, i * 3 + 1, i * 3 + 2, 10 + i % 20, i % 10, i *
                    2, i * 3
                ),
            );
    }
    for i in 0..50 {
        config
            .push_str(
                &format!(
                    r#"
workflow "workflow-{:03}" {{
    trigger = "{}"
    
"#, i, if i
                    % 3 == 0 { "manual" } else if i % 3 == 1 { "schedule:0 * * * *" }
                    else { "event:data-ready" }
                ),
            );
        let step_count = 5 + (i % 6);
        for j in 0..step_count {
            let depends = if j == 0 {
                String::new()
            } else {
                format!("depends_on = [\"step-{}\"]", j - 1)
            };
            config
                .push_str(
                    &format!(
                        r#"
    step "step-{}" {{
        agent = "agent-{:03}"
        task = "Process task {} in workflow {}"
        timeout = {}m
        parallel = {}
        {}
        
        retry {{
            max_attempts = {}
            delay = {}s
            backoff = "{}"
        }}
    }}
"#,
                        j, (i * 2 + j) % 100, j, i, 10 + j * 5, j % 2 == 0, depends, 3,
                        30, if j % 3 == 0 { "fixed" } else if j % 3 == 1 { "linear" }
                        else { "exponential" }
                    ),
                );
        }
        config.push_str("    pipeline {\n        ");
        for j in 0..step_count {
            if j > 0 {
                config.push_str(" -> ");
            }
            config.push_str(&format!("step-{}", j));
        }
        config.push_str("\n    }\n}\n");
    }
    for i in 0..10 {
        config
            .push_str(
                &format!(
                    r#"
context "context-{}" {{
    environment = "{}"
    debug = {}
    max_tokens = {}
    
    secrets {{
        api_key_{} = $API_KEY_{}
        secret_{} = vault:"secrets/{}/key"
        config_{} = file:"/etc/config/{}.json"
    }}
    
    variables {{
        timeout = {}s
        retries = {}
        batch_size = {}
    }}
}}
"#,
                    i, if i % 3 == 0 { "dev" } else if i % 3 == 1 { "staging" } else {
                    "prod" }, i % 2 == 0, 100000 + i * 10000, i, i, i, i, i, i, 30 + i *
                    10, 3 + i % 5, 100 + i * 10
                ),
            );
    }
    for i in 0..20 {
        config.push_str(&format!(r#"
crew "crew-{:02}" {{
    agents [
"#, i));
        let agent_count = 3 + (i % 5);
        for j in 0..agent_count {
            config.push_str(&format!("        \"agent-{:03}\"\n", (i * 5 + j) % 100));
        }
        config
            .push_str(
                &format!(
                    r#"    ]
    
    process = "{}"
    manager = "agent-{:03}"
    max_iterations = {}
    verbose = {}
}}
"#,
                    if i % 4 == 0 { "sequential" } else if i % 4 == 1 { "hierarchical" }
                    else if i % 4 == 2 { "parallel" } else { "consensus" }, i % 100, 10 +
                    i, i % 2 == 0
                ),
            );
    }
    config
}
#[cfg(test)]
mod lexer_benchmarks {
    use super::*;
    use crate::lexer;
    #[test]
    fn bench_lexer_small() {
        let config = generate_small_config();
        let bench = Benchmark::new("lexer_small").with_iterations(1000).with_warmup(100);
        let result = bench
            .run(|| {
                let _ = lexer::tokenize(&config).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(1));
    }
    #[test]
    fn bench_lexer_medium() {
        let config = generate_medium_config();
        let bench = Benchmark::new("lexer_medium").with_iterations(100).with_warmup(10);
        let result = bench
            .run(|| {
                let _ = lexer::tokenize(&config).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(10));
    }
    #[test]
    fn bench_lexer_large() {
        let config = generate_large_config();
        let bench = Benchmark::new("lexer_large").with_iterations(10).with_warmup(2);
        let result = bench
            .run(|| {
                let _ = lexer::tokenize(&config).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(100));
    }
    #[test]
    fn bench_lexer_with_source_locations() {
        let config = generate_medium_config();
        let bench = Benchmark::new("lexer_with_locations")
            .with_iterations(100)
            .with_warmup(10);
        let result = bench
            .run(|| {
                let _ = lexer::tokenize_with_locations(&config).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(15));
    }
}
#[cfg(test)]
mod parser_benchmarks {
    use super::*;
    use crate::{lexer, parser};
    #[test]
    fn bench_parser_small() {
        let config = generate_small_config();
        let tokens = lexer::tokenize(&config).unwrap();
        let bench = Benchmark::new("parser_small")
            .with_iterations(1000)
            .with_warmup(100);
        let result = bench
            .run(|| {
                let _ = parser::parse(tokens.clone()).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(1));
    }
    #[test]
    fn bench_parser_medium() {
        let config = generate_medium_config();
        let tokens = lexer::tokenize(&config).unwrap();
        let bench = Benchmark::new("parser_medium").with_iterations(100).with_warmup(10);
        let result = bench
            .run(|| {
                let _ = parser::parse(tokens.clone()).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(20));
    }
    #[test]
    fn bench_parser_large() {
        let config = generate_large_config();
        let tokens = lexer::tokenize(&config).unwrap();
        let bench = Benchmark::new("parser_large").with_iterations(10).with_warmup(2);
        let result = bench
            .run(|| {
                let _ = parser::parse(tokens.clone()).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(200));
    }
    #[test]
    fn bench_end_to_end() {
        let config = generate_large_config();
        let bench = Benchmark::new("end_to_end_large")
            .with_iterations(10)
            .with_warmup(2);
        let result = bench
            .run(|| {
                let tokens = lexer::tokenize(&config).unwrap();
                let _ = parser::parse(tokens).unwrap();
            });
        println!("{}", result);
        assert!(result.median < Duration::from_millis(300));
    }
}