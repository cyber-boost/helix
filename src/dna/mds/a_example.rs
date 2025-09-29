// src/mds/emb.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

/// The optional key/value bag you can store alongside every record.
pub type Metadata = HashMap<String, serde_json::Value>;

/// Core document that we want to index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier (string works for all back‑ends)
    pub id: String,

    /// Optional raw text – handy for debugging or fallback keyword search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Dense vector representation.
    ///
    ///  * In the "plain" JSON formats we keep it as `Vec<f32>`.
    ///  * In the base‑64 variant we use a custom serializer that writes the
    ///    vector as a base64‑encoded blob.
    #[serde(flatten)]
    pub embedding: Embedding,

    /// Arbitrary key/value pairs that you may want to filter on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// (Optional) provenance – e.g., URL or file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Enum that abstracts over the two ways we can store a vector:
/// * `Raw`  – a normal JSON array of numbers (used for NDJSON / array mode)
/// * `B64`  – a base64‑encoded binary blob (used for the compact format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "vector_storage", rename_all = "snake_case")]
pub enum Embedding {
    /// `embedding` field holds the plain float array.
    Raw {
        embedding: Vec<f32>,
    },

    /// `embedding` field holds a base64 string encoding the raw `f32` bytes.
    ///
    /// For now, we store the base64-encoded binary blob as a string.
    /// TODO: Implement proper base64 serialization with serde_with
    B64 {
        embedding: String,
    },
}

impl Document {
    /// Helper to create a Document with a *raw* vector.
    pub fn new_raw<S: Into<String>>(id: S, text: Option<String>, vec: Vec<f32>, meta: Option<Metadata>) -> Self {
        Document {
            id: id.into(),
            text,
            embedding: Embedding::Raw { embedding: vec },
            metadata: meta,
            source: None,
        }
    }

    /// Helper to create a Document with a *base64* vector.
    pub fn new_b64<S: Into<String>>(id: S, text: Option<String>, vec: Vec<f32>, meta: Option<Metadata>) -> Self {
        // Convert Vec<f32> to bytes (little-endian)
        let bytes: Vec<u8> = vec.iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect();

        // Encode as base64
        let b64_string = general_purpose::STANDARD.encode(&bytes);

        Document {
            id: id.into(),
            text,
            embedding: Embedding::B64 { embedding: b64_string },
            metadata: meta,
            source: None,
        }
    }
}


    