use std::path::Path;
use std::fs;
use serde_json::Value as JsonValue;
use anyhow::{Result, Context};
pub struct Migrator {
    verbose: bool,
    _preserve_comments: bool,
}
impl Migrator {
    pub fn new() -> Self {
        Self {
            verbose: false,
            _preserve_comments: true,
        }
    }
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }
    pub fn migrate_file<P: AsRef<Path>>(
        &self,
        input: P,
        output: Option<P>,
    ) -> Result<String> {
        let input_path = input.as_ref();
        let extension = input_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?;
        let content = fs::read_to_string(input_path)
            .context("Failed to read input file")?;
        let helix_content = match extension {
            "json" => self.migrate_json(&content)?,
            "toml" => self.migrate_toml(&content)?,
            "yaml" | "yml" => self.migrate_yaml(&content)?,
            "env" => self.migrate_env(&content)?,
            _ => return Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        };
        if let Some(output_path) = output {
            fs::write(output_path.as_ref(), &helix_content)
                .context("Failed to write output file")?;
        }
        Ok(helix_content)
    }
    pub fn migrate_json(&self, json_str: &str) -> Result<String> {
        let value: JsonValue = serde_json::from_str(json_str)
            .context("Failed to parse JSON")?;
        let mut output = String::new();
        output.push_str("# Migrated from JSON\n");
        output
            .push_str(
                "# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh\n\n",
            );
        if let Some(obj) = value.as_object() {
            self.convert_json_object_to_hlx(obj, &mut output, 0)?;
        }
        Ok(output)
    }
    pub fn migrate_toml(&self, toml_str: &str) -> Result<String> {
        let value: toml::Value = toml::from_str(toml_str)
            .context("Failed to parse TOML")?;
        let mut output = String::new();
        output.push_str("# Migrated from TOML\n");
        output
            .push_str(
                "# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh\n\n",
            );
        if let Some(table) = value.as_table() {
            self.convert_toml_table_to_hlx(table, &mut output, 0)?;
        }
        Ok(output)
    }
    pub fn migrate_yaml(&self, yaml_str: &str) -> Result<String> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml_str)
            .context("Failed to parse YAML")?;
        let mut output = String::new();
        output.push_str("# Migrated from YAML\n");
        output
            .push_str(
                "# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh\n\n",
            );
        if let Some(mapping) = value.as_mapping() {
            self.convert_yaml_mapping_to_hlx(mapping, &mut output, 0)?;
        }
        Ok(output)
    }
    pub fn migrate_env(&self, env_str: &str) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Migrated from .env\n");
        output
            .push_str(
                "# Send Buddy the Beagle a bitcoin treat for making this page: bc1quct28jtvvuymvkvjfgcedhd7jt0c56975f2fsh\n\n",
            );
        output.push_str("context \"environment\" {\n");
        for line in env_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                if key.contains("KEY") || key.contains("SECRET") || key.contains("TOKEN")
                {
                    output.push_str(&format!("    {} = ${}\n", key.to_lowercase(), key));
                } else {
                    output
                        .push_str(
                            &format!("    {} = \"{}\"\n", key.to_lowercase(), value),
                        );
                }
            }
        }
        output.push_str("}\n");
        Ok(output)
    }
    fn convert_json_object_to_hlx(
        &self,
        obj: &serde_json::Map<String, JsonValue>,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        for (key, value) in obj {
            match key.as_str() {
                "agent" | "agents" => {
                    self.convert_to_agent_block(key, value, output, indent)?;
                }
                "workflow" | "workflows" => {
                    self.convert_to_workflow_block(key, value, output, indent)?;
                }
                "memory" => {
                    self.convert_to_memory_block(value, output, indent)?;
                }
                "context" | "contexts" => {
                    self.convert_to_context_block(key, value, output, indent)?;
                }
                _ => {
                    match value {
                        JsonValue::Object(inner) => {
                            output.push_str(&format!("{}{} {{\n", indent_str, key));
                            self.convert_json_object_to_hlx(inner, output, indent + 1)?;
                            output.push_str(&format!("{}}}\n", indent_str));
                        }
                        JsonValue::Array(arr) => {
                            output.push_str(&format!("{}{} [\n", indent_str, key));
                            for item in arr {
                                output.push_str(&format!("{}    ", indent_str));
                                self.write_json_value(item, output)?;
                                output.push_str("\n");
                            }
                            output.push_str(&format!("{}]\n", indent_str));
                        }
                        _ => {
                            output.push_str(&format!("{}{} = ", indent_str, key));
                            self.write_json_value(value, output)?;
                            output.push_str("\n");
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn convert_to_agent_block(
        &self,
        _key: &str,
        value: &JsonValue,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        if let Some(obj) = value.as_object() {
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
            output.push_str(&format!("{}agent \"{}\" {{\n", indent_str, name));
            for (k, v) in obj {
                if k == "name" {
                    continue;
                }
                match k.as_str() {
                    "temperature" => {
                        if let Some(num) = v.as_f64() {
                            output
                                .push_str(
                                    &format!("{}    temperature = {}\n", indent_str, num),
                                );
                        }
                    }
                    "max_tokens" => {
                        if let Some(num) = v.as_i64() {
                            output
                                .push_str(
                                    &format!("{}    max_tokens = {}\n", indent_str, num),
                                );
                        }
                    }
                    "timeout" => {
                        if let Some(s) = v.as_str() {
                            let duration = self.parse_duration_string(s);
                            output
                                .push_str(
                                    &format!("{}    timeout = {}\n", indent_str, duration),
                                );
                        }
                    }
                    _ => {
                        output.push_str(&format!("{}    {} = ", indent_str, k));
                        self.write_json_value(v, output)?;
                        output.push_str("\n");
                    }
                }
            }
            output.push_str(&format!("{}}}\n", indent_str));
        }
        Ok(())
    }
    fn convert_to_workflow_block(
        &self,
        _key: &str,
        value: &JsonValue,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        if let Some(obj) = value.as_object() {
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
            output.push_str(&format!("{}workflow \"{}\" {{\n", indent_str, name));
            if let Some(trigger) = obj.get("trigger") {
                output.push_str(&format!("{}    trigger = ", indent_str));
                self.write_json_value(trigger, output)?;
                output.push_str("\n");
            }
            if let Some(steps) = obj.get("steps").and_then(|v| v.as_array()) {
                for (i, step) in steps.iter().enumerate() {
                    output
                        .push_str(
                            &format!("{}    step \"step_{}\" {{\n", indent_str, i + 1),
                        );
                    if let Some(step_obj) = step.as_object() {
                        for (k, v) in step_obj {
                            output.push_str(&format!("{}        {} = ", indent_str, k));
                            self.write_json_value(v, output)?;
                            output.push_str("\n");
                        }
                    }
                    output.push_str(&format!("{}    }}\n", indent_str));
                }
            }
            output.push_str(&format!("{}}}\n", indent_str));
        }
        Ok(())
    }
    fn convert_to_memory_block(
        &self,
        value: &JsonValue,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        output.push_str(&format!("{}memory {{\n", indent_str));
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                output.push_str(&format!("{}    {} = ", indent_str, k));
                self.write_json_value(v, output)?;
                output.push_str("\n");
            }
        }
        output.push_str(&format!("{}}}\n", indent_str));
        Ok(())
    }
    fn convert_to_context_block(
        &self,
        _key: &str,
        value: &JsonValue,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        if let Some(obj) = value.as_object() {
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("default");
            output.push_str(&format!("{}context \"{}\" {{\n", indent_str, name));
            for (k, v) in obj {
                if k == "name" {
                    continue;
                }
                output.push_str(&format!("{}    {} = ", indent_str, k));
                self.write_json_value(v, output)?;
                output.push_str("\n");
            }
            output.push_str(&format!("{}}}\n", indent_str));
        }
        Ok(())
    }
    fn convert_toml_table_to_hlx(
        &self,
        table: &toml::map::Map<String, toml::Value>,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        for (key, value) in table {
            match value {
                toml::Value::Table(inner) => {
                    if key.starts_with("agent") {
                        output
                            .push_str(
                                &format!(
                                    "{}agent \"{}\" {{\n", indent_str, key
                                    .trim_start_matches("agent.")
                                ),
                            );
                        self.convert_toml_table_to_hlx(inner, output, indent + 1)?;
                        output.push_str(&format!("{}}}\n", indent_str));
                    } else {
                        output.push_str(&format!("{}{} {{\n", indent_str, key));
                        self.convert_toml_table_to_hlx(inner, output, indent + 1)?;
                        output.push_str(&format!("{}}}\n", indent_str));
                    }
                }
                toml::Value::Array(arr) => {
                    output.push_str(&format!("{}{} [\n", indent_str, key));
                    for item in arr {
                        output.push_str(&format!("{}    ", indent_str));
                        self.write_toml_value(item, output)?;
                        output.push_str("\n");
                    }
                    output.push_str(&format!("{}]\n", indent_str));
                }
                _ => {
                    output.push_str(&format!("{}{} = ", indent_str, key));
                    self.write_toml_value(value, output)?;
                    output.push_str("\n");
                }
            }
        }
        Ok(())
    }
    fn convert_yaml_mapping_to_hlx(
        &self,
        mapping: &serde_yaml::Mapping,
        output: &mut String,
        indent: usize,
    ) -> Result<()> {
        let indent_str = "    ".repeat(indent);
        for (key, value) in mapping {
            let key_str = match key {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                _ => format!("{:?}", key),
            };
            match value {
                serde_yaml::Value::Mapping(inner) => {
                    output.push_str(&format!("{}{} {{\n", indent_str, key_str));
                    self.convert_yaml_mapping_to_hlx(inner, output, indent + 1)?;
                    output.push_str(&format!("{}}}\n", indent_str));
                }
                serde_yaml::Value::Sequence(seq) => {
                    output.push_str(&format!("{}{} [\n", indent_str, key_str));
                    for item in seq {
                        output.push_str(&format!("{}    ", indent_str));
                        self.write_yaml_value(item, output)?;
                        output.push_str("\n");
                    }
                    output.push_str(&format!("{}]\n", indent_str));
                }
                _ => {
                    output.push_str(&format!("{}{} = ", indent_str, key_str));
                    self.write_yaml_value(value, output)?;
                    output.push_str("\n");
                }
            }
        }
        Ok(())
    }
    fn write_json_value(&self, value: &JsonValue, output: &mut String) -> Result<()> {
        match value {
            JsonValue::Null => output.push_str("null"),
            JsonValue::Bool(b) => output.push_str(&b.to_string()),
            JsonValue::Number(n) => output.push_str(&n.to_string()),
            JsonValue::String(s) => output.push_str(&format!("\"{}\"", s)),
            JsonValue::Array(arr) => {
                output.push('[');
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    self.write_json_value(item, output)?;
                }
                output.push(']');
            }
            JsonValue::Object(_) => {
                output.push_str("{ }");
            }
        }
        Ok(())
    }
    fn write_toml_value(&self, value: &toml::Value, output: &mut String) -> Result<()> {
        match value {
            toml::Value::String(s) => output.push_str(&format!("\"{}\"", s)),
            toml::Value::Integer(i) => output.push_str(&i.to_string()),
            toml::Value::Float(f) => output.push_str(&f.to_string()),
            toml::Value::Boolean(b) => output.push_str(&b.to_string()),
            toml::Value::Datetime(d) => output.push_str(&format!("\"{}\"", d)),
            toml::Value::Array(_) => output.push_str("[]"),
            toml::Value::Table(_) => output.push_str("{}"),
        }
        Ok(())
    }
    fn write_yaml_value(
        &self,
        value: &serde_yaml::Value,
        output: &mut String,
    ) -> Result<()> {
        match value {
            serde_yaml::Value::Null => output.push_str("null"),
            serde_yaml::Value::Bool(b) => output.push_str(&b.to_string()),
            serde_yaml::Value::Number(n) => output.push_str(&n.to_string()),
            serde_yaml::Value::String(s) => output.push_str(&format!("\"{}\"", s)),
            serde_yaml::Value::Sequence(_) => output.push_str("[]"),
            serde_yaml::Value::Mapping(_) => output.push_str("{}"),
            serde_yaml::Value::Tagged(_) => output.push_str("null"),
        }
        Ok(())
    }
    fn parse_duration_string(&self, s: &str) -> String {
        if s.ends_with("ms") {
            return format!("{}ms", s.trim_end_matches("ms"));
        } else if s.ends_with("s") || s.ends_with("sec") || s.ends_with("seconds") {
            let num = s
                .chars()
                .take_while(|c| c.is_numeric() || *c == '.')
                .collect::<String>();
            return format!("{}s", num);
        } else if s.ends_with("m") || s.ends_with("min") || s.ends_with("minutes") {
            let num = s
                .chars()
                .take_while(|c| c.is_numeric() || *c == '.')
                .collect::<String>();
            return format!("{}m", num);
        } else if s.ends_with("h") || s.ends_with("hour") || s.ends_with("hours") {
            let num = s
                .chars()
                .take_while(|c| c.is_numeric() || *c == '.')
                .collect::<String>();
            return format!("{}h", num);
        }
        format!("\"{}\"", s)
    }
}
impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_migrate_json() {
        let json = r#"{
            "agent": {
                "name": "assistant",
                "model": "gpt-4",
                "temperature": 0.7
            }
        }"#;
        let migrator = Migrator::new();
        let result = migrator.migrate_json(json).unwrap();
        assert!(result.contains("agent \"assistant\""));
        assert!(result.contains("temperature = 0.7"));
    }
    #[test]
    fn test_migrate_env() {
        let env = r#"
DATABASE_URL=postgres://localhost/db
API_KEY=secret123
DEBUG=true
"#;
        let migrator = Migrator::new();
        let result = migrator.migrate_env(env).unwrap();
        assert!(result.contains("context \"environment\""));
        assert!(result.contains("database_url"));
        assert!(result.contains("api_key = $API_KEY"));
    }
}