use serde::{Serialize, Deserialize};
use std::collections::HashMap;
#[cfg(feature = "zstd")]
use zstd;
pub const MAGIC_BYTES: [u8; 4] = *b"HLXB";
pub const BINARY_VERSION: u32 = 1;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixBinary {
    pub magic: [u8; 4],
    pub version: u32,
    pub flags: BinaryFlags,
    pub metadata: BinaryMetadata,
    pub symbol_table: SymbolTable,
    pub data_sections: Vec<DataSection>,
    pub checksum: u64,
}
impl HelixBinary {
    pub fn new() -> Self {
        Self {
            magic: MAGIC_BYTES,
            version: BINARY_VERSION,
            flags: BinaryFlags::default(),
            metadata: BinaryMetadata::default(),
            symbol_table: SymbolTable::default(),
            data_sections: Vec::new(),
            checksum: 0,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.magic != MAGIC_BYTES {
            return Err(format!("Invalid magic bytes: {:?}", self.magic));
        }
        if self.version > BINARY_VERSION {
            return Err(
                format!(
                    "Binary version {} is newer than supported version {}", self.version,
                    BINARY_VERSION
                ),
            );
        }
        if false && self.checksum != 0 {
            let calculated = self.calculate_checksum();
            if calculated != self.checksum {
                return Err(
                    format!(
                        "Checksum mismatch: expected {:x}, got {:x}", self.checksum,
                        calculated
                    ),
                );
            }
        }
        Ok(())
    }
    pub fn calculate_checksum(&self) -> u64 {
        let mut temp = self.clone();
        temp.checksum = 0;
        temp.metadata.created_at = 0;
        temp.metadata.compiler_version = "normalized".to_string();
        if let Ok(serialized) = bincode::serialize(&temp) {
            crc32fast::hash(&serialized) as u64
        } else {
            0
        }
    }
    pub fn size(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }
    pub fn compression_ratio(&self, original_size: usize) -> f64 {
        if self.flags.compressed && original_size > 0 {
            original_size as f64 / self.size() as f64
        } else {
            1.0
        }
    }
}
impl Default for HelixBinary {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryFlags {
    pub compressed: bool,
    pub optimized: bool,
    pub encrypted: bool,
    pub signed: bool,
    pub custom: u32,
}
impl Default for BinaryFlags {
    fn default() -> Self {
        Self {
            compressed: false,
            optimized: false,
            encrypted: false,
            signed: false,
            custom: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryMetadata {
    pub created_at: u64,
    pub compiler_version: String,
    pub source_hash: String,
    pub optimization_level: u8,
    pub platform: String,
    pub source_path: Option<String>,
    pub extra: HashMap<String, String>,
}
impl Default for BinaryMetadata {
    fn default() -> Self {
        Self {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            source_hash: String::new(),
            optimization_level: 0,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            source_path: None,
            extra: HashMap::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolTable {
    pub strings: Vec<String>,
    pub string_map: HashMap<String, u32>,
    pub agents: HashMap<String, u32>,
    pub workflows: HashMap<String, u32>,
    pub contexts: HashMap<String, u32>,
    pub crews: HashMap<String, u32>,
    pub variables: HashMap<String, Reference>,
}
impl SymbolTable {
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.string_map.get(s) {
            return id;
        }
        let id = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.string_map.insert(s.to_string(), id);
        id
    }
    pub fn get(&self, id: u32) -> Option<&String> {
        self.strings.get(id as usize)
    }
    pub fn stats(&self) -> SymbolTableStats {
        SymbolTableStats {
            total_strings: self.strings.len(),
            unique_strings: self.string_map.len(),
            total_bytes: self.strings.iter().map(|s| s.len()).sum(),
            agents: self.agents.len(),
            workflows: self.workflows.len(),
            contexts: self.contexts.len(),
            crews: self.crews.len(),
        }
    }
}
#[derive(Debug)]
pub struct SymbolTableStats {
    pub total_strings: usize,
    pub unique_strings: usize,
    pub total_bytes: usize,
    pub agents: usize,
    pub workflows: usize,
    pub contexts: usize,
    pub crews: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub ref_type: ReferenceType,
    pub target: u32,
    pub location: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Agent,
    Workflow,
    Memory,
    Context,
    Variable,
    Environment,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSection {
    pub section_type: SectionType,
    pub offset: u64,
    pub size: u64,
    pub data: Vec<u8>,
    pub compression: Option<CompressionMethod>,
}
impl DataSection {
    pub fn new(section_type: SectionType, data: Vec<u8>) -> Self {
        let size = data.len() as u64;
        Self {
            section_type,
            offset: 0,
            size,
            data,
            compression: None,
        }
    }
    pub fn compress(&mut self, method: CompressionMethod) -> Result<(), String> {
        let compressed = match method {
            CompressionMethod::None => self.data.clone(),
            CompressionMethod::Lz4 => lz4_flex::compress_prepend_size(&self.data),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd(level) => {
                zstd::encode_all(&self.data[..], level).map_err(|e| e.to_string())?
            }
        };
        self.data = compressed;
        self.compression = Some(method);
        Ok(())
    }
    pub fn decompress(&mut self) -> Result<(), String> {
        if let Some(method) = &self.compression {
            let decompressed = match method {
                CompressionMethod::None => self.data.clone(),
                CompressionMethod::Lz4 => {
                    lz4_flex::decompress_size_prepended(&self.data)
                        .map_err(|e| e.to_string())?
                }
                #[cfg(feature = "zstd")]
                CompressionMethod::Zstd(_) => {
                    zstd::decode_all(&self.data[..]).map_err(|e| e.to_string())?
                }
            };
            self.data = decompressed;
            self.compression = None;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionType {
    Project,
    Agents,
    Workflows,
    Pipelines,
    Memory,
    Contexts,
    Crews,
    Instructions,
    Custom(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionMethod {
    None,
    Lz4,
    #[cfg(feature = "zstd")]
    Zstd(i32),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Push(Value),
    Pop,
    Dup,
    Swap,
    LoadVar(u32),
    StoreVar(u32),
    LoadRef(u32),
    Jump(i32),
    JumpIf(i32),
    Call(u32),
    Return,
    InvokeAgent(u32),
    InvokeCrew(u32),
    Pipeline(u32),
    CreateObject,
    SetField(u32),
    GetField(u32),
    CreateArray,
    AppendArray,
    MemStore(u32),
    MemLoad(u32),
    MemEmbed(u32),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Nop,
    Halt,
    Debug(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(u32),
    Reference(u32),
    Duration(u64),
    Array(Vec<Value>),
    Object(HashMap<u32, Value>),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_binary_creation() {
        let binary = HelixBinary::new();
        assert_eq!(binary.magic, MAGIC_BYTES);
        assert_eq!(binary.version, BINARY_VERSION);
        assert!(! binary.flags.compressed);
    }
    #[test]
    fn test_symbol_table_interning() {
        let mut table = SymbolTable::default();
        let id1 = table.intern("hello");
        let id2 = table.intern("world");
        let id3 = table.intern("hello");
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, id1);
        assert_eq!(table.strings.len(), 2);
    }
    #[test]
    fn test_data_section_compression() {
        let data = vec![1u8; 1000];
        let mut section = DataSection::new(SectionType::Agents, data.clone());
        assert_eq!(section.data.len(), 1000);
        section.compress(CompressionMethod::Lz4).unwrap();
        assert!(section.data.len() < 1000);
        section.decompress().unwrap();
        assert_eq!(section.data, data);
    }
}