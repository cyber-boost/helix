use std::path::PathBuf;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum CaptionAction {
    Process {
        files: Vec<PathBuf>,
        output: Option<PathBuf>,
        config: Option<Value>,
    },
    E621 {
        files: Vec<PathBuf>,
        output: Option<PathBuf>,
        filter_tags: String,
        format: Option<String>,
    },
    Convert {
        input: PathBuf,
        output: Option<PathBuf>,
        format: Option<String>,
    },
    Preview {
        file: PathBuf,
        format: Option<String>,
        rows: Option<usize>,
        columns: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone)]
pub enum JsonAction {
    Format {
        files: Vec<PathBuf>,
        check: bool,
    },
    Validate {
        files: Vec<PathBuf>,
        schema: Option<PathBuf>,
    },
    Metadata {
        files: Vec<PathBuf>,
        output: Option<PathBuf>,
    },
    Split {
        file: PathBuf,
        output: Option<PathBuf>,
    },
    Merge {
        files: Vec<PathBuf>,
        output: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct E621Config {
    pub filter_tags: String,
    pub format: Option<String>,
}

impl E621Config {
    pub fn new() -> Self {
        Self {
            filter_tags: String::new(),
            format: None,
        }
    }

    pub fn with_filter_tags(mut self, filter_tags: String) -> Self {
        self.filter_tags = filter_tags;
        self
    }

    pub fn with_format(mut self, format: Option<String>) -> Self {
        self.format = format;
        self
    }
}

pub async fn process_file(_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Placeholder implementation
    Ok(())
}

pub async fn process_e621_json_file(
    _file: &PathBuf,
    _config: Option<E621Config>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Placeholder implementation
    Ok(())
}
