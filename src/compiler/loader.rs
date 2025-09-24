use crate::compiler::binary::{HelixBinary, DataSection, SectionType};
use crate::types::HelixConfig;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use memmap2::MmapOptions;
use bincode;
pub struct BinaryLoader {
    enable_mmap: bool,
    enable_lazy: bool,
    _cache_enabled: bool,
}
impl BinaryLoader {
    pub fn new() -> Self {
        Self {
            enable_mmap: true,
            enable_lazy: false,
            _cache_enabled: true,
        }
    }
    pub fn with_mmap(mut self, enable: bool) -> Self {
        self.enable_mmap = enable;
        self
    }
    pub fn with_lazy(mut self, enable: bool) -> Self {
        self.enable_lazy = enable;
        self
    }
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<HelixBinary, LoadError> {
        let path = path.as_ref();
        if self.enable_mmap {
            self.load_with_mmap(path)
        } else {
            self.load_standard(path)
        }
    }
    fn load_with_mmap(&self, path: &Path) -> Result<HelixBinary, LoadError> {
        let file = File::open(path)
            .map_err(|e| LoadError::IoError(format!("Failed to open file: {}", e)))?;
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| LoadError::MmapError(format!("Failed to map file: {}", e)))?
        };
        let binary: HelixBinary = bincode::deserialize(&mmap)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize: {}", e),
            ))?;
        binary.validate().map_err(|e| LoadError::ValidationError(e))?;
        Ok(binary)
    }
    fn load_standard(&self, path: &Path) -> Result<HelixBinary, LoadError> {
        let mut file = File::open(path)
            .map_err(|e| LoadError::IoError(format!("Failed to open file: {}", e)))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| LoadError::IoError(format!("Failed to read file: {}", e)))?;
        let binary: HelixBinary = bincode::deserialize(&buffer)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize: {}", e),
            ))?;
        binary.validate().map_err(|e| LoadError::ValidationError(e))?;
        Ok(binary)
    }
    pub fn load_to_config<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<HelixConfig, LoadError> {
        let binary = self.load_file(path)?;
        self.binary_to_config(binary)
    }
    pub fn binary_to_config(
        &self,
        binary: HelixBinary,
    ) -> Result<HelixConfig, LoadError> {
        let mut config = HelixConfig::default();
        for section in binary.data_sections {
            let mut section = section;
            if section.compression.is_some() {
                section.decompress().map_err(|e| LoadError::DecompressionError(e))?;
            }
            match section.section_type {
                SectionType::Agents => {
                    self.load_agents_section(&section, &mut config)?;
                }
                SectionType::Workflows => {
                    self.load_workflows_section(&section, &mut config)?;
                }
                SectionType::Contexts => {
                    self.load_contexts_section(&section, &mut config)?;
                }
                SectionType::Memory => {
                    self.load_memory_section(&section, &mut config)?;
                }
                SectionType::Crews => {
                    self.load_crews_section(&section, &mut config)?;
                }
                SectionType::Pipelines => {
                    self.load_pipelines_section(&section, &mut config)?;
                }
                _ => {}
            }
        }
        Ok(config)
    }
    fn load_agents_section(
        &self,
        section: &DataSection,
        config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::AgentConfig;
        let agents: HashMap<String, AgentConfig> = bincode::deserialize(&section.data)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize agents: {}", e),
            ))?;
        for (name, agent) in agents {
            config.agents.insert(name, agent);
        }
        Ok(())
    }
    fn load_workflows_section(
        &self,
        section: &DataSection,
        config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::WorkflowConfig;
        let workflows: HashMap<String, WorkflowConfig> = bincode::deserialize(
                &section.data,
            )
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize workflows: {}", e),
            ))?;
        for (name, workflow) in workflows {
            config.workflows.insert(name, workflow);
        }
        Ok(())
    }
    fn load_contexts_section(
        &self,
        section: &DataSection,
        config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::ContextConfig;
        let contexts: HashMap<String, ContextConfig> = bincode::deserialize(
                &section.data,
            )
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize contexts: {}", e),
            ))?;
        for (name, context) in contexts {
            config.contexts.insert(name, context);
        }
        Ok(())
    }
    fn load_memory_section(
        &self,
        section: &DataSection,
        config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::MemoryConfig;
        let memory: MemoryConfig = bincode::deserialize(&section.data)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize memory config: {}", e),
            ))?;
        config.memory = Some(memory);
        Ok(())
    }
    fn load_crews_section(
        &self,
        section: &DataSection,
        config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::CrewConfig;
        let crews: HashMap<String, CrewConfig> = bincode::deserialize(&section.data)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize crews: {}", e),
            ))?;
        for (name, crew) in crews {
            config.crews.insert(name, crew);
        }
        Ok(())
    }
    fn load_pipelines_section(
        &self,
        section: &DataSection,
        _config: &mut HelixConfig,
    ) -> Result<(), LoadError> {
        use crate::types::PipelineConfig;
        let pipelines: Vec<PipelineConfig> = bincode::deserialize(&section.data)
            .map_err(|e| LoadError::DeserializationError(
                format!("Failed to deserialize pipelines: {}", e),
            ))?;
        if !pipelines.is_empty() {
            eprintln!(
                "Warning: Loaded {} pipelines but HelixConfig has no dedicated pipeline storage",
                pipelines.len()
            );
        }
        Ok(())
    }
}
impl Default for BinaryLoader {
    fn default() -> Self {
        Self::new()
    }
}
pub struct LazyBinaryLoader {
    binary: HelixBinary,
    loaded_sections: std::collections::HashSet<usize>,
}
impl LazyBinaryLoader {
    pub fn new(binary: HelixBinary) -> Self {
        Self {
            binary,
            loaded_sections: std::collections::HashSet::new(),
        }
    }
    pub fn load_section(
        &mut self,
        section_type: SectionType,
    ) -> Result<&DataSection, LoadError> {
        for (idx, section) in self.binary.data_sections.iter_mut().enumerate() {
            if std::mem::discriminant(&section.section_type)
                == std::mem::discriminant(&section_type)
            {
                if !self.loaded_sections.contains(&idx) {
                    if section.compression.is_some() {
                        section
                            .decompress()
                            .map_err(|e| LoadError::DecompressionError(e))?;
                    }
                    self.loaded_sections.insert(idx);
                }
                return Ok(section);
            }
        }
        Err(LoadError::SectionNotFound(format!("{:?}", section_type)))
    }
    pub fn is_loaded(&self, section_type: &SectionType) -> bool {
        for (idx, section) in self.binary.data_sections.iter().enumerate() {
            if std::mem::discriminant(&section.section_type)
                == std::mem::discriminant(section_type)
            {
                return self.loaded_sections.contains(&idx);
            }
        }
        false
    }
    pub fn stats(&self) -> LoaderStats {
        LoaderStats {
            total_sections: self.binary.data_sections.len(),
            loaded_sections: self.loaded_sections.len(),
            total_size: self.binary.size(),
            loaded_size: self.calculate_loaded_size(),
        }
    }
    fn calculate_loaded_size(&self) -> usize {
        self.binary
            .data_sections
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.loaded_sections.contains(idx))
            .map(|(_, section)| section.size as usize)
            .sum()
    }
}
#[derive(Debug)]
pub struct LoaderStats {
    pub total_sections: usize,
    pub loaded_sections: usize,
    pub total_size: usize,
    pub loaded_size: usize,
}
#[derive(Debug)]
pub enum LoadError {
    IoError(String),
    MmapError(String),
    DeserializationError(String),
    DecompressionError(String),
    ValidationError(String),
    SectionNotFound(String),
}
impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::MmapError(e) => write!(f, "Memory mapping error: {}", e),
            Self::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            Self::DecompressionError(e) => write!(f, "Decompression error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
            Self::SectionNotFound(e) => write!(f, "Section not found: {}", e),
        }
    }
}
impl std::error::Error for LoadError {}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_loader_creation() {
        let loader = BinaryLoader::new();
        assert!(loader.enable_mmap);
        assert!(! loader.enable_lazy);
        assert!(loader.cache_enabled);
    }
    #[test]
    fn test_loader_builder() {
        let loader = BinaryLoader::new().with_mmap(false).with_lazy(true);
        assert!(! loader.enable_mmap);
        assert!(loader.enable_lazy);
    }
}