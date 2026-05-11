mod naive;
mod indexed;

use std::path::Path;
use anyhow::{Result, Context};
use rio_turtle::{TurtleParser, TurtleError};
use rio_api::parser::TriplesParser;
use rio_api::model::{Subject, Term};

pub use gemtext_ld::RdfNode;
pub use naive::NaiveStore;
pub use indexed::IndexedStore;

/// The default store implementation used throughout the server.
pub type Store = IndexedStore;

/// Trait defining the common interface for RDF triple stores.
pub trait TripleStore: Send + Sync {
    /// Load triples from a file path.
    fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = std::fs::read_to_string(path).context("Failed to read RDF file")?;
        self.load_from_string(&content)
    }

    /// Load triples from a string (Turtle or RDF/XML with automatic fallback).
    fn load_from_string(&mut self, content: &str) -> Result<()>;

    /// Number of triples in the store.
    fn triple_count(&self) -> usize;

    /// Get all (predicate, object) pairs for a given subject IRI.
    fn get_resource_description(&self, iri: &str) -> Vec<(String, RdfNode)>;

    /// Get all distinct subject IRIs, sorted.
    fn get_all_subjects(&self) -> Vec<String>;
}

/// Parsed triple with owned strings, used as the intermediate representation
/// between RDF parsers and store implementations.
pub(crate) struct ParsedTriple {
    pub subject: String,
    pub predicate: String,
    pub object: RdfNode,
}

/// Parse an RDF string (Turtle with RDF/XML fallback) into a vector of owned triples.
///
/// This is shared by both store implementations so the parsing logic isn't duplicated.
pub(crate) fn parse_rdf_string(content: &str) -> Result<Vec<ParsedTriple>> {
    let mut triples = Vec::new();

    let mut parser = TurtleParser::new(content.as_bytes(), None);
    let turtle_result = parser.parse_all(&mut |t| {
        let s = match t.subject {
            Subject::NamedNode(n) => n.iri.to_string(),
            Subject::BlankNode(b) => b.id.to_string(),
            Subject::Triple(_) => "triple".to_string(),
        };

        let p = t.predicate.iri.to_string();

        let o = match t.object {
            Term::NamedNode(n) => RdfNode::Iri(n.iri.to_string()),
            Term::BlankNode(b) => RdfNode::BlankNode(b.id.to_string()),
            Term::Literal(l) => match l {
                rio_api::model::Literal::Simple { value } => RdfNode::SimpleLiteral(value.to_string()),
                rio_api::model::Literal::LanguageTaggedString { value, language } => RdfNode::LanguageTaggedLiteral(value.to_string(), language.to_string()),
                rio_api::model::Literal::Typed { value, datatype } => RdfNode::DatatypedLiteral(value.to_string(), datatype.iri.to_string()),
            },
            Term::Triple(_) => RdfNode::SimpleLiteral("triple".to_string()),
        };

        triples.push(ParsedTriple { subject: s, predicate: p, object: o });
        Ok(()) as Result<(), TurtleError>
    });

    if let Err(turtle_err) = turtle_result {
        // Turtle parsing failed, clear any partial results and try RDF/XML
        triples.clear();
        let mut xml_parser = rio_xml::RdfXmlParser::new(content.as_bytes(), None);
        xml_parser.parse_all(&mut |t| {
            let s = match t.subject {
                Subject::NamedNode(n) => n.iri.to_string(),
                Subject::BlankNode(b) => b.id.to_string(),
                Subject::Triple(_) => "triple".to_string(),
            };

            let p = t.predicate.iri.to_string();

            let o = match t.object {
                Term::NamedNode(n) => RdfNode::Iri(n.iri.to_string()),
                Term::BlankNode(b) => RdfNode::BlankNode(b.id.to_string()),
                Term::Literal(l) => match l {
                    rio_api::model::Literal::Simple { value } => RdfNode::SimpleLiteral(value.to_string()),
                    rio_api::model::Literal::LanguageTaggedString { value, language } => RdfNode::LanguageTaggedLiteral(value.to_string(), language.to_string()),
                    rio_api::model::Literal::Typed { value, datatype } => RdfNode::DatatypedLiteral(value.to_string(), datatype.iri.to_string()),
                },
                Term::Triple(_) => RdfNode::SimpleLiteral("triple".to_string()),
            };

            triples.push(ParsedTriple { subject: s, predicate: p, object: o });
            Ok(()) as Result<(), rio_xml::RdfXmlError>
        }).context(format!("Failed to parse as Turtle ({}) and fallback to RDF/XML also failed", turtle_err))?;
    }

    Ok(triples)
}
