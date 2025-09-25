use crate::error::HlxError;
use crate::value::Value;
use crate::ast::Expression;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

const DEFAULT_ULATOR_GRAMMAR: &str = r##"// ------------------------------------------------
//  The tiny language used in the ulator DSL
// ------------------------------------------------

WHITESPACE = _{ " " | "\t" | "\r" | "\n" }

program       = { reproducibility }
reproducibility = { "reproducibility" ~ "{" ~ statement* ~ "}" }

statement     = { identifier ~ "=" ~ expr }
expr          = { term ~ (("+" | "-") ~ term)* }
term          = { factor ~ (("x" | "*") ~ factor)* }
factor        = _{ reference | signed_number | identifier | "(" ~ expr ~ ")" }

reference     = { "@" ~ identifier ~ ("#" ~ number)? }
identifier    = @{ ASCII_ALPHANUMERIC+ }
signed_number = { ("-")? ~ number }
number        = @{ (ASCII_DIGIT | "_")+ }
"##;

pub struct OperatorParser {
    data: HashMap<String, Value>,
    global_variables: HashMap<String, Value>,
    section_variables: HashMap<String, Value>,
    cache: HashMap<String, Value>,
    cross_file_cache: HashMap<String, Value>,
    current_section: String,
    in_object: bool,
    object_key: String,
    hlx_loaded: bool,
    
    // Standard dna.hlx locations
    hlx_locations: Vec<String>,
    
    // Operator engine for @ operators
    operator_engine: crate::operators::OperatorEngine,
}

pub fn get_or_create_helix_dir() -> std::io::Result<PathBuf> {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE")) // fallback for Windows
        .map(PathBuf::from)
        .or_else(|_| {
            // As a last resort, use the `dirs` crate (adds a tiny dependency).
            #[cfg(feature = "dirs")]
            {
                dirs::home_dir().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Could not locate a home directory",
                    )
                })
            }
            #[cfg(not(feature = "dirs"))]
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "HOME (or USERPROFILE) environment variable is missing",
                ))
            }
        })?;
    let helix_dir = home_dir.join(".dna").join("hlx");
    if !helix_dir.exists() {
        fs::create_dir_all(&helix_dir)?;
        
    }
    let _ = ensure_calc()?;
    Ok(helix_dir)
}

/// Ensure calculator directory exists and return its path.
/// If calculator.pest does not exist, create it with the default grammar.
pub fn ensure_calc() -> std::io::Result<PathBuf> {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .or_else(|_| {
            #[cfg(feature = "dirs")]
            {
                dirs::home_dir().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Could not locate a home directory",
                    )
                })
            }
            #[cfg(not(feature = "dirs"))]
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "HOME (or USERPROFILE) environment variable is missing",
                ))
            }
        })?;
    let calc_dir = home_dir.join(".dna").join("calc");
    if !calc_dir.exists() {
        fs::create_dir_all(&calc_dir)?;
    }

    // Write ulator.pest if it does not exist ULATOR IS THE RIGHT SPELLING
    let pest_path = calc_dir.join("ulator.pest");
    if !pest_path.exists() {
        let mut file = fs::File::create(&pest_path)?;
        file.write_all(DEFAULT_ULATOR_GRAMMAR.as_bytes())?;
    }

    Ok(calc_dir)
}

impl OperatorParser {
    /// Create a new enhanced parser
    pub async fn new() -> Self {
        let helix_config = env::var("HELIX_CONFIG").unwrap_or_else(|_| {
                    get_or_create_helix_dir().unwrap_or_else(|_| PathBuf::new()).to_string_lossy().to_string()
                });        
        Self {
            data: HashMap::new(),
            global_variables: HashMap::new(),
            section_variables: HashMap::new(),
            cache: HashMap::new(),
            cross_file_cache: HashMap::new(),
            current_section: String::new(),
            in_object: false,
            object_key: String::new(),
            hlx_loaded: false,
            hlx_locations: vec![
                "./dna.hlx".to_string(),
                "../dna.hlx".to_string(),
                "../../dna.hlx".to_string(),
                "/root/.dna/hlx/dna.hlx".to_string(),
                get_or_create_helix_dir().unwrap_or_else(|_| PathBuf::new()).join("dna.hlx").to_string_lossy().to_string(),
                helix_config,
            ],
            operator_engine: crate::operators::OperatorEngine::new().await.unwrap_or_else(|_| {
                panic!("Failed to initialize operator engine")
            }),
        }
    }
    
    /// Load dna.hlx if available
    pub async fn load_hlx(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.hlx_loaded {
            return Ok(());
        }

        self.hlx_loaded = true; // Mark first to prevent recursion

        let locations: Vec<String> = self.hlx_locations.iter().cloned().collect();
        for location in locations {
            if location.is_empty() {
                continue;
            }

            if Path::new(&location).exists() {
                println!("# Loading universal config from: {}", location);
                return self.parse_file(&location).await;
            }
        }

        Ok(())
    }
    
    /// Parse helix value with all syntax support
    pub async fn parse_value(&mut self, value: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let value = value.trim();

        // Remove optional semicolon
        let value = if value.ends_with(';') {
            value.trim_end_matches(';').trim()
        } else {
            value
        };

        // Basic types
        match value {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            "null" => return Ok(Value::Null),
            _ => {}
        }

        // Numbers
        if let Ok(num) = value.parse::<i64>() {
            return Ok(Value::Number(num as f64));
        }
        if let Ok(num) = value.parse::<f64>() {
            return Ok(Value::Number(num));
        }

        // $variable references (global)
        let global_var_re = Regex::new(r"^\$([a-zA-Z_][a-zA-Z0-9_]*)$").unwrap();
        if let Some(captures) = global_var_re.captures(value) {
            let var_name = captures.get(1).unwrap().as_str();
            if let Some(val) = self.global_variables.get(var_name) {
                return Ok(val.clone());
            }
            return Ok(Value::String("".to_string()));
        }

        // Section-local variable references
        if !self.current_section.is_empty() {
            let local_var_re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
            if local_var_re.is_match(value) {
                let section_key = format!("{}.{}", self.current_section, value);
                if let Some(val) = self.section_variables.get(&section_key) {
                    return Ok(val.clone());
                }
            }
        }

        // @date function
        let date_re = Regex::new(r#"^@date\(["'](.*)["']\)$"#).unwrap();
        if let Some(captures) = date_re.captures(value) {
            let format_str = captures.get(1).unwrap().as_str();
            return Ok(Value::String(self.execute_date(format_str)));
        }

        // @env function with default
        let env_re = Regex::new(r#"^@env\(["']([^"']*)["'](?:,\s*(.+))?\)$"#).unwrap();
        if let Some(captures) = env_re.captures(value) {
            let env_var = captures.get(1).unwrap().as_str();
            let default_val = if let Some(default_match) = captures.get(2) {
                default_match.as_str().trim_matches('"').trim_matches('\'')
            } else {
                ""
            };
            return Ok(Value::String(env::var(env_var.to_string()).unwrap_or_else(|_| default_val.to_string())));
        }
        
        // Ranges: 8000-9000
        let range_re = Regex::new(r"^(\d+)-(\d+)$").unwrap();
        if let Some(captures) = range_re.captures(value) {
            let min = captures.get(1).unwrap().as_str().parse::<f64>().unwrap();
            let max = captures.get(2).unwrap().as_str().parse::<f64>().unwrap();
            let mut range_obj = HashMap::new();
            range_obj.insert("min".to_string(), Value::Number(min));
            range_obj.insert("max".to_string(), Value::Number(max));
            range_obj.insert("type".to_string(), Value::String("range".to_string()));
            return Ok(Value::Object(range_obj));
        }

        // Arrays
        if value.starts_with('[') && value.ends_with(']') {
            return Box::pin(self.parse_array(value)).await;
        }

        // Objects
        if value.starts_with('{') && value.ends_with('}') {
            return Box::pin(self.parse_object(value)).await;
        }

        // Cross-file references: @file.hlx.get('key')
        let cross_get_re = Regex::new(r#"^@([a-zA-Z0-9_-]+)\.hlx\.get\(["'](.*)["']\)$"#).unwrap();
        if let Some(captures) = cross_get_re.captures(value) {
            let file_name = captures.get(1).unwrap().as_str();
            let key = captures.get(2).unwrap().as_str();
            return Box::pin(self.cross_file_get(file_name, key)).await;
        }

        // Cross-file set: @file.hlx.set('key', value)
        let cross_set_re = Regex::new(r#"^@([a-zA-Z0-9_-]+)\.hlx\.set\(["']([^"']*)["'],\s*(.+)\)$"#).unwrap();
        if let Some(captures) = cross_set_re.captures(value) {
            let file_name = captures.get(1).unwrap().as_str();
            let key = captures.get(2).unwrap().as_str();
            let val = captures.get(3).unwrap().as_str();
            return Box::pin(self.cross_file_set(file_name, key, val)).await;
        }

        // @query function
        let query_re = Regex::new(r#"^@query\(["'](.*)["'](.*)\)$"#).unwrap();
        if let Some(captures) = query_re.captures(value) {
            let query = captures.get(1).unwrap().as_str();
            return Ok(Value::String(self.execute_query(query).await));
        }

        // @ operators
        let operator_re = Regex::new(r"^@([a-zA-Z_][a-zA-Z0-9_]*)\((.+)\)$").unwrap();
        if let Some(captures) = operator_re.captures(value) {
            let operator = captures.get(1).unwrap().as_str();
            let params = captures.get(2).unwrap().as_str();
            return self.execute_operator(operator, params).await;
        }

        // String concatenation
        if value.contains(" + ") {
            let parts: Vec<&str> = value.split(" + ").collect();
            let mut result = String::new();
            for part in parts {
                let part = part.trim().trim_matches('"').trim_matches('\'');
                if !part.starts_with('"') {
                    let parsed_part = Box::pin(self.parse_value(part)).await?;
                    result.push_str(&parsed_part.to_string());
                } else {
                    result.push_str(&part[1..part.len()-1]);
                }
            }
            return Ok(Value::String(result.to_string()));
        }

        // Conditional/ternary: condition ? true_val : false_val
        let ternary_re = Regex::new(r"(.+?)\s*\?\s*(.+?)\s*:\s*(.+)").unwrap();
        if let Some(captures) = ternary_re.captures(value) {
            let condition = captures.get(1).unwrap().as_str().trim();
            let true_val = captures.get(2).unwrap().as_str().trim();
            let false_val = captures.get(3).unwrap().as_str().trim();

            if Box::pin(self.evaluate_condition(condition)).await {
                return Box::pin(self.parse_value(true_val)).await;
            } else {
                return Box::pin(self.parse_value(false_val)).await;
            }
        }

        // Remove quotes from strings
        if (value.starts_with('"') && value.ends_with('"')) ||
           (value.starts_with('\'') && value.ends_with('\'')) {
            return Ok(Value::String(value[1..value.len()-1].to_string()));
        }

        // Return as string
        Ok(Value::String(value.to_string()))
    }
    
    /// Parse array syntax
    async fn parse_array(&mut self, value: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let content = value[1..value.len()-1].trim();
        if content.is_empty() {
            return Ok(Value::Array(Vec::new()));
        }

        let mut items = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut quote_char = '\0';

        for ch in content.chars() {
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                quote_char = ch;
            } else if ch == quote_char && in_string {
                in_string = false;
                quote_char = '\0';
            }

            if !in_string {
                match ch {
                    '[' | '{' => depth += 1,
                    ']' | '}' => depth -= 1,
                    ',' if depth == 0 => {
                        items.push(self.parse_value(current.trim()).await?);
                        current.clear();
                        continue;
                    }
                    _ => {}
                }
            }

            current.push(ch);
        }

        if !current.trim().is_empty() {
            items.push(self.parse_value(current.trim()).await?);
        }

        Ok(Value::Array(items))
    }
    
    /// Parse object syntax
    async fn parse_object(&mut self, value: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let content = value[1..value.len()-1].trim();
        if content.is_empty() {
            return Ok(Value::Object(HashMap::new()));
        }

        let mut pairs = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut quote_char = '\0';

        for ch in content.chars() {
            if (ch == '"' || ch == '\'') && !in_string {
                in_string = true;
                quote_char = ch;
            } else if ch == quote_char && in_string {
                in_string = false;
                quote_char = '\0';
            }

            if !in_string {
                match ch {
                    '[' | '{' => depth += 1,
                    ']' | '}' => depth -= 1,
                    ',' if depth == 0 => {
                        pairs.push(current.trim().to_string());
                        current.clear();
                        continue;
                    }
                    _ => {}
                }
            }

            current.push(ch);
        }

        if !current.trim().is_empty() {
            pairs.push(current.trim().to_string());
        }

        let mut obj = HashMap::new();
        for pair in pairs {
            if let Some(colon_pos) = pair.find(':') {
                let key = pair[..colon_pos].trim().trim_matches('"').trim_matches('\'');
                let val = pair[colon_pos+1..].trim();
                obj.insert(key.to_string(), self.parse_value(val).await?);
            } else if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].trim().trim_matches('"').trim_matches('\'');
                let val = pair[eq_pos+1..].trim();
                obj.insert(key.to_string(), self.parse_value(val).await?);
            }
        }

        Ok(Value::Object(obj))
    }
    
    /// Evaluate conditions for ternary expressions
    async fn evaluate_condition(&mut self, condition: &str) -> bool {
        let condition = condition.trim();

        // Simple equality check
        if let Some(eq_pos) = condition.find("==") {
            let left = self.parse_value(condition[..eq_pos].trim()).await.unwrap_or(Value::String("".to_string()));
            let right = self.parse_value(condition[eq_pos+2..].trim()).await.unwrap_or(Value::String("".to_string()));
            return left.to_string() == right.to_string();
        }

        // Not equal
        if let Some(ne_pos) = condition.find("!=") {
            let left = self.parse_value(condition[..ne_pos].trim()).await.unwrap_or(Value::String("".to_string()));
            let right = self.parse_value(condition[ne_pos+2..].trim()).await.unwrap_or(Value::String("".to_string()));
            return left.to_string() != right.to_string();
        }

        // Greater than
        if let Some(gt_pos) = condition.find('>') {
            let left = self.parse_value(condition[..gt_pos].trim()).await.unwrap_or(Value::String("".to_string()));
            let right = self.parse_value(condition[gt_pos+1..].trim()).await.unwrap_or(Value::String("".to_string()));

            if let (Value::Number(l), Value::Number(r)) = (&left, &right) {
                return l > r;
            }
            return left.to_string() > right.to_string();
        }

        // Default: check if truthy
        let value = self.parse_value(condition).await.unwrap_or(Value::String("".to_string()));
        match value {
            Value::Bool(b) => b,
            Value::String(s) => !s.is_empty() && s != "false" && s != "null" && s != "0",
            Value::Number(n) => n != 0.0,
            Value::Null => false,
            _ => true,
        }
    }
    
    /// Get value from another HLX file
    async fn cross_file_get(&mut self, file_name: &str, key: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("{}:{}", file_name, key);

        // Check cache
        if let Some(val) = self.cross_file_cache.get(&cache_key) {
            return Ok(val.clone());
        }

        let helix_home = get_or_create_helix_dir().unwrap_or_else(|_| PathBuf::new());
        let helix_home_str = helix_home.to_string_lossy().to_string();

        // Find file
        let directories = [".", &helix_home_str, "./config", "..", "../config"];
        let mut file_path = None;

        for directory in &directories {
            let potential_path = Path::new(directory).join(format!("{}.hlx", file_name));
            if potential_path.exists() {
                file_path = Some(potential_path);
                break;
            }
        }

        if let Some(path) = file_path {
            // Parse file and get value
            let mut temp_parser = OperatorParser::new().await;
            if Box::pin(temp_parser.parse_file(path.to_str().unwrap())).await.is_ok() {
                if let Some(value) = temp_parser.get(key) {
                    // Cache result
                    let _ = self.cross_file_cache.insert(cache_key, value.clone());
                    return Ok(value);
                }
            }
        }

        Ok(Value::String("".to_string()))
    }
    
    /// Set value in another HLX file (cache only for now)
    async fn cross_file_set(&mut self, file_name: &str, key: &str, value: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("{}:{}", file_name, key);
        let parsed_value = self.parse_value(value).await?;
        let _ = self.cross_file_cache.insert(cache_key, parsed_value.clone());
        Ok(parsed_value)
    }
    
    /// Execute @date function
    fn execute_date(&self, format_str: &str) -> String {
        let now: DateTime<Utc> = Utc::now();
        
        // Convert PHP-style format to Rust
        match format_str {
            "Y" => now.format("%Y").to_string(),
            "Y-m-d" => now.format("%Y-%m-%d").to_string(),
            "Y-m-d H:i:s" => now.format("%Y-%m-%d %H:%M:%S").to_string(),
            "c" => now.to_rfc3339(),
            _ => now.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
    
    /// Execute database query with enhanced @query syntax
    async fn execute_query(&mut self, query: &str) -> String {
        let _ = self.load_hlx().await;

        // Determine database type
        let db_type = self.get("database.default")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "sqlite".to_string());

        // Enhanced @query implementation with cross-database support
        if query.contains(':') {
            // Cross-database query: @db1,db2:SELECT * FROM users
            let parts: Vec<&str> = query.splitn(2, ':').collect();
            if parts.len() == 2 {
                let databases = parts[0];
                let actual_query = parts[1];
                format!("[Cross-DB Query: {} on {}]", actual_query, databases)
            } else {
                format!("[Query: {} on {}]", query, db_type)
            }
        } else if query.to_lowercase().contains("insert") && query.contains("{") {
            // MongoDB-like insert: INSERT INTO users {name: "John", age: 30}
            format!("[Auto-Schema Insert: {} on {}]", query, db_type)
        } else if query.to_lowercase().contains("sync:") {
            // Cross-database sync: sync:sqlite->postgres:users
            format!("[Sync Operation: {} on {}]", query, db_type)
        } else {
            // Standard query
            format!("[Query: {} on {}]", query, db_type)
        }
    }
    
    /// Execute @ operators
    async fn execute_operator(&mut self, operator: &str, params: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match self.operator_engine.execute_operator(operator, params).await {
            Ok(value) => Ok(value),
            Err(e) => {
                eprintln!("Operator execution error: {:?}", e);
                Ok(Value::String(format!("@{}({})", operator, params)))
            }
        }
    }
    
    /// Parse a single line
    pub async fn parse_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(());
        }

        // Remove optional semicolon
        let trimmed = if trimmed.ends_with(';') {
            trimmed.trim_end_matches(';').trim()
        } else {
            trimmed
        };

        // Check for section declaration []
        let section_re = Regex::new(r"^\[([a-zA-Z_][a-zA-Z0-9_]*)\]$").unwrap();
        if let Some(captures) = section_re.captures(trimmed) {
            self.current_section = captures.get(1).unwrap().as_str().to_string();
            self.in_object = false;
            return Ok(());
        }

        // Check for angle bracket object >
        let angle_open_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\s*>$").unwrap();
        if let Some(captures) = angle_open_re.captures(trimmed) {
            self.in_object = true;
            self.object_key = captures.get(1).unwrap().as_str().to_string();
            return Ok(());
        }

        // Check for closing angle bracket <
        if trimmed == "<" {
            self.in_object = false;
            self.object_key.clear();
            return Ok(());
        }

        // Check for curly brace object {
        let brace_open_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\s*\{$").unwrap();
        if let Some(captures) = brace_open_re.captures(trimmed) {
            self.in_object = true;
            self.object_key = captures.get(1).unwrap().as_str().to_string();
            return Ok(());
        }

        // Check for closing curly brace }
        if trimmed == "}" {
            self.in_object = false;
            self.object_key.clear();
            return Ok(());
        }

        // Parse key-value pairs (both : and = supported)
        let kv_re = Regex::new(r"^([\$]?[a-zA-Z_][a-zA-Z0-9_-]*)\s*[:=]\s*(.+)$").unwrap();
        if let Some(captures) = kv_re.captures(trimmed) {
            let key = captures.get(1).unwrap().as_str();
            let value = captures.get(2).unwrap().as_str();
            let parsed_value = Box::pin(self.parse_value(value)).await?;

            // Determine storage location
            let storage_key = if self.in_object && !self.object_key.is_empty() {
                if !self.current_section.is_empty() {
                    format!("{}.{}.{}", self.current_section, self.object_key, key)
                } else {
                    format!("{}.{}", self.object_key, key)
                }
            } else if !self.current_section.is_empty() {
                format!("{}.{}", self.current_section, key)
            } else {
                key.to_string()
            };

            // Store the value
            let _ = self.data.insert(storage_key.clone(), parsed_value.clone());

            // Handle global variables
            if key.starts_with('$') {
                let var_name = &key[1..];
                let _ = self.global_variables.insert(var_name.to_string(), parsed_value.clone());
            } else if !self.current_section.is_empty() && !key.starts_with('$') {
                // Store section-local variable
                let section_key = format!("{}.{}", self.current_section, key);
                self.section_variables.insert(section_key, parsed_value);
            }
        }

        Ok(())
    }
    
    /// Evaluate an AST expression with special operator support
    pub async fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        match expr {
            Expression::String(s) => {
                // Check if string contains special operators
                if s.starts_with('@') || s.contains(" + ") || s.contains('?') {
                    Ok(self.parse_value(s).await?)
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Bool(b) => Ok(Value::Bool(*b)),
            Expression::Array(arr) => {
                let mut values = Vec::new();
                for item in arr {
                    values.push(Box::pin(self.evaluate_expression(item)).await?);
                }
                Ok(Value::Array(values))
            }
            Expression::Object(obj) => {
                let mut map = HashMap::new();
                for (key, expr) in obj {
                    map.insert(key.clone(), Box::pin(self.evaluate_expression(expr)).await?);
                }
                Ok(Value::Object(map))
            }
            Expression::OperatorCall(operator, params) => {
                Err(Box::new(HlxError::validation_error("OperatorCall not supported", "Use @ prefixed operators instead")))
            }
            Expression::AtOperatorCall(operator, params) => {
                let json_params = self.params_to_json(params).await?;
                Ok(self.execute_operator(operator, &json_params).await?)
            }
            Expression::Variable(name) => {
                if let Some(value) = self.global_variables.get(name) {
                    Ok(value.clone())
                } else if let Some(value) = self.section_variables.get(name) {
                    Ok(value.clone())
                } else {
                    Ok(Value::String("".to_string()))
                }
            }
            _ => Ok(Value::String(format!("Unsupported expression: {:?}", expr))),
        }
    }

    /// Convert expression parameters to JSON string
    async fn params_to_json(&mut self, params: &HashMap<String, Expression>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut json_map = serde_json::Map::new();
        for (key, expr) in params {
            let value = Box::pin(self.evaluate_expression(expr)).await?;
            let json_value = self.value_to_json_value(&value);
            json_map.insert(key.clone(), json_value);
        }
        let json_obj = serde_json::Value::Object(json_map);
        Ok(serde_json::to_string(&json_obj)?)
    }

    /// Convert Value to serde_json::Value
    fn value_to_json_value(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Number(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(*n).unwrap_or_else(|| serde_json::Number::from(0))
            ),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.value_to_json_value(v)).collect())
            },
            Value::Object(obj) => {
                serde_json::Value::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), self.value_to_json_value(v)))
                        .collect()
                )
            },
            Value::Null => serde_json::Value::Null,
        }
    }

    /// Parse helix content (configuration parser - fallback for non-Helix files)
    pub async fn parse(&mut self, content: &str) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            Box::pin(self.parse_line(line)).await?;
        }

        Ok(self.data.clone())
    }
    
    /// Parse a HLX file
    pub async fn parse_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(file_path)?;
        Box::pin(self.parse(&content)).await?;
        Ok(())
    }
    
    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }
    
    /// Set a value
    pub fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }
    
    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
    
    /// Get all key-value pairs
    pub fn items(&self) -> HashMap<String, Value> {
        self.data.clone()
    }
}


/// Load configuration from dna.hlx
pub async fn load_from_hlx() -> Result<OperatorParser, Box<dyn std::error::Error + Send + Sync>> {
    let mut parser = OperatorParser::new().await;
    Box::pin(parser.load_hlx()).await?;
    Ok(parser)
}

/// Parse helix content and return configuration
pub async fn parse_hlx_content(content: &str) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
    let mut parser = OperatorParser::new().await;
    Box::pin(parser.parse(content)).await
}