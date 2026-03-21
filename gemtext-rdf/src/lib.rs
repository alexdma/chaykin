//! RDF serialization and parsing for Gemtext format.
//!
//! This crate provides a serializer that converts RDF triples into
//! [Gemtext](https://geminiprotocol.net/docs/gemtext-specification.gmi)
//! and a parser that reconstructs RDF triples from Gemtext documents.
//!
//! Two serialization modes are supported:
//! - **Expanded**: one Gemtext line per triple
//! - **Condensed**: triples grouped by predicate under headings
//!
//! # Example
//!
//! ```
//! use gemtext_rdf::{RdfNode, RdfTriple, SerializationMode, serialize, parse};
//!
//! let triples = vec![
//!     RdfTriple::new(
//!         "http://example.org/Alice",
//!         "http://xmlns.com/foaf/0.1/name",
//!         RdfNode::SimpleLiteral("Alice".into()),
//!     ),
//! ];
//!
//! let gemtext = serialize(&triples, SerializationMode::Expanded, &None);
//! let roundtripped = parse(&gemtext);
//! assert_eq!(roundtripped[0].object, triples[0].object);
//! ```

pub mod model;
pub mod prefixes;
pub mod serializer;
pub mod parser;

// Re-exports for convenience
pub use model::{RdfNode, RdfTriple};
pub use prefixes::{shorten_uri, expand_uri};
pub use serializer::{SerializationMode, serialize};
pub use parser::parse;
