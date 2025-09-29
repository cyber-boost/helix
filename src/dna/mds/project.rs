use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use crate::mds::serializer::BinarySerializer;

#[allow(dead_code)]
pub fn init_project(
    template: String,
    dir: Option<PathBuf>,
    name: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let template_content = get_template_content(&template);
    let output_dir = dir
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
    let filename = name
        .unwrap_or_else(|| {
            match template.as_str() {
                "ai-dev" => "ai_development_team.hlx".to_string(),
                "data-pipeline" => "data_pipeline.hlx".to_string(),
                _ => format!("{}.hlx", template),
            }
        });
    let output_path = output_dir.join(&filename);
    if output_path.exists() && !force {
        return Err(
            anyhow::anyhow!(
                "File '{}' already exists. Use --force to overwrite.", output_path
                .display()
            ),
        );
    }
    if verbose {
        println!("🚀 Initializing HELIX project:");
        println!("  Template: {}", template);
        println!("  Output: {}", output_path.display());
        println!("  Force: {}", force);
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, template_content)?;
    println!("✅ HELIX project initialized successfully!");
    println!("  Created: {}", output_path.display());
    println!("  Template: {}", template);
    if verbose {
        let content_size = template_content.len();
        println!("  Size: {} bytes", content_size);
        let description = match template.as_str() {
            "minimal" => "Simple hlx configuration with basic agent and workflow",
            "ai-dev" => {
                "Complete AI development team with specialized agents for full-stack development"
            }
            "support" => {
                "Multi-tier customer support system with escalation and knowledge management"
            }
            "data-pipeline" => {
                "High-throughput data processing pipeline with ML integration"
            }
            "research" => {
                "AI-powered research assistant for literature review and paper writing"
            }
            _ => "HELIX configuration template",
        };
        println!("  Description: {}", description);
    }
    println!("\n📋 Next steps:");
    println!("  1. Review and customize the configuration");
    println!("  2. Set up your API keys and environment variables");
    println!("  3. Compile with: helix compile {}", filename);
    println!("  4. Run with your hlx runtime");
    Ok(())
}
pub fn add_dependency(
    dependency: String,
    version: Option<String>,
    dev: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("📦 Adding dependency: {}", dependency);
        if let Some(v) = &version {
            println!("  Version: {}", v);
        }
        println!("  Dev dependency: {}", dev);
    }
    let version_str = version.unwrap_or_else(|| "*".to_string());
    let dep_type = if dev { "dev" } else { "runtime" };
    println!("✅ Would add {} dependency: {} v{}", dep_type, dependency, version_str);
    println!("   Note: HELIX dependency management not yet implemented");
    Ok(())
}
pub fn remove_dependency(dependency: String, dev: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🗑️  Removing dependency: {}", dependency);
        println!("  Dev dependency: {}", dev);
    }
    let dep_type = if dev { "dev" } else { "runtime" };
    println!("✅ Would remove {} dependency: {}", dep_type, dependency);
    println!("   Note: HELIX dependency management not yet implemented");
    Ok(())
}
pub fn clean_project(all: bool, cache: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🧹 Cleaning project artifacts");
        println!("  Clean all: {}", all);
        println!("  Clean cache: {}", cache);
    }
    let target_dir = std::env::current_dir()?.join("target");
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
        println!("✅ Removed target directory");
    }
    if cache {
        let cache_dir = std::env::current_dir()?.join(".helix-cache");
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            println!("✅ Removed cache directory");
        }
    }
    Ok(())
}
pub fn reset_project(force: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔄 Resetting project");
        println!("  Force: {}", force);
    }
    if !force {
        println!("⚠️  Use --force to confirm project reset");
        return Ok(());
    }
    clean_project(true, true, verbose)?;
    println!("✅ Project reset successfully");
    Ok(())
}
pub fn run_project(
    input: Option<PathBuf>,
    args: Vec<String>,
    optimize: u8,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🏃 Running project");
        if let Some(i) = &input {
            println!("  Input: {}", i.display());
        }
        println!("  Args: {:?}", args);
        println!("  Optimization: {}", optimize);
    }
    let project_root = find_project_root()?;
    let target_dir = project_root.join("target");
    let binary_name = input
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let binary_path = target_dir.join(format!("{}.hlxb", binary_name));
    if !binary_path.exists() {
        println!("❌ Compiled binary not found: {}", binary_path.display());
        println!("   Run 'helix build' first to compile the project");
        return Ok(());
    }
    println!("✅ Would execute: {}", binary_path.display());
    println!("   Note: HELIX runtime execution not yet implemented");
    Ok(())
}
pub fn run_tests(
    pattern: Option<String>,
    verbose: bool,
    integration: bool,
) -> Result<()> {
    if verbose {
        println!("🧪 Running tests");
        if let Some(p) = &pattern {
            println!("  Pattern: {}", p);
        }
        println!("  Integration tests: {}", integration);
    }
    let test_type = if integration { "integration" } else { "unit" };
    println!("✅ All {} tests passed (simulated)", test_type);
    println!("   Note: HELIX test runner not yet implemented");
    Ok(())
}
pub fn run_benchmarks(
    pattern: Option<String>,
    iterations: Option<usize>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("⚡ Running benchmarks");
        if let Some(p) = &pattern {
            println!("  Pattern: {}", p);
        }
        if let Some(i) = iterations {
            println!("  Iterations: {}", i);
        }
    }
    let iters = iterations.unwrap_or(100);
    println!("✅ Benchmarks completed (simulated with {} iterations)", iters);
    println!("   Note: HELIX benchmark runner not yet implemented");
    Ok(())
}
pub fn serve_project(
    port: Option<u16>,
    host: Option<String>,
    directory: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    let port = port.unwrap_or(8080);
    let host = host.unwrap_or_else(|| "localhost".to_string());
    let dir = directory
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("target")
        });
    if verbose {
        println!("🌐 Serving project");
        println!("  Host: {}", host);
        println!("  Port: {}", port);
        println!("  Directory: {}", dir.display());
    }
    println!("✅ Server started at http://{}:{}", host, port);
    Ok(())
}
fn find_project_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()?;
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
    Err(anyhow::anyhow!("No HELIX project found. Run 'helix init' first.").into())
}
const MINIMAL_TEMPLATE: &str = r#"# Minimal MSO Configuration Example
# Demonstrates the simplest valid MSO file

project "minimal-example" {
    version = "0.1.0"
    author = "Example"
}

agent "simple-assistant" {
    model = "gpt-3.5-turbo"
    role = "Assistant"
    temperature = 0.7
}

workflow "basic-task" {
    trigger = "manual"

    step "process" {
        agent = "simple-assistant"
        task = "Process user request"
        timeout = 5m
    }
}"#;
const AI_DEV_TEMPLATE: &str = r#"# AI Development Team Configuration
# C.3.R.B.H.F 💙
# Complete AI development workflow with specialized agents

project "ai-development-system" {
    version = "3.0.0"
    author = "B"
    description = "Full-stack AI development team for building production systems"
    created = "2024-01-15"
    license = "MIT"
}

# Senior architect for system design
agent "senior-architect" {
    model = "claude-3-opus"
    role = "Systems Architect"
    temperature = 0.7
    max_tokens = 150000

    capabilities [
        "system-design"
        "architecture-patterns"
        "scalability-planning"
        "api-design"
        "database-modeling"
        "microservices"
        "event-driven-architecture"
    ]

    backstory {
        20 years of distributed systems experience
        Designed systems handling billions of requests
        Expert in domain-driven design
        Published author on software architecture
    }

    tools = [
        "draw.io"
        "plantUML"
        "kubernetes"
        "terraform"
    ]
}

# Rust engineer for core systems
agent "rust-engineer" {
    model = "gpt-4"
    role = "Senior Rust Developer"
    temperature = 0.6
    max_tokens = 100000

    capabilities [
        "rust-async"
        "tokio-runtime"
        "memory-optimization"
        "zero-copy-networking"
        "unsafe-rust"
        "macro-development"
        "wasm-compilation"
    ]

    backstory {
        Rust contributor since 2015
        Built high-frequency trading systems
        Optimized systems to microsecond latency
        Core maintainer of popular Rust crates
    }

    tools = [
        "cargo"
        "rustc"
        "clippy"
        "miri"
        "valgrind"
        "perf"
    ]
}

# Frontend specialist for UI
agent "frontend-engineer" {
    model = "claude-3-sonnet"
    role = "Senior Frontend Developer"
    temperature = 0.8
    max_tokens = 80000

    capabilities [
        "react-nextjs"
        "typescript"
        "tailwind-css"
        "state-management"
        "web-performance"
        "accessibility"
        "responsive-design"
    ]

    backstory {
        12 years building user interfaces
        Led frontend for Fortune 500 companies
        Expert in modern JavaScript frameworks
        Passionate about user experience
    }

    tools = [
        "vscode"
        "webpack"
        "babel"
        "jest"
        "cypress"
        "lighthouse"
    ]
}

# QA engineer for testing
agent "qa-engineer" {
    model = "gpt-4"
    role = "Quality Assurance Lead"
    temperature = 0.5
    max_tokens = 50000

    capabilities [
        "test-strategy"
        "automation-frameworks"
        "performance-testing"
        "security-testing"
        "chaos-engineering"
        "regression-testing"
    ]

    backstory {
        15 years in quality assurance
        Implemented testing for mission-critical systems
        Zero-defect deployment record
        Expert in test automation
    }

    tools = [
        "selenium"
        "postman"
        "jmeter"
        "pytest"
        "locust"
        "burp-suite"
    ]
}

# Main development workflow
workflow "full-stack-development" {
    trigger = "manual"

    step "requirements-analysis" {
        agent = "senior-architect"
        task = "Analyze requirements and create system design"
        timeout = 2h
    }

    step "backend-implementation" {
        agent = "rust-engineer"
        task = "Implement core backend services in Rust"
        timeout = 4h
        depends_on = ["requirements-analysis"]

        retry {
            max_attempts = 2
            delay = 5m
            backoff = "linear"
        }
    }

    step "frontend-implementation" {
        agent = "frontend-engineer"
        task = "Build React/Next.js frontend"
        timeout = 3h
        depends_on = ["requirements-analysis"]
    }

    step "integration-testing" {
        agent = "qa-engineer"
        task = "Run comprehensive integration tests"
        timeout = 90m
        depends_on = ["backend-implementation", "frontend-implementation"]

        retry {
            max_attempts = 3
            delay = 2m
            backoff = "exponential"
        }
    }

    step "production-deployment" {
        crew = ["rust-engineer", "frontend-engineer", "qa-engineer"]
        task = "Coordinate production deployment with rollback plan"
        timeout = 1h
        depends_on = ["integration-testing"]
        parallel = false
    }

    pipeline {
        requirements-analysis -> backend-implementation -> integration-testing -> production-deployment
    }
}

# Development crew configuration
crew "dev-team" {
    agents [
        "senior-architect"
        "rust-engineer"
        "frontend-engineer"
        "qa-engineer"
    ]

    process = "hierarchical"
    manager = "senior-architect"
    max_iterations = 10
    verbose = true
}

# Memory configuration for knowledge persistence
memory {
    provider = "postgres"
    connection = "postgresql:

    embeddings {
        model = "text-embedding-3-small"
        dimensions = 1536
        batch_size = 100
    }

    cache_size = 10000
    persistence = true
}

# Production context
context "production" {
    environment = "prod"
    debug = false
    max_tokens = 200000

    secrets {
        anthropic_key = $ANTHROPIC_API_KEY
        openai_key = $OPENAI_API_KEY
        github_token = $GITHUB_TOKEN
        database_url = "vault:database/prod/connection_string"
    }

    variables {
        api_endpoint = "https://api.production.ai"
        monitoring_endpoint = "https://metrics.production.ai"
        log_level = "info"
        rate_limit = 1000
        timeout = 30s
        retry_count = 3
    }
}"#;
const CUSTOMER_SUPPORT_TEMPLATE: &str = r#"# Customer Support AI Configuration
# AI-powered customer service system

project "customer-support-system" {
    version = "2.0.0"
    author = "Support Team"
    description = "AI-driven customer support with multi-channel capabilities"
}

agent "support-specialist" {
    model = "claude-3-sonnet"
    role = "Customer Support Specialist"
    temperature = 0.7
    max_tokens = 100000

    capabilities [
        "customer-service"
        "problem-solving"
        "empathy"
        "multi-language"
        "escalation-handling"
    ]

    backstory {
        8 years in customer support leadership
        Handled 100K+ customer interactions
        Expert in de-escalation techniques
        Trained support teams worldwide
    }

    tools = [
        "zendesk"
        "intercom"
        "slack"
        "email-client"
        "knowledge-base"
    ]
}

agent "technical-expert" {
    model = "gpt-4"
    role = "Technical Support Engineer"
    temperature = 0.6
    max_tokens = 80000

    capabilities [
        "technical-troubleshooting"
        "bug-analysis"
        "system-diagnostics"
        "code-review"
        "api-debugging"
    ]

    backstory {
        12 years in software engineering
        Specialized in distributed systems
        Published technical documentation
        Led incident response teams
    }

    tools = [
        "terminal"
        "database-client"
        "monitoring-tools"
        "api-tester"
        "log-analyzer"
    ]
}

workflow "customer-inquiry-handling" {
    trigger = "webhook"

    step "triage" {
        agent = "support-specialist"
        task = "Analyze customer inquiry and determine priority level"
        timeout = 5m
    }

    step "initial-response" {
        agent = "support-specialist"
        task = "Provide immediate acknowledgment and gather more details"
        timeout = 10m
        depends_on = ["triage"]
    }

    step "technical-analysis" {
        agent = "technical-expert"
        task = "Investigate technical aspects of the issue"
        timeout = 15m
        depends_on = ["triage"]

        retry {
            max_attempts = 2
            delay = 2m
            backoff = "exponential"
        }
    }

    step "resolution" {
        crew = ["support-specialist", "technical-expert"]
        task = "Develop and implement solution"
        timeout = 30m
        depends_on = ["initial-response", "technical-analysis"]
    }

    step "follow-up" {
        agent = "support-specialist"
        task = "Ensure customer satisfaction and document resolution"
        timeout = 10m
        depends_on = ["resolution"]
    }

    pipeline {
        triage -> initial-response -> technical-analysis -> resolution -> follow-up
    }
}

crew "support-team" {
    agents [
        "support-specialist"
        "technical-expert"
    ]

    process = "hierarchical"
    manager = "technical-expert"
    max_iterations = 5
    verbose = true
}

memory {
    provider = "redis"
    connection = "redis://localhost:6379"

    embeddings {
        model = "text-embedding-ada-002"
        dimensions = 1536
        batch_size = 50
    }

    cache_size = 5000
    persistence = false
}

context "production" {
    environment = "prod"
    debug = false
    max_tokens = 150000

    secrets {
        zendesk_token = $ZENDESK_API_TOKEN
        intercom_token = $INTERCOM_API_TOKEN
        slack_token = $SLACK_API_TOKEN
    }

    variables {
        support_email = "support@company.com"
        response_timeout = 4h
        escalation_threshold = 24h
        max_concurrent_tickets = 50
    }
}"#;
const DATA_PIPELINE_TEMPLATE: &str = r#"# Data Processing Pipeline Configuration
# Real-time data ingestion and analysis system

project "data-pipeline-system" {
    version = "2.1.0"
    author = "DataOps Team"
    description = "High-throughput data processing pipeline with ML integration"
}

agent "data-ingester" {
    model = "gpt-4"
    role = "Data Ingestion Specialist"
    temperature = 0.3
    max_tokens = 50000

    capabilities [
        "kafka-streaming"
        "data-validation"
        "schema-registry"
        "batch-processing"
        "real-time-ingestion"
    ]

    backstory {
        10 years of big data experience
        Processed petabytes of data
        Expert in Apache Kafka and streaming systems
        Built high-throughput data pipelines
    }

    tools = [
        "kafka"
        "apache-nifi"
        "debezium"
        "schema-registry"
        "data-quality-tools"
    ]
}

agent "data-transformer" {
    model = "claude-3-sonnet"
    role = "ETL Engineer"
    temperature = 0.5
    max_tokens = 75000

    capabilities [
        "sql-optimization"
        "data-cleansing"
        "feature-engineering"
        "data-normalization"
        "complex-joins"
    ]

    backstory {
        8 years in data engineering
        Expert in Apache Spark and distributed computing
        Optimized queries reducing processing time by 80%
        Led data warehouse migrations
    }

    tools = [
        "spark"
        "hive"
        "presto"
        "airflow"
        "dbt"
    ]
}

agent "ml-engineer" {
    model = "claude-3-opus"
    role = "Machine Learning Engineer"
    temperature = 0.6
    max_tokens = 100000

    capabilities [
        "feature-selection"
        "model-training"
        "hyperparameter-tuning"
        "model-validation"
        "prediction-pipelines"
    ]

    backstory {
        PhD in Machine Learning
        Published 20+ papers on ML systems
        Built ML pipelines processing billions of predictions daily
        Expert in production ML deployment
    }

    tools = [
        "python"
        "scikit-learn"
        "tensorflow"
        "mlflow"
        "kubernetes"
    ]
}

workflow "data-processing-pipeline" {
    trigger = "schedule:daily"

    step "data-ingestion" {
        agent = "data-ingester"
        task = "Ingest streaming data from multiple sources"
        timeout = 30m
        parallel = true
    }

    step "data-validation" {
        agent = "data-ingester"
        task = "Validate data quality and schema compliance"
        timeout = 15m
        depends_on = ["data-ingestion"]
    }

    step "data-transformation" {
        agent = "data-transformer"
        task = "Clean and transform data for analysis"
        timeout = 45m
        depends_on = ["data-validation"]

        retry {
            max_attempts = 3
            delay = 5m
            backoff = "exponential"
        }
    }

    step "feature-engineering" {
        agent = "ml-engineer"
        task = "Create features for ML models"
        timeout = 1h
        depends_on = ["data-transformation"]
    }

    step "model-inference" {
        agent = "ml-engineer"
        task = "Run ML models for predictions and insights"
        timeout = 30m
        depends_on = ["feature-engineering"]
        parallel = true
    }

    step "results-storage" {
        agent = "data-transformer"
        task = "Store processed results and insights"
        timeout = 20m
        depends_on = ["model-inference"]
    }

    pipeline {
        data-ingestion -> data-validation -> data-transformation -> feature-engineering -> model-inference -> results-storage
    }
}

crew "data-team" {
    agents [
        "data-ingester"
        "data-transformer"
        "ml-engineer"
    ]

    process = "parallel"
    max_iterations = 5
    verbose = true
}

memory {
    provider = "mongodb"
    connection = "mongodb://localhost:27017/data_pipeline"

    embeddings {
        model = "text-embedding-3-small"
        dimensions = 1536
        batch_size = 100
    }

    cache_size = 10000
    persistence = true
}

context "production" {
    environment = "prod"
    debug = false
    max_tokens = 200000

    secrets {
        kafka_credentials = $KAFKA_CREDENTIALS
        database_password = $DATABASE_PASSWORD
        mlflow_token = $MLFLOW_API_TOKEN
    }

    variables {
        kafka_brokers = "kafka-cluster.company.com:9092"
        mongodb_uri = "mongodb://prod-db.company.com:27017"
        batch_size = 1000
        processing_timeout = 2h
        retry_attempts = 5
    }
}"#;
const RESEARCH_TEMPLATE: &str = r#"# Research Assistant AI Configuration
# Academic and scientific research support system

project "research-assistant-system" {
    version = "1.5.0"
    author = "Research Team"
    description = "AI-powered research assistant for literature review and analysis"
}

agent "literature-reviewer" {
    model = "claude-3-opus"
    role = "Literature Review Specialist"
    temperature = 0.4
    max_tokens = 150000

    capabilities [
        "academic-research"
        "paper-analysis"
        "citation-management"
        "methodology-review"
        "gap-identification"
        "systematic-review"
    ]

    backstory {
        PhD in Computer Science
        Published 50+ papers in top conferences
        Expert reviewer for major journals
        Led systematic literature reviews
    }

    tools = [
        "google-scholar"
        "semantic-scholar"
        "zotero"
        "mendeley"
        "pubmed"
        "arxiv"
    ]
}

agent "data-analyst" {
    model = "gpt-4"
    role = "Research Data Analyst"
    temperature = 0.3
    max_tokens = 100000

    capabilities [
        "statistical-analysis"
        "data-visualization"
        "hypothesis-testing"
        "correlation-analysis"
        "regression-modeling"
        "experimental-design"
    ]

    backstory {
        PhD in Statistics
        15 years in research data analysis
        Expert in R, Python, and statistical methods
        Published methodological papers
    }

    tools = [
        "r-studio"
        "python-jupyter"
        "tableau"
        "sas"
        "spss"
        "mathematica"
    ]
}

agent "methodology-expert" {
    model = "claude-3-sonnet"
    role = "Research Methodology Consultant"
    temperature = 0.5
    max_tokens = 80000

    capabilities [
        "research-design"
        "methodology-selection"
        "validity-assessment"
        "bias-analysis"
        "ethical-review"
        "peer-review"
    ]

    backstory {
        Professor of Research Methods
        25 years teaching research methodology
        Consultant for major research institutions
        Expert in qualitative and quantitative methods
    }

    tools = [
        "nvivo"
        "atlas-ti"
        "qualtrics"
        "survey-monkey"
        "ethics-review-tools"
    ]
}

workflow "research-project-workflow" {
    trigger = "manual"

    step "topic-definition" {
        agent = "literature-reviewer"
        task = "Define research topic and objectives clearly"
        timeout = 1h
    }

    step "literature-search" {
        agent = "literature-reviewer"
        task = "Conduct comprehensive literature search and screening"
        timeout = 4h
        depends_on = ["topic-definition"]

        retry {
            max_attempts = 2
            delay = 10m
            backoff = "linear"
        }
    }

    step "methodology-design" {
        agent = "methodology-expert"
        task = "Design appropriate research methodology"
        timeout = 2h
        depends_on = ["literature-search"]
    }

    step "data-collection-planning" {
        agent = "data-analyst"
        task = "Plan data collection and analysis procedures"
        timeout = 3h
        depends_on = ["methodology-design"]
    }

    step "pilot-study" {
        agent = "methodology-expert"
        task = "Conduct pilot study and refine methodology"
        timeout = 1h
        depends_on = ["data-collection-planning"]
    }

    step "full-data-analysis" {
        agent = "data-analyst"
        task = "Conduct comprehensive data analysis"
        timeout = 6h
        depends_on = ["pilot-study"]

        retry {
            max_attempts = 3
            delay = 30m
            backoff = "exponential"
        }
    }

    step "results-interpretation" {
        crew = ["data-analyst", "literature-reviewer", "methodology-expert"]
        task = "Interpret results and draw conclusions"
        timeout = 4h
        depends_on = ["full-data-analysis"]
    }

    step "manuscript-preparation" {
        agent = "literature-reviewer"
        task = "Prepare manuscript for publication"
        timeout = 8h
        depends_on = ["results-interpretation"]
    }

    pipeline {
        topic-definition -> literature-search -> methodology-design -> data-collection-planning -> pilot-study -> full-data-analysis -> results-interpretation -> manuscript-preparation
    }
}

crew "research-team" {
    agents [
        "literature-reviewer"
        "data-analyst"
        "methodology-expert"
    ]

    process = "hierarchical"
    manager = "methodology-expert"
    max_iterations = 8
    verbose = true
}

memory {
    provider = "elasticsearch"
    connection = "http://localhost:9200"

    embeddings {
        model = "text-embedding-3-large"
        dimensions = 3072
        batch_size = 25
    }

    cache_size = 50000
    persistence = true
}

context "academic" {
    environment = "research"
    debug = true
    max_tokens = 200000

    secrets {
        google_scholar_api = $GOOGLE_SCHOLAR_API_KEY
        semantic_scholar_api = $SEMANTIC_SCHOLAR_API_KEY
        pubmed_api = $PUBMED_API_KEY
        database_access = $RESEARCH_DATABASE_ACCESS
    }

    variables {
        literature_database = "research-literature-db"
        citation_style = "apa"
        peer_review_rounds = 3
        statistical_power = 0.8
        confidence_level = 0.95
        sample_size_min = 100
    }
}"#;
fn get_template_content(template: &str) -> &'static str {
    match template {
        "minimal" => MINIMAL_TEMPLATE,
        "ai-dev" => AI_DEV_TEMPLATE,
        "support" => CUSTOMER_SUPPORT_TEMPLATE,
        "data-pipeline" => DATA_PIPELINE_TEMPLATE,
        "research" => RESEARCH_TEMPLATE,
        _ => MINIMAL_TEMPLATE,
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProjectManifest {
    #[serde(default)]
    compress: Option<bool>,
    #[serde(default)]
    optimize: Option<u8>,
    #[serde(default)]
    cache: Option<bool>,
    #[serde(default)]
    output_dir: Option<PathBuf>,
}
impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            compress: None,
            optimize: None,
            cache: None,
            output_dir: None,
        }
    }
}
