use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use rio_turtle::{TurtleParser, TurtleError};
use rio_api::parser::TriplesParser;
use rio_api::model::{Subject, Term};

pub struct Store {
    triples: Vec<(String, String, String)>,
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
        let mut parser = TurtleParser::new(content.as_bytes(), None);
        
        let mut triples = Vec::new();
        
        parser.parse_all(&mut |t| {
            // Convert Subject to String
            let s = match t.subject {
                Subject::NamedNode(n) => n.iri.to_string(),
                Subject::BlankNode(b) => b.id.to_string(),
                Subject::Triple(_) => "triple".to_string(),
            };
             
            // Convert Predicate to String
            let p = t.predicate.iri.to_string();

            // Convert Object to String
            let o = match t.object {
                Term::NamedNode(n) => n.iri.to_string(),
                Term::BlankNode(b) => b.id.to_string(),
                Term::Literal(l) => format!("{:?}", l), 
                Term::Triple(_) => "triple".to_string(),
            };
            
            triples.push((s, p, o));
            Ok(()) as Result<(), TurtleError>
        }).context("Failed to parse turtle")?;

        self.triples.extend(triples);
        Ok(())
    }

    pub fn triple_count(&self) -> usize {
        self.triples.len()
    }

    pub fn get_resource_description(&self, iri: &str) -> Vec<(String, String)> {
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
