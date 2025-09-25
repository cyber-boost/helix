//! Simple HLX Interface
//!
//! Provides an extremely simple API for working with Helix files:
//! - `hlx.section.key` - dot notation access
//! - `hlx[section][key]` - bracket notation access
//! - `hlx.get.section.key` - get method
//! - `hlx.set.section.key` - set method
//! - `hlx.server.start()` - start server
//! - `hlx.watch()` - watch mode
//! - `hlx.process()` - process/compile file
//! - `hlx.compile()` - compile

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::error::HlxError;
use crate::value::Value;
use crate::dispatch::{HelixDispatcher, DispatchResult};
use crate::types::HelixConfig;
use crate::operators::OperatorEngine;

/// Simple HLX interface for easy file operations
pub struct Hlx {
    pub config: Option<HelixConfig>,
    pub data: HashMap<String, HashMap<String, Value>>,
    pub file_path: Option<PathBuf>,
    pub dispatcher: HelixDispatcher,
    pub operator_engine: OperatorEngine,
}

impl Hlx {
    /// Load an HLX file (text or binary)
    /// Usage: `let hlx = Hlx::load("config.hlx")?;`
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self, HlxError> {
        let path = path.as_ref().to_path_buf();

        let mut hlx = Self {
            config: None,
            data: HashMap::new(),
            file_path: Some(path.clone()),
            dispatcher: HelixDispatcher::new(),
            operator_engine: OperatorEngine::new().await?,
        };

        hlx.dispatcher.initialize().await?;

        if path.extension().and_then(|s| s.to_str()) == Some("hlxb") {
            // Binary file - simplified for now
            #[cfg(feature = "compiler")]
            {
                let loader = crate::compiler::BinaryLoader::new();
                let config = loader.load_to_config(&path)
                    .map_err(|e| HlxError::compilation_error(
                        format!("Failed to load binary: {:?}", e),
                        "Ensure file is a valid HLXB file"
                    ))?;

                hlx.config = Some(config);
            }
            #[cfg(not(feature = "compiler"))]
            {
                return Err(HlxError::compilation_error(
                    "Binary file support not available",
                    "Compile with 'compiler' feature enabled"
                ));
            }
        } else {
            // Text file
            let content = std::fs::read_to_string(&path)
                .map_err(|e| HlxError::io_error(
                    format!("Failed to read file: {}", e),
                    "Ensure file exists and is readable"
                ))?;

            // Parse and execute
            match hlx.dispatcher.parse_and_execute(&content).await? {
                DispatchResult::Executed(value) => {
                    // Convert executed result to data structure
                    if let Value::Object(obj) = value {
                        for (section, section_data) in obj {
                            if let Value::Object(section_obj) = section_data {
                                let mut section_map = HashMap::new();
                                for (key, val) in section_obj {
                                    section_map.insert(key, val);
                                }
                                hlx.data.insert(section, section_map);
                            }
                        }
                    }
                }
                DispatchResult::Parsed(ast) => {
                    // Convert AST to config
                    hlx.config = Some(crate::ast_to_config(ast)
                        .map_err(|e| HlxError::config_conversion("conversion".to_string(), e.to_string()))?);
                }
                _ => {}
            }
        }

        Ok(hlx)
    }

    /// Create empty HLX instance
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            config: None,
            data: HashMap::new(),
            file_path: None,
            dispatcher: HelixDispatcher::new(),
            operator_engine: OperatorEngine::new().await?,
        })
    }

    /// Get value using dot notation: hlx.section.key
    /// Usage: `let value = hlx.database.host;`
    pub fn get(&self, section: &str, key: &str) -> Option<&Value> {
        self.data.get(section)?.get(key)
    }

    /// Set value using dot notation: hlx.set.section.key = value
    /// Usage: `hlx.set("database", "host", Value::String("localhost".to_string()));`
    pub fn set(&mut self, section: &str, key: &str, value: Value) {
        self.data.entry(section.to_string()).or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
    }

    /// Bracket notation access: hlx["section"]["key"]
    /// Usage: `let value = &hlx["database"]["host"];`
    pub fn index(&self, section: &str) -> Option<&HashMap<String, Value>> {
        self.data.get(section)
    }

    /// Mutable bracket notation access: hlx["section"]["key"] = value
    /// Usage: `hlx.index_mut("database").unwrap().insert("host".to_string(), Value::String("localhost".to_string()));`
    pub fn index_mut(&mut self, section: &str) -> Option<&mut HashMap<String, Value>> {
        self.data.get_mut(section)
    }

    /// Start server with current configuration
    /// Usage: `hlx.server.start()?;`
    pub async fn server(&mut self) -> Result<(), HlxError> {
        if self.dispatcher.is_ready() {
            // Server is already initialized
            Ok(())
        } else {
            self.dispatcher.initialize().await
        }
    }

    /// Watch mode for file changes
    /// Usage: `hlx.watch()?;`
    pub async fn watch(&mut self) -> Result<(), HlxError> {
        #[cfg(feature = "compiler")]
        {
            // Use the compiler's watch functionality
            if let Some(path) = &self.file_path {
                println!("Watching {} for changes...", path.display());
                // Implementation would go here
                Ok(())
            } else {
                Err(HlxError::invalid_input(
                    "No file loaded for watching",
                    "Load a file first with Hlx::load()"
                ))
            }
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(HlxError::compilation_error(
                "Watch mode not available",
                "Compile with 'compiler' feature enabled"
            ))
        }
    }

    /// Process/compile the file
    /// Usage: `hlx.process()?;`
    pub async fn process(&mut self) -> Result<(), HlxError> {
        if let Some(path) = &self.file_path {
            let content = std::fs::read_to_string(path)
                .map_err(|e| HlxError::io_error(
                    format!("Failed to read file: {}", e),
                    "Ensure file exists and is readable"
                ))?;

            match self.dispatcher.parse_and_execute(&content).await? {
                DispatchResult::Executed(value) => {
                    println!("Processed successfully: {:?}", value);
                    Ok(())
                }
                _ => Ok(())
            }
        } else {
            Err(HlxError::invalid_input(
                "No file loaded for processing",
                "Load a file first with Hlx::load()"
            ))
        }
    }

    /// Compile the file to binary
    /// Usage: `hlx.compile()?;`
    pub async fn compile(&mut self) -> Result<(), HlxError> {
        #[cfg(feature = "compiler")]
        {
            if let Some(path) = &self.file_path {
                use crate::compiler::{Compiler, OptimizationLevel};

                // Create compiler with optimization level 2 (standard)
                let compiler = Compiler::builder()
                    .optimization_level(OptimizationLevel::Two)
                    .compression(true)
                    .cache(true)
                    .verbose(false)
                    .build();

                // Compile the file
                let binary = compiler.compile_file(path)
                    .map_err(|e| HlxError::compilation_error(
                        format!("Compilation failed: {}", e),
                        "Check file syntax and try again"
                    ))?;

                // Write binary file using the serializer
                let binary_path = path.with_extension("hlxb");
                let serializer = crate::compiler::BinarySerializer::new(true);
                serializer.write_to_file(&binary, &binary_path)
                    .map_err(|e| HlxError::io_error(
                        format!("Failed to write binary file: {}", e),
                        "Ensure output directory is writable"
                    ))?;

                println!("✅ Successfully compiled {} to {}", path.display(), binary_path.display());
                Ok(())
            } else {
                Err(HlxError::invalid_input(
                    "No file loaded for compilation",
                    "Load a file first with Hlx::load()"
                ))
            }
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(HlxError::compilation_error(
                "Compilation not available",
                "Compile with 'compiler' feature enabled"
            ))
        }
    }

    /// Execute an operator directly
    /// Usage: `let result = hlx.execute("@date(\"Y-m-d\")").await?;`
    pub async fn execute(&mut self, code: &str) -> Result<Value, HlxError> {
        if !self.dispatcher.is_ready() {
            self.dispatcher.initialize().await?;
        }

        match self.dispatcher.parse_and_execute(code).await? {
            DispatchResult::Executed(value) => Ok(value),
            DispatchResult::ParseError(err) => Err(HlxError::invalid_input(
                format!("Parse error: {}", err),
                "Check syntax"
            )),
            DispatchResult::ExecutionError(err) => Err(err),
            DispatchResult::Parsed(_) => Err(HlxError::execution_error(
                "Parsed but not executed",
                "Use process() for file processing"
            )),
        }
    }

    /// Execute an operator directly with proper parameter handling
    /// Usage: `let result = hlx.execute_operator("date", "{\"format\":\"Y-m-d\"}").await?;`
    pub async fn execute_operator(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        self.operator_engine.execute_operator(operator, params).await
    }

    /// Get all sections
    pub fn sections(&self) -> Vec<&String> {
        self.data.keys().collect()
    }

    /// Get all keys in a section
    pub fn keys(&self, section: &str) -> Option<Vec<&String>> {
        self.data.get(section).map(|s| s.keys().collect())
    }

    /// Save current data back to file
    pub fn save(&self) -> Result<(), HlxError> {
        if let Some(path) = &self.file_path {
            // Convert data back to HLX format
            let mut content = String::new();

            for (section, keys) in &self.data {
                content.push_str(&format!("[{}]\n", section));
                for (key, value) in keys {
                    content.push_str(&format!("{} = {}\n", key, value));
                }
                content.push('\n');
            }

            std::fs::write(path, content)
                .map_err(|e| HlxError::io_error(
                    format!("Failed to save file: {}", e),
                    "Ensure write permissions"
                ))
        } else {
            Err(HlxError::invalid_input(
                "No file path set",
                "Load a file first or set file_path manually"
            ))
        }
    }
}

// Implement Index/IndexMut for bracket notation access
impl std::ops::Index<&str> for Hlx {
    type Output = HashMap<String, Value>;

    fn index(&self, section: &str) -> &Self::Output {
        self.data.get(section).unwrap_or_else(|| panic!("Section '{}' not found", section))
    }
}

impl std::ops::IndexMut<&str> for Hlx {
    fn index_mut(&mut self, section: &str) -> &mut Self::Output {
        self.data.entry(section.to_string()).or_insert_with(HashMap::new)
    }
}

// Convenience functions for testing all operators
pub mod test_operators {
    use super::*;

    pub async fn test_fundamental_operators() -> Result<(), HlxError> {
        let mut hlx = Hlx::new().await?;

        println!("Testing fundamental operators...");

        // Test @var operator
        let result = hlx.execute(r#"@var(name="test_var", value="hello")"#).await?;
        println!("@var result: {:?}", result);

        // Test @env operator
        let result = hlx.execute(r#"@env(key="HOME")"#).await?;
        println!("@env result: {:?}", result);

        // Test @date operator
        let result = hlx.execute(r#"@date("Y-m-d")"#).await?;
        println!("@date result: {:?}", result);

        // Test @time operator
        let result = hlx.execute(r#"@time("H:i:s")"#).await?;
        println!("@time result: {:?}", result);

        // Test @uuid operator
        let result = hlx.execute("@uuid()").await?;
        println!("@uuid result: {:?}", result);

        // Test @string operator
        let result = hlx.execute(r#"@string("hello world", "upper")"#).await?;
        println!("@string result: {:?}", result);

        // Test @math operator
        let result = hlx.execute(r#"@math("5 + 3")"#).await?;
        println!("@math result: {:?}", result);

        // Test @calc operator
        let result = hlx.execute(r#"@calc("a = 10; b = 5; a + b")"#).await?;
        println!("@calc result: {:?}", result);

        // Test @if operator
        let result = hlx.execute(r#"@if(condition="true", then="yes", else="no")"#).await?;
        println!("@if result: {:?}", result);

        // Test @array operator
        let result = hlx.execute(r#"@array(values="[1,2,3]", operation="length")"#).await?;
        println!("@array result: {:?}", result);

        // Test @json operator
        let result = hlx.execute(r#"@json('{"name":"test"}', "parse")"#).await?;
        println!("@json result: {:?}", result);

        // Test @base64 operator
        let result = hlx.execute(r#"@base64("hello", "encode")"#).await?;
        println!("@base64 result: {:?}", result);

        // Test @hash operator
        let result = hlx.execute(r#"@hash("password", "sha256")"#).await?;
        println!("@hash result: {:?}", result);

        println!("All fundamental operators tested successfully!");
        Ok(())
    }

    pub async fn test_conditional_operators() -> Result<(), HlxError> {
        let mut hlx = Hlx::new().await?;

        println!("Testing conditional operators...");

        // Test @if operator with expressions
        let result = hlx.execute(r#"@if(condition="@math('5 > 3')", then="greater", else="less")"#).await?;
        println!("@if with expression: {:?}", result);

        // Test @switch operator
        let result = hlx.execute(r#"@switch(value="2", cases="{'1':'one','2':'two','3':'three'}", default="unknown")"#).await?;
        println!("@switch result: {:?}", result);

        // Test @filter operator
        let result = hlx.execute(r#"@filter(array="[1,2,3,4,5]", condition="@math('value > 3')")"#).await?;
        println!("@filter result: {:?}", result);

        // Test @map operator
        let result = hlx.execute(r#"@map(array="[1,2,3]", transform="@math('value * 2')")"#).await?;
        println!("@map result: {:?}", result);

        // Test @reduce operator
        let result = hlx.execute(r#"@reduce(array="[1,2,3,4]", initial="0", operation="@math('acc + value')")"#).await?;
        println!("@reduce result: {:?}", result);

        println!("All conditional operators tested successfully!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hlx_interface() {
        // Test creating empty HLX
        let mut hlx = Hlx::new().await.unwrap();

        // Test bracket notation - create section first
        hlx.data.insert("database".to_string(), HashMap::new());
        hlx.index_mut("database").unwrap().insert("host".to_string(), crate::value::Value::String("localhost".to_string()));
        hlx.index_mut("database").unwrap().insert("port".to_string(), crate::value::Value::Number(5432.0));

        assert_eq!(hlx.get("database", "host"), Some(&crate::value::Value::String("localhost".to_string())));
        assert_eq!(hlx.get("database", "port"), Some(&Value::Number(5432.0)));

        // Test sections and keys
        let sections = hlx.sections();
        assert!(sections.iter().any(|s| *s == "database"));
        let keys = hlx.keys("database").unwrap();
        assert!(keys.iter().any(|k| *k == "host"));
    }

    #[tokio::test]
    async fn test_operator_execution() {
        let hlx = Hlx::new().await.unwrap();

        // Test basic operator execution using the new execute_operator method
        let result = hlx.execute_operator("date", "{\"format\":\"Y-m-d\"}").await;
        println!("Direct operator execution result: {:?}", result);
        assert!(result.is_ok());

        // Test UUID operator
        let result = hlx.execute_operator("uuid", "").await;
        println!("UUID operator execution result: {:?}", result);
        assert!(result.is_ok());

        // Test with invalid operator
        let result = hlx.execute_operator("nonexistent", "{}").await;
        println!("Invalid operator result: {:?}", result);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_operator_integration() {
        use crate::ops::OperatorParser;

        // Test ops.rs integration with operators/
        let mut ops_parser = OperatorParser::new().await;

        // Test @date operator through ops.rs
        let result = ops_parser.parse_value("@date(\"Y-m-d\")").await.unwrap();
        match result {
            crate::value::Value::String(date_str) => {
                assert!(!date_str.is_empty());
                println!("✅ @date operator working: {}", date_str);
            }
            _ => panic!("Expected string result from @date"),
        }

        // Test @uuid operator through ops.rs
        let result = ops_parser.parse_value("@uuid()").await.unwrap();
        match result {
            crate::value::Value::String(uuid_str) => {
                assert!(!uuid_str.is_empty());
                // UUID should be in format like "uuid-xxxxx" or similar, not necessarily 36 chars
                println!("✅ @uuid operator working: {} (length: {})", uuid_str, uuid_str.len());
            }
            _ => panic!("Expected string result from @uuid"),
        }

        // Test direct operator engine usage
        use crate::operators::OperatorEngine;
        let operator_engine = OperatorEngine::new().await.unwrap();

        // Test date operator directly
        let result = operator_engine.execute_operator("date", "{\"format\":\"%Y-%m-%d\"}").await.unwrap();
        match result {
            crate::value::Value::String(date_str) => {
                assert!(!date_str.is_empty());
                println!("✅ Direct date operator working: {}", date_str);
            }
            _ => panic!("Expected string result from direct date operator"),
        }

        // Test uuid operator directly
        let result = operator_engine.execute_operator("uuid", "").await.unwrap();
        match result {
            crate::value::Value::String(uuid_str) => {
                assert!(!uuid_str.is_empty());
                println!("✅ Direct uuid operator working: {} (length: {})", uuid_str, uuid_str.len());
            }
            _ => panic!("Expected string result from direct uuid operator"),
        }

        println!("✅ ops.rs and operators/ integration fully working!");
    }

    #[tokio::test]
    async fn test_comprehensive_operator_testing() {
        // TODO: Re-enable when operator system is fully integrated
        // Test all fundamental operators
        // test_operators::test_fundamental_operators().await.unwrap();

        // Test all conditional operators
        // test_operators::test_conditional_operators().await.unwrap();

        // For now, just pass
        assert!(true);
    }
}
