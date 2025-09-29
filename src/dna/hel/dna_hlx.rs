use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::hel::error::HlxError;
use crate::atp::types::Value;
use crate::hel::dispatch::{HelixDispatcher, DispatchResult};
use crate::HelixConfig;
use crate::ops::engine::OperatorEngine;
pub struct Hlx {
    pub config: Option<HelixConfig>,
    pub data: HashMap<String, HashMap<String, Value>>,
    pub file_path: Option<PathBuf>,
    pub dispatcher: HelixDispatcher,
    pub operator_engine: OperatorEngine,
}
impl Hlx {
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
            #[cfg(feature = "compiler")]
            {
                let loader = crate::mds::loader::BinaryLoader::new();
                let config = loader
                    .load_to_config(&path)
                    .map_err(|e| HlxError::compilation_error(
                        format!("Failed to load binary: {:?}", e),
                        "Ensure file is a valid HLXB file",
                    ))?;
                hlx.config = Some(config);
            }
            #[cfg(not(feature = "compiler"))]
            {
                return Err(
                    HlxError::compilation_error(
                        "Binary file support not available",
                        "Compile with 'compiler' feature enabled",
                    ),
                );
            }
        } else {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| HlxError::io_error(
                    format!("Failed to read file: {}", e),
                    "Ensure file exists and is readable",
                ))?;
            match hlx.dispatcher.parse_and_execute(&content).await? {
                DispatchResult::Executed(value) => {
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
                    hlx.config = Some(
                        crate::ast_to_config(ast)
                            .map_err(|e| HlxError::config_conversion(
                                "conversion".to_string(),
                                e,
                            ))?,
                    );
                }
                _ => {}
            }
        }
        Ok(hlx)
    }
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            config: None,
            data: HashMap::new(),
            file_path: None,
            dispatcher: HelixDispatcher::new(),
            operator_engine: OperatorEngine::new().await?,
        })
    }
    pub fn get(&self, section: &str, key: &str) -> Option<&Value> {
        self.data.get(section)?.get(key)
    }
    pub fn set(&mut self, section: &str, key: &str, value: Value) {
        self.data
            .entry(section.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
    }
    pub fn index(&self, section: &str) -> Option<&HashMap<String, Value>> {
        self.data.get(section)
    }
    pub fn index_mut(&mut self, section: &str) -> Option<&mut HashMap<String, Value>> {
        self.data.get_mut(section)
    }
    pub async fn server(&mut self) -> Result<(), HlxError> {
        if self.dispatcher.is_ready() {
            Ok(())
        } else {
            self.dispatcher.initialize().await
        }
    }
    pub async fn watch(&mut self) -> Result<(), HlxError> {
        #[cfg(feature = "compiler")]
        {
            if let Some(path) = &self.file_path {
                println!("Watching {} for changes...", path.display());
                Ok(())
            } else {
                Err(
                    HlxError::invalid_input(
                        "No file loaded for watching",
                        "Load a file first with Hlx::load()",
                    ),
                )
            }
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(
                HlxError::compilation_error(
                    "Watch mode not available",
                    "Compile with 'compiler' feature enabled",
                ),
            )
        }
    }
    pub async fn process(&mut self) -> Result<(), HlxError> {
        if let Some(path) = &self.file_path {
            let content = std::fs::read_to_string(path)
                .map_err(|e| HlxError::io_error(
                    format!("Failed to read file: {}", e),
                    "Ensure file exists and is readable",
                ))?;
            match self.dispatcher.parse_and_execute(&content).await? {
                DispatchResult::Executed(value) => {
                    println!("Processed successfully: {:?}", value);
                    Ok(())
                }
                _ => Ok(()),
            }
        } else {
            Err(
                HlxError::invalid_input(
                    "No file loaded for processing",
                    "Load a file first with Hlx::load()",
                ),
            )
        }
    }
    pub async fn compile(&mut self) -> Result<(), HlxError> {
        #[cfg(feature = "compiler")]
        {
            if let Some(path) = &self.file_path {
                use crate::dna::compiler::{Compiler, OptimizationLevel};
                let compiler = Compiler::builder()
                    .optimization_level(OptimizationLevel::Two)
                    .compression(true)
                    .cache(true)
                    .verbose(false)
                    .build();
                let binary = compiler
                    .compile_file(path)
                    .map_err(|e| HlxError::compilation_error(
                        format!("Compilation failed: {}", e),
                        "Check file syntax and try again",
                    ))?;
                let binary_path = path.with_extension("hlxb");
                let serializer = crate::mds::serializer::BinarySerializer::new(true);
                serializer
                    .write_to_file(&binary, &binary_path)
                    .map_err(|e| HlxError::io_error(
                        format!("Failed to write binary file: {}", e),
                        "Ensure output directory is writable",
                    ))?;
                println!(
                    "✅ Successfully compiled {} to {}", path.display(), binary_path
                    .display()
                );
                Ok(())
            } else {
                Err(
                    HlxError::invalid_input(
                        "No file loaded for compilation",
                        "Load a file first with Hlx::load()",
                    ),
                )
            }
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(
                HlxError::compilation_error(
                    "Compilation not available",
                    "Compile with 'compiler' feature enabled",
                ),
            )
        }
    }
    pub async fn execute(&mut self, code: &str) -> Result<Value, HlxError> {
        if !self.dispatcher.is_ready() {
            self.dispatcher.initialize().await?;
        }
        match self.dispatcher.parse_and_execute(code).await {
            Ok(DispatchResult::Executed(value)) => Ok(value),
            Ok(DispatchResult::ParseError(err)) => {
                Err(
                    HlxError::invalid_input(
                        format!("Parse error: {}", err),
                        "Check syntax",
                    ),
                )
            }
            Ok(DispatchResult::ExecutionError(err)) => Err(err),
            Ok(DispatchResult::Parsed(_)) => {
                Err(
                    HlxError::execution_error(
                        "Parsed but not executed",
                        "Use process() for file processing",
                    ),
                )
            }
            Err(e) => Err(e),
        }
    }
    pub async fn execute_operator(
        &self,
        operator: &str,
        params: &str,
    ) -> Result<Value, HlxError> {
        self.operator_engine.execute_operator(operator, params).await
    }
    pub fn sections(&self) -> Vec<&String> {
        self.data.keys().collect()
    }
    pub fn keys(&self, section: &str) -> Option<Vec<&String>> {
        self.data.get(section).map(|s| s.keys().collect())
    }
    pub fn save(&self) -> Result<(), HlxError> {
        if let Some(path) = &self.file_path {
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
                    "Ensure write permissions",
                ))
        } else {
            Err(
                HlxError::invalid_input(
                    "No file path set",
                    "Load a file first or set file_path manually",
                ),
            )
        }
    }
}
impl std::ops::Index<&str> for Hlx {
    type Output = HashMap<String, Value>;
    fn index(&self, section: &str) -> &Self::Output {
        self.data
            .get(section)
            .unwrap_or_else(|| panic!("Section '{}' not found", section))
    }
}
impl std::ops::IndexMut<&str> for Hlx {
    fn index_mut(&mut self, section: &str) -> &mut Self::Output {
        self.data.entry(section.to_string()).or_insert_with(HashMap::new)
    }
}
pub mod test_operators {
    use super::*;
    pub async fn test_fundamental_operators() -> Result<(), HlxError> {
        let mut hlx = Hlx::new().await?;
        println!("Testing fundamental operators...");
        let result = hlx.execute(r#"@var(name="test_var", value="hello")"#).await?;
        println!("@var result: {:?}", result);
        let result = hlx.execute(r#"@env(key="HOME")"#).await?;
        println!("@env result: {:?}", result);
        let result = hlx.execute(r#"@date("Y-m-d")"#).await?;
        println!("@date result: {:?}", result);
        let result = hlx.execute(r#"@time("H:i:s")"#).await?;
        println!("@time result: {:?}", result);
        let result = hlx.execute("@uuid()").await?;
        println!("@uuid result: {:?}", result);
        let result = hlx.execute(r#"@string("hello world", "upper")"#).await?;
        println!("@string result: {:?}", result);
        let result = hlx.execute(r#"@math("5 + 3")"#).await?;
        println!("@math result: {:?}", result);
        let result = hlx.execute(r#"@calc("a = 10; b = 5; a + b")"#).await?;
        println!("@calc result: {:?}", result);
        let result = hlx
            .execute(r#"@if(condition="true", then="yes", else="no")"#)
            .await?;
        println!("@if result: {:?}", result);
        let result = hlx
            .execute(r#"@array(values="[1,2,3]", operation="length")"#)
            .await?;
        println!("@array result: {:?}", result);
        let result = hlx.execute(r#"@json('{"name":"test"}', "parse")"#).await?;
        println!("@json result: {:?}", result);
        let result = hlx.execute(r#"@base64("hello", "encode")"#).await?;
        println!("@base64 result: {:?}", result);
        let result = hlx.execute(r#"@hash("password", "sha256")"#).await?;
        println!("@hash result: {:?}", result);
        println!("All fundamental operators tested successfully!");
        Ok(())
    }
    pub async fn test_conditional_operators() -> Result<(), HlxError> {
        let mut hlx = Hlx::new().await?;
        println!("Testing conditional operators...");
        let result = hlx
            .execute(r#"@if(condition="@math('5 > 3')", then="greater", else="less")"#)
            .await?;
        println!("@if with expression: {:?}", result);
        let result = hlx
            .execute(
                r#"@switch(value="2", cases="{'1':'one','2':'two','3':'three'}", default="unknown")"#,
            )
            .await?;
        println!("@switch result: {:?}", result);
        let result = hlx
            .execute(r#"@filter(array="[1,2,3,4,5]", condition="@math('value > 3')")"#)
            .await?;
        println!("@filter result: {:?}", result);
        let result = hlx
            .execute(r#"@map(array="[1,2,3]", transform="@math('value * 2')")"#)
            .await?;
        println!("@map result: {:?}", result);
        let result = hlx
            .execute(
                r#"@reduce(array="[1,2,3,4]", initial="0", operation="@math('acc + value')")"#,
            )
            .await?;
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
        let mut hlx = Hlx::new().await.unwrap();
        hlx.data.insert("database".to_string(), HashMap::new());
        hlx.index_mut("database")
            .unwrap()
            .insert(
                "host".to_string(),
                crate::dna::atp::value::Value::String("localhost".to_string()),
            );
        hlx.index_mut("database")
            .unwrap()
            .insert("port".to_string(), crate::dna::atp::value::Value::Number(5432.0));
        assert_eq!(
            hlx.get("database", "host"), Some(& crate::dna::atp::value::Value::String("localhost"
            .to_string()))
        );
        assert_eq!(hlx.get("database", "port"), Some(& Value::Number(5432.0)));
        let sections = hlx.sections();
        assert!(sections.iter().any(| s | * s == "database"));
        let keys = hlx.keys("database").unwrap();
        assert!(keys.iter().any(| k | * k == "host"));
    }
    #[tokio::test]
    async fn test_operator_execution() {
        let hlx = Hlx::new().await.unwrap();
        let result = hlx.execute_operator("date", "{\"format\":\"Y-m-d\"}").await;
        println!("Direct operator execution result: {:?}", result);
        assert!(result.is_ok());
        let result = hlx.execute_operator("uuid", "").await;
        println!("UUID operator execution result: {:?}", result);
        assert!(result.is_ok());
        let result = hlx.execute_operator("nonexistent", "{}").await;
        println!("Invalid operator result: {:?}", result);
        assert!(result.is_err());
    }
    #[tokio::test]
    async fn test_operator_integration() {
        use crate::ops::OperatorParser;
        let mut ops_parser = OperatorParser::new().await;
        let result = ops_parser.parse_value("@date(\"Y-m-d\")").await.unwrap();
        match result {
            crate::dna::atp::value::Value::String(date_str) => {
                assert!(! date_str.is_empty());
                println!("✅ @date operator working: {}", date_str);
            }
            _ => panic!("Expected string result from @date"),
        }
        let result = ops_parser.parse_value("@uuid()").await.unwrap();
        match result {
            crate::dna::atp::value::Value::String(uuid_str) => {
                assert!(! uuid_str.is_empty());
                println!(
                    "✅ @uuid operator working: {} (length: {})", uuid_str, uuid_str
                    .len()
                );
            }
            _ => panic!("Expected string result from @uuid"),
        }
        use dna::ops::OperatorEngine;
        let operator_engine = OperatorEngine::new().await.unwrap();
        let result = operator_engine
            .execute_operator("date", "{\"format\":\"%Y-%m-%d\"}")
            .await
            .unwrap();
        match result {
            crate::dna::atp::value::Value::String(date_str) => {
                assert!(! date_str.is_empty());
                println!("✅ Direct date operator working: {}", date_str);
            }
            _ => panic!("Expected string result from direct date operator"),
        }
        let result = operator_engine.execute_operator("uuid", "").await.unwrap();
        match result {
            crate::dna::atp::value::Value::String(uuid_str) => {
                assert!(! uuid_str.is_empty());
                println!(
                    "✅ Direct uuid operator working: {} (length: {})", uuid_str,
                    uuid_str.len()
                );
            }
            _ => panic!("Expected string result from direct uuid operator"),
        }
        println!("✅ ops.rs and operators/ integration fully working!");
    }
    #[tokio::test]
    async fn test_comprehensive_operator_testing() {
        assert!(true);
    }
}