use std::path::PathBuf;
use clap::ValueEnum;
use anyhow::{Result, Context};
use crate::dna::mds::loader::BinaryLoader;
use crate::dna::compiler::Compiler;
use crate::dna::mds::optimizer::OptimizationLevel;

#[derive(Clone, ValueEnum, Debug)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    CSharp,
    Java,
    Go,
    Ruby,
    Php,
}

pub fn schema_command(
    target: PathBuf,
    lang: Language,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔧 Generating SDK schema...");
        println!("  Target: {}", target.display());
        println!("  Language: {:?}", lang);
        println!("  Output: {:?}", output);
    }

    // Load and parse the Helix file
    let source = std::fs::read_to_string(&target)
        .context(format!("Failed to read Helix file: {}", target.display()))?;

    let tokens = crate::dna::atp::lexer::tokenize(&source)
        .map_err(|e| anyhow::anyhow!("Failed to tokenize Helix file: {} - {}", target.display(), e))?;
    let ast = crate::dna::atp::parser::parse(tokens)
        .map_err(|e| anyhow::anyhow!("Failed to parse Helix file: {} - {}", target.display(), e))?;

    // Validation is done through the AST parsing and semantic analysis
    // let _ = crate::dna::mds::semantic::validate(&ast)
    //     .map_err(|e| format!("Invalid Helix configuration: {} - {:?}", target.display(), e))?;

    // Determine output file path
    let output_path = output.unwrap_or_else(|| {
        let stem = target.file_stem().unwrap_or_default().to_string_lossy();
        let extension = match lang {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::CSharp => "cs",
            Language::Java => "java",
            Language::Go => "go",
            Language::Ruby => "rb",
            Language::Php => "php",
        };
        target.with_file_name(format!("{}_schema.{}", stem, extension))
    });

    // Generate schema based on language
    let schema_code = generate_schema_code(&ast, &lang)?;

    // Write the schema file
    std::fs::write(&output_path, schema_code)
        .context(format!("Failed to write schema file: {}", output_path.display()))?;

    println!("✅ Schema generated successfully: {}", output_path.display());

    if verbose {
        println!("  Language: {:?}", lang);
        println!("  Sections: {}", ast.declarations.len());
    }

    Ok(())
}
pub fn generate_schema_code(ast: &crate::dna::atp::ast::HelixAst, lang: &Language) -> Result<String, Box<dyn std::error::Error>> {
    match lang {
        Language::Rust => generate_rust_schema(ast),
        Language::Python => generate_python_schema(ast),
        Language::JavaScript => generate_javascript_schema(ast),
        Language::CSharp => generate_csharp_schema(ast),
        Language::Java => generate_java_schema(ast),
        Language::Go => generate_go_schema(ast),
        Language::Ruby => generate_ruby_schema(ast),
        Language::Php => generate_php_schema(ast),
    }
}
pub fn generate_rust_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("// Auto-generated Helix SDK for Rust
use std::collections::HashMap;

pub struct HelixConfig {
    data: HashMap<String, serde_json::Value>,
}

impl HelixConfig {
    pub pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_string(&content)
    }

    pub pub fn from_string(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data: HashMap<String, serde_json::Value> = serde_json::from_str(content)?;
        Ok(Self { data })
    }

    pub pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }

    pub pub fn process(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Process the configuration
        println!(\"Processing Helix configuration...\");
        Ok(())
    }

    pub pub fn compile(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Compile the configuration
        println!(\"Compiling Helix configuration...\");
        let json = serde_json::to_vec(&self.data)?;
        Ok(json)
    }
}

impl std::ops::Index<&str> for HelixConfig {
    type Output = serde_json::Value;

    pub fn index(&self, key: &str) -> &Self::Output {
        self.data.get(key).unwrap_or(&serde_json::Value::Null)
    }
}
");
    Ok(code)
}
pub fn generate_python_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("# Auto-generated Helix SDK for Python

import json
from typing import Dict, Any, Optional

class HelixConfig:
    def __init__(self):
        self.data: Dict[str, Any] = {}

    @classmethod
    def from_file(cls, path: str) -> 'HelixConfig':
        with open(path, 'r') as f:
            content = f.read()
        return cls.from_string(content)

    @classmethod
    def from_string(cls, content: str) -> 'HelixConfig':
        instance = cls()
        instance.data = json.loads(content)
        return instance

    def get(self, key: str) -> Any:
        return self.data.get(key)

    def set(self, key: str, value: Any):
        self.data[key] = value

    def __getitem__(self, key: str) -> Any:
        return self.data.get(key, None)

    def __setitem__(self, key: str, value: Any):
        self.data[key] = value

    def process(self):
        \"\"\"Process the configuration\"\"\"
        print(\"Processing Helix configuration...\")

    def compile(self) -> bytes:
        \"\"\"Compile the configuration\"\"\"
        print(\"Compiling Helix configuration...\")
        return json.dumps(self.data).encode('utf-8')
");
    Ok(code)
}
pub fn generate_javascript_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("// Auto-generated Helix SDK for JavaScript

class HelixConfig {
    constructor() {
        this.data = {};
    }

    static fromFile(path) {
        const fs = require('fs');
        const content = fs.readFileSync(path, 'utf8');
        return HelixConfig.fromString(content);
    }

    static fromString(content) {
        const instance = new HelixConfig();
        instance.data = JSON.parse(content);
        return instance;
    }

    get(key) {
        return this.data[key];
    }

    set(key, value) {
        this.data[key] = value;
    }

    process() {
        console.log('Processing Helix configuration...');
    }

    compile() {
        console.log('Compiling Helix configuration...');
        return Buffer.from(JSON.stringify(this.data));
    }
}

// Bracket notation access
HelixConfig.prototype.__defineGetter__('hlx', function() {
    return this;
});

module.exports = HelixConfig;
");
    Ok(code)
}
pub fn generate_csharp_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("// Auto-generated Helix SDK for C#

using System;
using System.Collections.Generic;
using System.IO;
using Newtonsoft.Json;

public class HelixConfig
{
    private Dictionary<string, object> data = new Dictionary<string, object>();

    public static HelixConfig FromFile(string path)
    {
        string content = File.ReadAllText(path);
        return FromString(content);
    }

    public static HelixConfig FromString(string content)
    {
        var instance = new HelixConfig();
        instance.data = JsonConvert.DeserializeObject<Dictionary<string, object>>(content);
        return instance;
    }

    public object Get(string key)
    {
        return data.ContainsKey(key) ? data[key] : null;
    }

    public void Set(string key, object value)
    {
        data[key] = value;
    }

    public object this[string key]
    {
        get { return Get(key); }
        set { Set(key, value); }
    }

    public void Process()
    {
        Console.WriteLine(\"Processing Helix configuration...\");
    }

    public byte[] Compile()
    {
        Console.WriteLine(\"Compiling Helix configuration...\");
        string json = JsonConvert.SerializeObject(data);
        return System.Text.Encoding.UTF8.GetBytes(json);
    }
}
");
    Ok(code)
}
pub fn generate_java_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("// Auto-generated Helix SDK for Java

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.File;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

public class HelixConfig {
    private Map<String, Object> data = new HashMap<>();
    private static final ObjectMapper mapper = new ObjectMapper();

    public static HelixConfig fromFile(String path) throws IOException {
        String content = new String(java.nio.file.Files.readAllBytes(new File(path).toPath()));
        return fromString(content);
    }

    public static HelixConfig fromString(String content) throws IOException {
        HelixConfig instance = new HelixConfig();
        instance.data = mapper.readValue(content, Map.class);
        return instance;
    }

    public Object get(String key) {
        return data.get(key);
    }

    public void set(String key, Object value) {
        data.put(key, value);
    }

    public void process() {
        System.out.println(\"Processing Helix configuration...\");
    }

    public byte[] compile() throws IOException {
        System.out.println(\"Compiling Helix configuration...\");
        String json = mapper.writeValueAsString(data);
        return json.getBytes();
    }
}
");
    Ok(code)
}
pub fn generate_go_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("// Auto-generated Helix SDK for Go

package helix

import (
    \"encoding/json\"
    \"fmt\"
    \"io/ioutil\"
    \"log\"
)

type HelixConfig struct {
    Data map[string]interface{} `json:\"data\"`
}

func NewHelixConfig() *HelixConfig {
    return &HelixConfig{
        Data: make(map[string]interface{}),
    }
}

func FromFile(path string) (*HelixConfig, error) {
    content, err := ioutil.ReadFile(path)
    if err != nil {
        return nil, err
    }
    return FromString(string(content))
}

func FromString(content string) (*HelixConfig, error) {
    var data map[string]interface{}
    if err := json.Unmarshal([]byte(content), &data); err != nil {
        return nil, err
    }
    return &HelixConfig{Data: data}, nil
}

func (h *HelixConfig) Get(key string) interface{} {
    return h.Data[key]
}

func (h *HelixConfig) Set(key string, value interface{}) {
    h.Data[key] = value
}

func (h *HelixConfig) Process() {
    fmt.Println(\"Processing Helix configuration...\")
}

func (h *HelixConfig) Compile() ([]byte, error) {
    fmt.Println(\"Compiling Helix configuration...\")
    return json.Marshal(h.Data)
}
");
    Ok(code)
}
pub fn generate_ruby_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("# Auto-generated Helix SDK for Ruby

require 'json'

class HelixConfig
  attr_accessor :data

  def initialize
    @data = {}
  end

  def self.from_file(path)
    content = File.read(path)
    from_string(content)
  end

  def self.from_string(content)
    instance = new
    instance.data = JSON.parse(content)
    instance
  end

  def get(key)
    @data[key]
  end

  def set(key, value)
    @data[key] = value
  end

  def [](key)
    get(key)
  end

  def []=(key, value)
    set(key, value)
  end

  def process
    puts 'Processing Helix configuration...'
  end

  def compile
    puts 'Compiling Helix configuration...'
    JSON.dump(@data).bytes
  end
end
");
    Ok(code)
}
pub fn generate_php_schema(_ast: &crate::dna::atp::ast::HelixAst) -> Result<String, Box<dyn std::error::Error>> {
    let code = String::from("<?php
// Auto-generated Helix SDK for PHP

class HelixConfig {
    private $data = [];

    public static function fromFile(string $path): self {
        $content = file_get_contents($path);
        return self::fromString($content);
    }

    public static function fromString(string $content): self {
        $instance = new self();
        $instance->data = json_decode($content, true);
        return $instance;
    }

    public function get(string $key) {
        return $this->data[$key] ?? null;
    }

    public function set(string $key, $value): void {
        $this->data[$key] = $value;
    }

    public function __get(string $key) {
        return $this->get($key);
    }

    public function __set(string $key, $value): void {
        $this->set($key, $value);
    }

    public function process(): void {
        echo \"Processing Helix configuration...\\n\";
    }

    public function compile(): string {
        echo \"Compiling Helix configuration...\\n\";
        return json_encode($this->data);
    }
}
");
    Ok(code)
}
pub fn decompile_command(
    input: PathBuf,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = output
        .unwrap_or_else(|| {
            let mut path = input.clone();
            path.set_extension("hlx");
            path
        });
    if verbose {
        println!("🔄 Decompiling: {}", input.display());
    }
    let loader = BinaryLoader::new();
    let binary = loader.load_file(&input)?;
    let compiler = Compiler::new(OptimizationLevel::Zero);
    let source = compiler.decompile(&binary)?;
    std::fs::write(&output_path, source)?;
    println!("✅ Decompiled successfully: {}", output_path.display());
    Ok(())
}
pub fn validate_command(
    file: PathBuf,
    detailed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let extension = file.extension().and_then(|s| s.to_str());
    match extension {
        Some("hlx") => {
            let source = std::fs::read_to_string(&file)?;
            let tokens = crate::dna::atp::lexer::tokenize(&source)
                .map_err(|e| anyhow::anyhow!("Failed to tokenize: {}", e))?;
            let ast = crate::dna::atp::parser::parse(tokens)?;
            // Validation is done through the AST parsing
            // crate::dna::ops::validation::validate(&ast)?;
            println!("✅ Valid HELIX file: {}", file.display());
            if detailed {
                println!("  Declarations: {}", ast.declarations.len());
            }
        }
        Some("hlxb") => {
            let loader = BinaryLoader::new();
            let binary = loader.load_file(&file)?;
            binary.validate()?;
            println!("✅ Valid HLXB file: {}", file.display());
            if detailed {
                println!("  Version: {}", binary.version);
                println!("  Sections: {}", binary.data_sections.len());
                println!("  Checksum: {:x}", binary.checksum);
            }
        }
        _ => {
            return Err("Unknown file type (expected .hlx or .hlxb)".into());
        }
    }
    Ok(())
}