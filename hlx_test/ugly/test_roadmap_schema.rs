// Auto-generated Helix SDK for Rust
use std::collections::HashMap;

pub struct HelixConfig {
    data: HashMap<String, serde_json::Value>,
}

impl HelixConfig {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_string(&content)
    }

    pub fn from_string(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data: HashMap<String, serde_json::Value> = serde_json::from_str(content)?;
        Ok(Self { data })
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }

    pub fn process(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Process the configuration
        println!("Processing Helix configuration...");
        Ok(())
    }

    pub fn compile(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Compile the configuration
        println!("Compiling Helix configuration...");
        let json = serde_json::to_vec(&self.data)?;
        Ok(json)
    }
}

impl std::ops::Index<&str> for HelixConfig {
    type Output = serde_json::Value;

    fn index(&self, key: &str) -> &Self::Output {
        self.data.get(key).unwrap_or(&serde_json::Value::Null)
    }
}
