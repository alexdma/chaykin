use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use rio_turtle::{TurtleParser, TurtleError};
use rio_api::parser::TriplesParser;
use rio_api::model::{Subject, Term};

pub use gemtext_rdf::RdfNode;

pub struct Store {
    triples: Vec<(String, String, RdfNode)>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            triples: Vec::new(),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let content = fs::read_to_string(path).context("Failed to read turtle file")?;
        self.load_from_string(&content)
    }

    pub fn load_from_string(&mut self, content: &str) -> Result<()> {
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
            
            triples.push((s, p, o));
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
                
                triples.push((s, p, o));
                Ok(()) as Result<(), rio_xml::RdfXmlError>
            }).context(format!("Failed to parse as Turtle ({}) and fallback to RDF/XML also failed", turtle_err))?;
        }

        self.triples.extend(triples);
        Ok(())
    }

    pub fn triple_count(&self) -> usize {
        self.triples.len()
    }

    pub fn get_resource_description(&self, iri: &str) -> Vec<(String, RdfNode)> {
        let mut results = Vec::new();
        for (s, p, o) in &self.triples {
             if s == iri {
                 results.push((p.clone(), o.clone()));
             }
        }
        results
    }
    
    pub fn get_all_subjects(&self) -> Vec<String> {
        let mut subjects = self.triples.iter().map(|(s, _, _)| s.clone()).collect::<Vec<_>>();
        subjects.sort();
        subjects.dedup();
        subjects
    }
}
