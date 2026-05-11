use anyhow::Result;

use super::{RdfNode, TripleStore, parse_rdf_string};

/// A naive in-memory RDF store backed by a flat `Vec` of triples.
///
/// All queries perform a linear scan over the entire triple set.
/// This is the original store implementation, preserved for benchmarking
/// and as a reference implementation.
pub struct NaiveStore {
    triples: Vec<(String, String, RdfNode)>,
}

impl NaiveStore {
    pub fn new() -> Self {
        NaiveStore {
            triples: Vec::new(),
        }
    }
}

impl TripleStore for NaiveStore {
    fn load_from_string(&mut self, content: &str) -> Result<()> {
        let parsed = parse_rdf_string(content)?;
        for t in parsed {
            self.triples.push((t.subject, t.predicate, t.object));
        }
        Ok(())
    }

    fn triple_count(&self) -> usize {
        self.triples.len()
    }

    fn get_resource_description(&self, iri: &str) -> Vec<(String, RdfNode)> {
        let mut results = Vec::new();
        for (s, p, o) in &self.triples {
             if s == iri {
                 results.push((p.clone(), o.clone()));
             }
        }
        results
    }
    
    fn get_all_subjects(&self) -> Vec<String> {
        let mut subjects = self.triples.iter().map(|(s, _, _)| s.clone()).collect::<Vec<_>>();
        subjects.sort();
        subjects.dedup();
        subjects
    }
}
