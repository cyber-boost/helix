use crate::codegen::HelixIR;
use crate::compiler::binary::{
    HelixBinary, BinaryFlags, BinaryMetadata, DataSection, SectionType, SymbolTable,
    Instruction, Value, CompressionMethod,
};
use std::path::Path;
use std::fs::File;
use std::io::{Write, Read};
use bincode;
pub struct BinarySerializer {
    enable_compression: bool,
    compression_method: CompressionMethod,
}
impl BinarySerializer {
    pub fn new(enable_compression: bool) -> Self {
        Self {
            enable_compression,
            compression_method: CompressionMethod::Lz4,
        }
    }
    pub fn with_compression_method(mut self, method: CompressionMethod) -> Self {
        self.compression_method = method;
        self
    }
    pub fn serialize(
        &self,
        ir: HelixIR,
        source_path: Option<&Path>,
    ) -> Result<HelixBinary, SerializationError> {
        let mut binary = HelixBinary::new();
        binary.metadata = BinaryMetadata {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            source_hash: self.calculate_source_hash(&ir),
            optimization_level: 2,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            source_path: source_path.map(|p| p.display().to_string()),
            extra: Default::default(),
        };
        binary.flags = BinaryFlags {
            compressed: self.enable_compression,
            optimized: true,
            encrypted: false,
            signed: false,
            custom: 0,
        };
        binary.symbol_table = self.convert_symbol_table(&ir);
        binary.data_sections = self.create_data_sections(&ir)?;
        if self.enable_compression {
            for section in &mut binary.data_sections {
                section.compress(self.compression_method.clone())?;
            }
        }
        binary.checksum = binary.calculate_checksum();
        Ok(binary)
    }
    pub fn write_to_file(
        &self,
        binary: &HelixBinary,
        path: &Path,
    ) -> Result<(), SerializationError> {
        let data = bincode::serialize(binary)
            .map_err(|e| SerializationError::BincodeError(e.to_string()))?;
        let mut file = File::create(path)
            .map_err(|e| SerializationError::IoError(e.to_string()))?;
        file.write_all(&data).map_err(|e| SerializationError::IoError(e.to_string()))?;
        Ok(())
    }
    pub fn read_from_file(
        &self,
        path: &Path,
    ) -> Result<HelixBinary, SerializationError> {
        let mut file = File::open(path)
            .map_err(|e| SerializationError::IoError(e.to_string()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| SerializationError::IoError(e.to_string()))?;
        let binary: HelixBinary = bincode::deserialize(&data)
            .map_err(|e| SerializationError::BincodeError(e.to_string()))?;
        binary.validate().map_err(|e| SerializationError::ValidationError(e))?;
        Ok(binary)
    }
    pub fn deserialize_to_ir(
        &self,
        binary: &HelixBinary,
    ) -> Result<HelixIR, SerializationError> {
        let mut ir = HelixIR {
            version: binary.version,
            metadata: self.convert_metadata(&binary.metadata),
            symbol_table: self.convert_symbol_table_to_ir(&binary.symbol_table),
            instructions: Vec::new(),
            string_pool: crate::codegen::StringPool {
                strings: binary.symbol_table.strings.clone(),
                index: binary.symbol_table.string_map.clone(),
            },
            constants: crate::codegen::ConstantPool::new(),
        };
        for section in &binary.data_sections {
            let mut section_clone = section.clone();
            if section.compression.is_some() {
                section_clone
                    .decompress()
                    .map_err(|e| SerializationError::DecompressionError(e))?;
            }
            match section.section_type {
                SectionType::Instructions => {
                    ir.instructions = self
                        .deserialize_instructions(&section_clone.data)?;
                }
                _ => {}
            }
        }
        Ok(ir)
    }
    fn convert_symbol_table(&self, ir: &HelixIR) -> SymbolTable {
        let mut table = SymbolTable::default();
        table.strings = ir.string_pool.strings.clone();
        for (i, s) in table.strings.iter().enumerate() {
            table.string_map.insert(s.clone(), i as u32);
        }
        for (id, agent) in &ir.symbol_table.agents {
            if let Some(name) = ir.string_pool.get(agent.name_idx) {
                table.agents.insert(name.clone(), *id);
            }
        }
        for (id, workflow) in &ir.symbol_table.workflows {
            if let Some(name) = ir.string_pool.get(workflow.name_idx) {
                table.workflows.insert(name.clone(), *id);
            }
        }
        for (id, context) in &ir.symbol_table.contexts {
            if let Some(name) = ir.string_pool.get(context.name_idx) {
                table.contexts.insert(name.clone(), *id);
            }
        }
        for (id, crew) in &ir.symbol_table.crews {
            if let Some(name) = ir.string_pool.get(crew.name_idx) {
                table.crews.insert(name.clone(), *id);
            }
        }
        table
    }
    fn convert_symbol_table_to_ir(
        &self,
        table: &SymbolTable,
    ) -> crate::codegen::SymbolTable {
        use crate::codegen::{AgentSymbol, WorkflowSymbol, ContextSymbol, CrewSymbol};
        use std::collections::HashMap;
        let mut symbol_table = crate::codegen::SymbolTable::default();
        for (name, id) in &table.agents {
            let name_idx = table.string_map.get(name).copied().unwrap_or(0);
            symbol_table
                .agents
                .insert(
                    *id,
                    AgentSymbol {
                        id: *id,
                        name_idx,
                        model_idx: 0,
                        role_idx: 0,
                        temperature: None,
                        max_tokens: None,
                        capabilities: Vec::new(),
                        backstory_idx: None,
                    },
                );
        }
        for (name, id) in &table.workflows {
            let name_idx = table.string_map.get(name).copied().unwrap_or(0);
            symbol_table
                .workflows
                .insert(
                    *id,
                    WorkflowSymbol {
                        id: *id,
                        name_idx,
                        trigger_type: crate::codegen::TriggerType::Manual,
                        steps: Vec::new(),
                        pipeline: None,
                    },
                );
        }
        for (name, id) in &table.contexts {
            let name_idx = table.string_map.get(name).copied().unwrap_or(0);
            symbol_table
                .contexts
                .insert(
                    *id,
                    ContextSymbol {
                        id: *id,
                        name_idx,
                        environment_idx: 0,
                        debug: false,
                        max_tokens: None,
                        secrets: HashMap::new(),
                    },
                );
        }
        for (name, id) in &table.crews {
            let name_idx = table.string_map.get(name).copied().unwrap_or(0);
            symbol_table
                .crews
                .insert(
                    *id,
                    CrewSymbol {
                        id: *id,
                        name_idx,
                        agent_ids: Vec::new(),
                        process_type: crate::codegen::ProcessTypeIR::Sequential,
                        manager_id: None,
                    },
                );
        }
        symbol_table
    }
    fn convert_metadata(&self, metadata: &BinaryMetadata) -> crate::codegen::Metadata {
        crate::codegen::Metadata {
            source_file: metadata.source_path.clone(),
            compile_time: metadata.created_at,
            compiler_version: metadata.compiler_version.clone(),
            checksum: None,
        }
    }
    fn create_data_sections(
        &self,
        ir: &HelixIR,
    ) -> Result<Vec<DataSection>, SerializationError> {
        let mut sections = Vec::new();
        if !ir.instructions.is_empty() {
            let instruction_data = self.serialize_instructions(&ir.instructions)?;
            sections.push(DataSection::new(SectionType::Instructions, instruction_data));
        }
        if !ir.symbol_table.agents.is_empty() {
            let agent_data = bincode::serialize(&ir.symbol_table.agents)
                .map_err(|e| SerializationError::BincodeError(e.to_string()))?;
            sections.push(DataSection::new(SectionType::Agents, agent_data));
        }
        if !ir.symbol_table.workflows.is_empty() {
            let workflow_data = bincode::serialize(&ir.symbol_table.workflows)
                .map_err(|e| SerializationError::BincodeError(e.to_string()))?;
            sections.push(DataSection::new(SectionType::Workflows, workflow_data));
        }
        Ok(sections)
    }
    fn serialize_instructions(
        &self,
        instructions: &[crate::codegen::Instruction],
    ) -> Result<Vec<u8>, SerializationError> {
        let binary_instructions: Vec<Instruction> = instructions
            .iter()
            .map(|inst| self.convert_instruction(inst))
            .collect();
        bincode::serialize(&binary_instructions)
            .map_err(|e| SerializationError::BincodeError(e.to_string()))
    }
    fn deserialize_instructions(
        &self,
        data: &[u8],
    ) -> Result<Vec<crate::codegen::Instruction>, SerializationError> {
        let binary_instructions: Vec<Instruction> = bincode::deserialize(data)
            .map_err(|e| SerializationError::BincodeError(e.to_string()))?;
        Ok(
            binary_instructions
                .iter()
                .map(|inst| self.convert_instruction_to_ir(inst))
                .collect(),
        )
    }
    fn convert_instruction(&self, inst: &crate::codegen::Instruction) -> Instruction {
        match inst {
            crate::codegen::Instruction::DeclareAgent(id) => {
                Instruction::InvokeAgent(*id)
            }
            crate::codegen::Instruction::DeclareWorkflow(_id) => Instruction::Nop,
            crate::codegen::Instruction::DeclareContext(_id) => Instruction::Nop,
            crate::codegen::Instruction::DeclareCrew(id) => Instruction::InvokeCrew(*id),
            crate::codegen::Instruction::SetProperty { .. } => Instruction::Nop,
            crate::codegen::Instruction::SetCapability { .. } => Instruction::Nop,
            crate::codegen::Instruction::SetSecret { .. } => Instruction::Nop,
            crate::codegen::Instruction::DefineStep { .. } => Instruction::Nop,
            crate::codegen::Instruction::DefinePipeline { workflow, .. } => {
                Instruction::Pipeline(*workflow)
            }
            crate::codegen::Instruction::ResolveReference { .. } => Instruction::Nop,
            crate::codegen::Instruction::SetMetadata { .. } => Instruction::Nop,
        }
    }
    fn convert_instruction_to_ir(
        &self,
        inst: &Instruction,
    ) -> crate::codegen::Instruction {
        match inst {
            Instruction::InvokeAgent(id) => {
                crate::codegen::Instruction::DeclareAgent(*id)
            }
            Instruction::InvokeCrew(id) => crate::codegen::Instruction::DeclareCrew(*id),
            Instruction::Pipeline(id) => {
                crate::codegen::Instruction::DeclareWorkflow(*id)
            }
            _ => crate::codegen::Instruction::DeclareAgent(0),
        }
    }
    #[allow(dead_code)]
    fn convert_value(&self, val: &crate::types::Value) -> Value {
        match val {
            crate::types::Value::Bool(b) => Value::Bool(*b),
            crate::types::Value::Number(n) => Value::Float(*n),
            crate::types::Value::String(_s) => {
                let id = 0;
                Value::String(id)
            }
            crate::types::Value::Duration(d) => {
                let secs = match &d.unit {
                    crate::types::TimeUnit::Seconds => d.value,
                    crate::types::TimeUnit::Minutes => d.value * 60,
                    crate::types::TimeUnit::Hours => d.value * 3600,
                    crate::types::TimeUnit::Days => d.value * 86400,
                } as u64;
                Value::Duration(secs)
            }
            crate::types::Value::Array(_) => Value::Null,
            crate::types::Value::Object(_) => Value::Null,
            crate::types::Value::Reference(_) => Value::Null,
        }
    }
    #[allow(dead_code)]
    fn convert_value_to_ir(&self, val: &Value) -> crate::types::Value {
        match val {
            Value::Null => crate::types::Value::String(String::new()),
            Value::Bool(b) => crate::types::Value::Bool(*b),
            Value::Int(i) => crate::types::Value::Number(*i as f64),
            Value::Float(f) => crate::types::Value::Number(*f),
            Value::String(_id) => crate::types::Value::String(String::new()),
            Value::Duration(secs) => {
                crate::types::Value::Duration(crate::types::Duration {
                    value: (*secs / 60) as u64,
                    unit: crate::types::TimeUnit::Minutes,
                })
            }
            Value::Reference(_id) => crate::types::Value::Reference(String::new()),
            Value::Array(arr) => {
                crate::types::Value::Array(
                    arr.iter().map(|v| self.convert_value_to_ir(v)).collect(),
                )
            }
            Value::Object(obj) => {
                let mut map = std::collections::HashMap::new();
                for (key_idx, value) in obj {
                    let key = format!("key_{}", key_idx);
                    map.insert(key, self.convert_value_to_ir(value));
                }
                crate::types::Value::Object(map)
            }
        }
    }
    fn calculate_source_hash(&self, ir: &HelixIR) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        ir.version.hash(&mut hasher);
        ir.string_pool.strings.len().hash(&mut hasher);
        ir.instructions.len().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
#[derive(Debug)]
pub enum SerializationError {
    IoError(String),
    BincodeError(String),
    CompressionError(String),
    DecompressionError(String),
    ValidationError(String),
}
impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::BincodeError(e) => write!(f, "Bincode error: {}", e),
            Self::CompressionError(e) => write!(f, "Compression error: {}", e),
            Self::DecompressionError(e) => write!(f, "Decompression error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}
impl std::error::Error for SerializationError {}
impl From<String> for SerializationError {
    fn from(s: String) -> Self {
        Self::CompressionError(s)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{StringPool, Metadata, ConstantPool};
    #[test]
    fn test_serialization_roundtrip() {
        let mut string_pool = StringPool::new();
        string_pool.intern("test");
        let ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table: crate::codegen::SymbolTable::default(),
            instructions: vec![
                crate ::codegen::Instruction::DeclareAgent(1), crate
                ::codegen::Instruction::DeclareWorkflow(2),
            ],
            string_pool,
            constants: ConstantPool::default(),
        };
        let serializer = BinarySerializer::new(false);
        let binary = serializer.serialize(ir.clone(), None).unwrap();
        let deserialized = serializer.deserialize_to_ir(&binary).unwrap();
        assert_eq!(ir.version, deserialized.version);
        assert_eq!(ir.instructions.len(), deserialized.instructions.len());
    }
}