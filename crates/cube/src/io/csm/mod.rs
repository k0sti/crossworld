// CSM format parsing and serialization

pub mod parser;
pub mod serializer;

// Re-export main functions and types
pub use parser::{parse_csm, CsmError};
pub use serializer::serialize_csm;
