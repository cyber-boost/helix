use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub role: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEntry {
    pub user: String,
    pub reasoning: String,
    pub assistant: String,
    pub template: String,
    pub conversations: Vec<Message>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningDataset {
    pub entries: Vec<ReasoningEntry>,
}
impl ReasoningDataset {
    #[must_use]
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let dataset: ReasoningDataset = serde_json::from_str(&content)?;
        Ok(dataset)
    }
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    pub fn add_entry(&mut self, entry: ReasoningEntry) {
        self.entries.push(entry);
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    #[must_use]
    pub fn create_template(user: &str, reasoning: &str, assistant: &str) -> String {
        format!(
            "<|im_start|>user\n{user}<|im_end|>\n<|im_start|>reasoning\n{reasoning}<|im_end|>\n<|im_start|>assistant\n{assistant}<|im_end|>",
        )
    }
}
impl Default for ReasoningDataset {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    #[tokio::test]
    async fn test_dataset_operations() -> Result<()> {
        let mut dataset = ReasoningDataset::new();
        let entry = ReasoningEntry {
            user: "What motivates Luna?".to_string(),
            reasoning: "Luna's motivations can be analyzed...".to_string(),
            assistant: "Luna is motivated by acceptance and self-expression."
                .to_string(),
            template: ReasoningDataset::create_template(
                "What motivates Luna?",
                "Luna's motivations can be analyzed...",
                "Luna is motivated by acceptance and self-expression.",
            ),
            conversations: vec![
                Message { content : "What motivates Luna?".to_string(), role : "user"
                .to_string(), }, Message { content :
                "Luna's motivations can be analyzed...".to_string(), role : "reasoning"
                .to_string(), }, Message { content :
                "Luna is motivated by acceptance and self-expression.".to_string(), role
                : "assistant".to_string(), },
            ],
        };
        dataset.add_entry(entry);
        assert_eq!(dataset.len(), 1);
        let temp_file = NamedTempFile::new()?;
        dataset.save(temp_file.path()).await?;
        let loaded_dataset = ReasoningDataset::load(temp_file.path()).await?;
        assert_eq!(loaded_dataset.len(), 1);
        Ok(())
    }
}