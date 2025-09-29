use std::collections::HashMap;
use std::io::{Read, Write, Seek, SeekFrom};
use serde::{Deserialize, Serialize};
use crate::dna::hel::error::HlxError;
pub use crate::dna::atp::types::{AgentConfig, WorkflowConfig, CrewConfig, ContextConfig, HelixConfig};
#[cfg(feature = "zstd")]
use zstd::{Encoder, Decoder};
#[cfg(feature = "lz4_flex")]
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
#[cfg(feature = "flate2")]
use flate2::{Compression, write::GzEncoder, read::GzDecoder};
#[cfg(feature = "bincode")]
use bincode::{serialize, deserialize};
#[cfg(feature = "crc32fast")]
use crc32fast::Hasher as Crc32Hasher;

/// HLXB Config Format (.hlxb files) - Binary configuration
/// This handles compact binary configuration files with .hlxb extension

/// HLXB Format Magic Header
pub const HLXB_MAGIC: &[u8; 4] = b"HLXB";
pub const HLXB_VERSION: u8 = 1;

/// HLXB Config structure (same as HLX but optimized for binary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxbConfig {
    /// Configuration sections
    pub sections: HashMap<String, HlxbSection>,
    /// Global metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxbSection {
    /// Section properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Section metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// HLXB File Header
#[derive(Debug)]
pub struct HlxbHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub created_at: u64,
    pub section_count: u32,
}

/// Section type identifiers
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SectionType {
    Agents = 0x01,
    Workflows = 0x02,
    Crews = 0x03,
    Contexts = 0x04,
    Metadata = 0x05,
}

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionAlgorithm {
    None,
    Lz4,
    Zstd,
    Gzip,
}

impl CompressionAlgorithm {
    /// Select the best compression algorithm based on data characteristics
    pub fn select_best(data: &[u8]) -> Self {
        let size = data.len();

        // For very small data, compression overhead isn't worth it
        if size < 1024 {
            return CompressionAlgorithm::None;
        }

        // For medium data, LZ4 provides good speed/compression ratio
        if size < 64 * 1024 {
            return CompressionAlgorithm::Lz4;
        }

        // For larger data, ZSTD provides better compression
        if size < 1024 * 1024 {
            return CompressionAlgorithm::Zstd;
        }

        // For very large data, GZIP provides maximum compression
        CompressionAlgorithm::Gzip
    }

    /// Get compression level based on algorithm
    pub fn get_level(&self) -> u32 {
        match self {
            CompressionAlgorithm::None => 0,
            CompressionAlgorithm::Lz4 => 1, // LZ4 doesn't have levels, this is just for consistency
            CompressionAlgorithm::Zstd => 3, // Good balance of speed/compression
            CompressionAlgorithm::Gzip => 6, // Default gzip compression level
        }
    }
}

/// Compression manager for HLXB format
pub struct CompressionManager;

impl CompressionManager {
    /// Compress data using the specified algorithm
    pub fn compress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>, HlxError> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Lz4 => {
                #[cfg(feature = "lz4_flex")]
                {
                    Ok(compress_prepend_size(data))
                }
                #[cfg(not(feature = "lz4_flex"))]
                {
                    Err(HlxError::feature_error("lz4_flex", "LZ4 compression requires lz4_flex feature"))
                }
            }
            CompressionAlgorithm::Zstd => {
                #[cfg(feature = "zstd")]
                {
                    let mut encoder = Encoder::new(Vec::new(), algorithm.get_level() as i32)
                        .map_err(|e| HlxError::compression_error(
                            format!("Failed to create ZSTD encoder: {}", e),
                            "Check zstd library"
                        ))?;

                    encoder.write_all(data)
                        .map_err(|e| HlxError::compression_error(
                            format!("ZSTD compression failed: {}", e),
                            "Data may be corrupted"
                        ))?;

                    encoder.finish()
                        .map_err(|e| HlxError::compression_error(
                            format!("Failed to finish ZSTD compression: {}", e),
                            "Compression process failed"
                        ))
                }
                #[cfg(not(feature = "zstd"))]
                {
                    Err(HlxError::feature_error("zstd", "ZSTD compression requires zstd feature"))
                }
            }
            CompressionAlgorithm::Gzip => {
                #[cfg(feature = "flate2")]
                {
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(algorithm.get_level()));
                    std::io::copy(&mut std::io::Cursor::new(data), &mut encoder)
                        .map_err(|e| HlxError::compression_error(
                            format!("GZIP compression failed: {}", e),
                            "Data may be corrupted"
                        ))?;
                    encoder.finish()
                        .map_err(|e| HlxError::compression_error(
                            format!("Failed to finish GZIP compression: {}", e),
                            "Compression process failed"
                        ))
                }
                #[cfg(not(feature = "flate2"))]
                {
                    Err(HlxError::feature_error("flate2", "GZIP compression requires flate2 feature"))
                }
            }
        }
    }

    /// Decompress data using the specified algorithm
    pub fn decompress(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>, HlxError> {
        match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Lz4 => {
                #[cfg(feature = "lz4_flex")]
                {
                    decompress_size_prepended(data).map_err(|e| {
                        HlxError::decompression_error(
                            format!("LZ4 decompression failed: {}", e),
                            "Data may be corrupted or compressed with different settings"
                        )
                    })
                }
                #[cfg(not(feature = "lz4_flex"))]
                {
                    Err(HlxError::feature_error("lz4_flex", "LZ4 decompression requires lz4_flex feature"))
                }
            }
            CompressionAlgorithm::Zstd => {
                #[cfg(feature = "zstd")]
                {
                    let mut decoder = Decoder::new(data)
                        .map_err(|e| HlxError::decompression_error(
                            format!("Failed to create ZSTD decoder: {}", e),
                            "Data may be corrupted"
                        ))?;

                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed)
                        .map_err(|e| HlxError::decompression_error(
                            format!("ZSTD decompression failed: {}", e),
                            "Data may be corrupted or compressed with incompatible settings"
                        ))?;
                    Ok(decompressed)
                }
                #[cfg(not(feature = "zstd"))]
                {
                    Err(HlxError::feature_error("zstd", "ZSTD decompression requires zstd feature"))
                }
            }
            CompressionAlgorithm::Gzip => {
                #[cfg(feature = "flate2")]
                {
                    let mut decoder = GzDecoder::new(data);
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed)
                        .map_err(|e| HlxError::decompression_error(
                            format!("GZIP decompression failed: {}", e),
                            "Data may be corrupted or compressed with different settings"
                        ))?;
                    Ok(decompressed)
                }
                #[cfg(not(feature = "flate2"))]
                {
                    Err(HlxError::feature_error("flate2", "GZIP decompression requires flate2 feature"))
                }
            }
        }
    }

    /// Benchmark compression algorithms for given data and return the best one
    pub fn benchmark_and_select(data: &[u8]) -> CompressionAlgorithm {
        if data.len() < 1024 {
            return CompressionAlgorithm::None;
        }

        let algorithms = [
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ];

        let mut best_algorithm = CompressionAlgorithm::None;
        let mut best_ratio = 1.0;

        for algorithm in algorithms.iter() {
            if let Ok(compressed) = Self::compress(data, *algorithm) {
                let ratio = compressed.len() as f64 / data.len() as f64;
                if ratio < best_ratio {
                    best_ratio = ratio;
                    best_algorithm = *algorithm;
                }
            }
        }

        // If no compression algorithm works, fall back to none
        if best_algorithm == CompressionAlgorithm::None && data.len() >= 1024 {
            // Try to at least use LZ4 if available
            #[cfg(feature = "lz4_flex")]
            { CompressionAlgorithm::Lz4 }
            #[cfg(not(feature = "lz4_flex"))]
            { CompressionAlgorithm::None }
        } else {
            best_algorithm
        }
    }
}

/// Section header for each data section
#[derive(Debug)]
pub struct SectionHeader {
    pub section_type: SectionType,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub crc32_checksum: u32,
}

/// HLXB Writer for creating binary config files
pub struct HlxbWriter<W: Write + Seek> {
    writer: W,
    section_count: u32,
}

impl<W: Write + Seek> HlxbWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer, section_count: 0 }
    }

    pub fn write_header(&mut self) -> Result<(), HlxError> {
        // Write magic number
        self.writer.write_all(HLXB_MAGIC)?;

        // Write version
        self.writer.write_all(&[HLXB_VERSION])?;

        // Write timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.writer.write_all(&timestamp.to_le_bytes())?;

        // Write placeholder for section count (will be updated in finalize)
        self.writer.write_all(&0u32.to_le_bytes())?;

        Ok(())
    }

    /// Write a section with compression and checksum
    fn write_section<T: Serialize>(&mut self, section_type: SectionType, data: &T) -> Result<(), HlxError> {
        #[cfg(not(feature = "bincode"))]
        return Err(HlxError::feature_error("bincode", "Binary serialization requires bincode feature"));

        #[cfg(feature = "bincode")]
        {
            // Serialize data to binary
            let serialized_data = serialize(data)
                .map_err(|e| HlxError::serialization_error(
                    format!("Failed to serialize section data: {}", e),
                    "Check data structure"
                ))?;

            let uncompressed_size = serialized_data.len() as u64;

            // Compress data
            #[cfg(not(feature = "zstd"))]
            let compressed_data = serialized_data.clone();
            #[cfg(feature = "zstd")]
            let compressed_data = {
                let mut encoder = Encoder::new(Vec::new(), 3)
                    .map_err(|e| HlxError::compression_error(
                        format!("Failed to create ZSTD encoder: {}", e),
                        "Check zstd library"
                    ))?;
                encoder.write_all(&serialized_data)
                    .map_err(|e| HlxError::compression_error(
                        format!("Failed to compress data: {}", e),
                        "Data may be corrupted"
                    ))?;
                encoder.finish()
                    .map_err(|e| HlxError::compression_error(
                        format!("Failed to finish compression: {}", e),
                        "Compression process failed"
                    ))?
            };

            let compressed_size = compressed_data.len() as u64;

            // Calculate CRC32 checksum
            #[cfg(not(feature = "crc32fast"))]
            let checksum = 0u32;
            #[cfg(feature = "crc32fast")]
            let checksum = {
                let mut hasher = Crc32Hasher::new();
                hasher.update(&compressed_data);
                hasher.finalize()
            };

            // Write section header
            self.writer.write_all(&[section_type as u8])?;
            self.writer.write_all(&uncompressed_size.to_le_bytes())?;
            self.writer.write_all(&compressed_size.to_le_bytes())?;
            self.writer.write_all(&checksum.to_le_bytes())?;

            // Write compressed data
            self.writer.write_all(&compressed_data)?;

            self.section_count += 1;

            Ok(())
        }
    }

    pub fn write_agents(&mut self, agents: &HashMap<String, AgentConfig>) -> Result<(), HlxError> {
        self.write_section(SectionType::Agents, agents)
    }

    pub fn write_workflows(&mut self, workflows: &HashMap<String, WorkflowConfig>) -> Result<(), HlxError> {
        self.write_section(SectionType::Workflows, workflows)
    }

    pub fn write_crews(&mut self, crews: &HashMap<String, CrewConfig>) -> Result<(), HlxError> {
        self.write_section(SectionType::Crews, crews)
    }

    pub fn write_contexts(&mut self, contexts: &HashMap<String, ContextConfig>) -> Result<(), HlxError> {
        self.write_section(SectionType::Contexts, contexts)
    }

    pub fn write_metadata(&mut self, metadata: &HashMap<String, serde_json::Value>) -> Result<(), HlxError> {
        self.write_section(SectionType::Metadata, metadata)
    }

    pub fn finalize(&mut self) -> Result<(), HlxError> {
        // Seek back to the section count position in header (after magic + version + timestamp)
        let section_count_pos = HLXB_MAGIC.len() + 1 + 8; // magic(4) + version(1) + timestamp(8)
        self.writer.seek(SeekFrom::Start(section_count_pos as u64))?;

        // Write the actual section count
        self.writer.write_all(&self.section_count.to_le_bytes())?;

        Ok(())
    }
}

/// HLXB Reader for parsing binary config files
pub struct HlxbReader<R: Read + Seek> {
    reader: R,
}

impl<R: Read + Seek> HlxbReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_header(&mut self) -> Result<HlxbHeader, HlxError> {
        // Read magic number
        let mut magic = [0u8; 4];
        self.reader.read_exact(&mut magic)?;

        // Validate magic number
        if magic != *HLXB_MAGIC {
            return Err(HlxError::validation_error(
                "Invalid HLXB magic number",
                "File does not appear to be a valid .hlxb config file"
            ));
        }

        // Read version
        let mut version = [0u8; 1];
        self.reader.read_exact(&mut version)?;

        // Validate version
        if version[0] != HLXB_VERSION {
            return Err(HlxError::validation_error(
                format!("Unsupported HLXB version: {} (expected {})", version[0], HLXB_VERSION),
                "File was created with an incompatible version of Helix"
            ));
        }

        // Read timestamp
        let mut timestamp_bytes = [0u8; 8];
        self.reader.read_exact(&mut timestamp_bytes)?;
        let created_at = u64::from_le_bytes(timestamp_bytes);

        // Read section count
        let mut section_count_bytes = [0u8; 4];
        self.reader.read_exact(&mut section_count_bytes)?;
        let section_count = u32::from_le_bytes(section_count_bytes);

        Ok(HlxbHeader {
            magic,
            version: version[0],
            created_at,
            section_count,
        })
    }

    /// Read a section header
    fn read_section_header(&mut self) -> Result<SectionHeader, HlxError> {
        let mut section_type_byte = [0u8; 1];
        self.reader.read_exact(&mut section_type_byte)?;

        let section_type = match section_type_byte[0] {
            0x01 => SectionType::Agents,
            0x02 => SectionType::Workflows,
            0x03 => SectionType::Crews,
            0x04 => SectionType::Contexts,
            0x05 => SectionType::Metadata,
            _ => return Err(HlxError::validation_error(
                format!("Unknown section type: 0x{:02x}", section_type_byte[0]),
                "File contains an unsupported section type"
            )),
        };

        let mut uncompressed_size_bytes = [0u8; 8];
        self.reader.read_exact(&mut uncompressed_size_bytes)?;
        let uncompressed_size = u64::from_le_bytes(uncompressed_size_bytes);

        let mut compressed_size_bytes = [0u8; 8];
        self.reader.read_exact(&mut compressed_size_bytes)?;
        let compressed_size = u64::from_le_bytes(compressed_size_bytes);

        let mut checksum_bytes = [0u8; 4];
        self.reader.read_exact(&mut checksum_bytes)?;
        let crc32_checksum = u32::from_le_bytes(checksum_bytes);

        Ok(SectionHeader {
            section_type,
            uncompressed_size,
            compressed_size,
            crc32_checksum,
        })
    }

    /// Read and decompress section data
    fn read_section_data(&mut self, header: &SectionHeader) -> Result<Vec<u8>, HlxError> {
        // Read compressed data
        let mut compressed_data = vec![0u8; header.compressed_size as usize];
        self.reader.read_exact(&mut compressed_data)?;

        // Verify checksum
        #[cfg(feature = "crc32fast")]
        {
            let mut hasher = Crc32Hasher::new();
            hasher.update(&compressed_data);
            let calculated_checksum = hasher.finalize();
            if calculated_checksum != header.crc32_checksum {
                return Err(HlxError::validation_error(
                    format!("CRC32 checksum mismatch: expected {}, got {}", header.crc32_checksum, calculated_checksum),
                    "File may be corrupted"
                ));
            }
        }

        // Decompress data
        #[cfg(not(feature = "zstd"))]
        let decompressed_data = compressed_data;
        #[cfg(feature = "zstd")]
        let decompressed_data = {
            let mut decoder = Decoder::new(&compressed_data[..])
                .map_err(|e| HlxError::decompression_error(
                    format!("Failed to create ZSTD decoder: {}", e),
                    "File may be corrupted"
                ))?;
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)
                .map_err(|e| HlxError::decompression_error(
                    format!("Failed to decompress data: {}", e),
                    "File may be corrupted or compressed with incompatible settings"
                ))?;
            decompressed
        };

        // Verify decompressed size
        if decompressed_data.len() != header.uncompressed_size as usize {
            return Err(HlxError::validation_error(
                format!("Decompressed size mismatch: expected {}, got {}", header.uncompressed_size, decompressed_data.len()),
                "File may be corrupted"
            ));
        }

        Ok(decompressed_data)
    }

    pub fn read_config(&mut self) -> Result<HelixConfig, HlxError> {
        #[cfg(not(feature = "bincode"))]
        return Err(HlxError::feature_error("bincode", "Binary deserialization requires bincode feature"));

        #[cfg(feature = "bincode")]
        {
            let header = self.read_header()?;

            let mut config = HelixConfig::default();

            for _ in 0..header.section_count {
                let section_header = self.read_section_header()?;
                let section_data = self.read_section_data(&section_header)?;

                match section_header.section_type {
                    SectionType::Agents => {
                        let agents: HashMap<String, AgentConfig> = deserialize(&section_data)
                            .map_err(|e| HlxError::deserialization_error(
                                format!("Failed to deserialize agents section: {}", e),
                                "Check agents configuration"
                            ))?;
                        config.agents = agents;
                    }
                    SectionType::Workflows => {
                        let workflows: HashMap<String, WorkflowConfig> = deserialize(&section_data)
                            .map_err(|e| HlxError::deserialization_error(
                                format!("Failed to deserialize workflows section: {}", e),
                                "Check workflows configuration"
                            ))?;
                        config.workflows = workflows;
                    }
                    SectionType::Crews => {
                        let crews: HashMap<String, CrewConfig> = deserialize(&section_data)
                            .map_err(|e| HlxError::deserialization_error(
                                format!("Failed to deserialize crews section: {}", e),
                                "Check crews configuration"
                            ))?;
                        config.crews = crews;
                    }
                    SectionType::Contexts => {
                        let contexts: HashMap<String, ContextConfig> = deserialize(&section_data)
                            .map_err(|e| HlxError::deserialization_error(
                                format!("Failed to deserialize contexts section: {}", e),
                                "Check contexts configuration"
                            ))?;
                        config.contexts = contexts;
                    }
                    SectionType::Metadata => {
                        // HelixConfig doesn't have a metadata field, skip for now
                        // TODO: Add metadata support to HelixConfig if needed
                        let _metadata: HashMap<String, serde_json::Value> = deserialize(&section_data)
                            .map_err(|e| HlxError::deserialization_error(
                                format!("Failed to deserialize metadata section: {}", e),
                                "Check metadata configuration"
                            ))?;
                    }
                }
            }

            Ok(config)
        }
    }
}

/// HLXB Config Reader/Writer
pub struct HlxbConfigHandler;

impl HlxbConfigHandler {
    /// Read HLXB config from file
    pub fn read_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<HlxbConfig, HlxError> {
        let mut file = std::fs::File::open(&path)
            .map_err(|e| HlxError::io_error(
                format!("Failed to open HLXB config file: {}", e),
                format!("Check if file exists: {}", path.as_ref().display())
            ))?;

        Self::read_from_reader(&mut file)
    }

    /// Write HLXB config to file
    pub fn write_to_file<P: AsRef<std::path::Path>>(config: &HlxbConfig, path: P) -> Result<(), HlxError> {
        let mut file = std::fs::File::create(&path)
            .map_err(|e| HlxError::io_error(
                format!("Failed to create HLXB config file: {}", e),
                format!("Check write permissions: {}", path.as_ref().display())
            ))?;

        Self::write_to_writer(config, &mut file)
    }

    /// Write a complete HelixConfig to binary format
    pub fn write_helix_config<W: Write + Seek>(config: &HelixConfig, writer: &mut W) -> Result<(), HlxError> {
        let mut hlxb_writer = HlxbWriter::new(writer);
        hlxb_writer.write_header()?;

        // Write all sections
        hlxb_writer.write_agents(&config.agents)?;
        hlxb_writer.write_workflows(&config.workflows)?;
        hlxb_writer.write_crews(&config.crews)?;
        hlxb_writer.write_contexts(&config.contexts)?;

        hlxb_writer.finalize()
    }

    /// Read a complete HelixConfig from binary format
    pub fn read_helix_config<R: Read + Seek>(reader: &mut R) -> Result<HelixConfig, HlxError> {
        let mut hlxb_reader = HlxbReader::new(reader);
        hlxb_reader.read_config()
    }

    /// Read from any reader (legacy method for HlxbConfig)
    pub fn read_from_reader<R: Read + Seek>(reader: &mut R) -> Result<HlxbConfig, HlxError> {
        // For legacy compatibility, this reads the old format
        // Read magic number
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != *HLXB_MAGIC {
            return Err(HlxError::validation_error(
                "Invalid HLXB magic number",
                "File does not appear to be a valid .hlxb config file"
            ));
        }

        // Read version
        let mut version = [0u8; 1];
        reader.read_exact(&mut version)?;
        if version[0] != HLXB_VERSION {
            return Err(HlxError::validation_error(
                format!("Unsupported HLXB version: {}", version[0]),
                "Only version 1 is supported"
            ));
        }

        // Read data length
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let data_len = u64::from_le_bytes(len_bytes) as usize;

        // Read compressed data
        let mut compressed_data = vec![0u8; data_len];
        reader.read_exact(&mut compressed_data)?;

        // Select decompression algorithm based on data characteristics
        let algorithm = CompressionAlgorithm::select_best(&compressed_data);
        let decompressed_data = CompressionManager::decompress(&compressed_data, algorithm)?;

        // Convert to string
        let json_data = String::from_utf8(decompressed_data)
            .map_err(|e| HlxError::validation_error(
                format!("Invalid UTF-8 in HLXB file: {}", e),
                "File may be corrupted"
            ))?;

        // Parse JSON
        serde_json::from_str(&json_data)
            .map_err(|e| HlxError::json_error(
                format!("Failed to parse HLXB config: {}", e),
                "Check file format"
            ))
    }

    /// Write to any writer (legacy method for HlxbConfig)
    pub fn write_to_writer<W: Write + Seek>(config: &HlxbConfig, writer: &mut W) -> Result<(), HlxError> {
        // Write magic number
        writer.write_all(HLXB_MAGIC)?;

        // Write version
        writer.write_all(&[HLXB_VERSION])?;

        // Serialize to JSON
        let json_data = serde_json::to_string(config)
            .map_err(|e| HlxError::json_error(e.to_string(), ""))?;

        // Compress using automatic algorithm selection
        let algorithm = CompressionAlgorithm::select_best(json_data.as_bytes());
        let compressed_data = CompressionManager::compress(json_data.as_bytes(), algorithm)?;

        // Write data length
        let data_len = compressed_data.len() as u64;
        writer.write_all(&data_len.to_le_bytes())?;

        // Write compressed data
        writer.write_all(&compressed_data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_hlxb_roundtrip() {
        let mut config = HlxbConfig::default();
        config.metadata.insert("version".to_string(), serde_json::Value::String("1.0".to_string()));

        let mut db_section = HlxbSection::default();
        db_section.properties.insert("host".to_string(), serde_json::Value::String("localhost".to_string()));
        db_section.properties.insert("port".to_string(), serde_json::Value::Number(5432.into()));

        config.sections.insert("database".to_string(), db_section);

        // Test write to buffer
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            HlxbConfigHandler::write_to_writer(&config, &mut cursor).unwrap();
        }

        // Test read from buffer
        {
            let mut cursor = Cursor::new(&buffer);
            let read_config = HlxbConfigHandler::read_from_reader(&mut cursor).unwrap();

            assert_eq!(read_config.metadata.get("version").unwrap().as_str().unwrap(), "1.0");
            assert!(read_config.sections.contains_key("database"));

            let db_section = &read_config.sections["database"];
            assert_eq!(db_section.properties.get("host").unwrap().as_str().unwrap(), "localhost");
            assert_eq!(db_section.properties.get("port").unwrap().as_i64().unwrap(), 5432);
        }
    }

    #[test]
    fn test_compression_algorithms() {
        let test_data = b"Hello, World! This is a test string for compression algorithms.";
        let large_data = vec![b'A'; 100_000]; // 100KB of 'A' characters

        // Test LZ4 compression/decompression
        #[cfg(feature = "lz4_flex")]
        {
            let compressed = CompressionManager::compress(test_data, CompressionAlgorithm::Lz4).unwrap();
            let decompressed = CompressionManager::decompress(&compressed, CompressionAlgorithm::Lz4).unwrap();
            assert_eq!(decompressed, test_data);
            println!("✅ LZ4 compression test passed");
        }

        // Test ZSTD compression/decompression
        #[cfg(feature = "zstd")]
        {
            let compressed = CompressionManager::compress(test_data, CompressionAlgorithm::Zstd).unwrap();
            let decompressed = CompressionManager::decompress(&compressed, CompressionAlgorithm::Zstd).unwrap();
            assert_eq!(decompressed, test_data);
            println!("✅ ZSTD compression test passed");
        }

        // Test GZIP compression/decompression
        #[cfg(feature = "flate2")]
        {
            let compressed = CompressionManager::compress(test_data, CompressionAlgorithm::Gzip).unwrap();
            let decompressed = CompressionManager::decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
            assert_eq!(decompressed, test_data);
            println!("✅ GZIP compression test passed");
        }

        // Test no compression
        let compressed = CompressionManager::compress(test_data, CompressionAlgorithm::None).unwrap();
        let decompressed = CompressionManager::decompress(&compressed, CompressionAlgorithm::None).unwrap();
        assert_eq!(decompressed, test_data);
        println!("✅ No compression test passed");

        // Test algorithm selection
        let algorithm = CompressionAlgorithm::select_best(test_data);
        assert_ne!(algorithm, CompressionAlgorithm::None); // Should select compression for this size
        println!("✅ Algorithm selection test passed: {:?}", algorithm);

        // Test algorithm selection for small data
        let small_data = b"small";
        let algorithm = CompressionAlgorithm::select_best(small_data);
        assert_eq!(algorithm, CompressionAlgorithm::None); // Should not compress small data
        println!("✅ Small data algorithm selection test passed");

        // Test benchmark and select
        let best_algorithm = CompressionManager::benchmark_and_select(&large_data);
        println!("✅ Benchmark and select test passed: {:?}", best_algorithm);
    }

    #[test]
    fn test_compressed_hlxb_config_roundtrip() {
        let mut config = HlxbConfig::default();
        config.metadata.insert("version".to_string(), serde_json::Value::String("1.0".to_string()));
        config.metadata.insert("compressed".to_string(), serde_json::Value::Bool(true));

        let db_section = HlxbSection {
            properties: {
                let mut props = HashMap::new();
                props.insert("host".to_string(), serde_json::Value::String("localhost".to_string()));
                props.insert("port".to_string(), serde_json::Value::Number(5432.into()));
                // Add more data to make compression worthwhile
                props.insert("description".to_string(), serde_json::Value::String("A very long description that should benefit from compression. ".repeat(50)));
                props
            },
            metadata: Some({
                let mut meta = HashMap::new();
                meta.insert("created".to_string(), serde_json::Value::String("2024-01-01".to_string()));
                meta
            }),
        };

        config.sections.insert("database".to_string(), db_section);

        // Test write to buffer
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            HlxbConfigHandler::write_to_writer(&config, &mut cursor).unwrap();
        }

        println!("✅ Compressed HLXB config written, size: {} bytes", buffer.len());

        // Test read from buffer
        {
            let mut cursor = Cursor::new(&buffer);
            let read_config = HlxbConfigHandler::read_from_reader(&mut cursor).unwrap();

            assert_eq!(read_config.metadata.get("version").unwrap().as_str().unwrap(), "1.0");
            assert_eq!(read_config.metadata.get("compressed").unwrap().as_bool().unwrap(), true);
            assert!(read_config.sections.contains_key("database"));

            let db_section = &read_config.sections["database"];
            assert_eq!(db_section.properties.get("host").unwrap().as_str().unwrap(), "localhost");
            assert_eq!(db_section.properties.get("port").unwrap().as_i64().unwrap(), 5432);
            assert!(db_section.metadata.is_some());
            assert_eq!(db_section.metadata.as_ref().unwrap().get("created").unwrap().as_str().unwrap(), "2024-01-01");
        }

        println!("✅ Compressed HLXB config roundtrip test passed");
    }
}

impl Default for HlxbConfig {
    fn default() -> Self {
        Self {
            sections: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl Default for HlxbSection {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
            metadata: None,
        }
    }
}
