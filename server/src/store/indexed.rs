use std::collections::{BTreeMap, BTreeSet, HashMap};
use anyhow::Result;

use super::{RdfNode, TripleStore, parse_rdf_string};

/// An indexed in-memory RDF store using subject-keyed hash map with
/// predicate-sorted inner maps.
///
/// Structure: `HashMap<subject, BTreeMap<predicate, Vec<object>>>`
///
/// This gives O(1) subject lookup and returns predicates already sorted,
/// which eliminates re-grouping and re-sorting during condensed-mode
/// Gemtext serialization.
pub struct IndexedStore {
    /// Subject → (Predicate → Objects) index.
    index: HashMap<String, BTreeMap<String, Vec<RdfNode>>>,
    /// Pre-maintained sorted set of all distinct subjects.
    subjects: BTreeSet<String>,
    /// Total number of triples stored.
    count: usize,
}

impl IndexedStore {
    pub fn new() -> Self {
        IndexedStore {
            index: HashMap::new(),
            subjects: BTreeSet::new(),
            count: 0,
        }
    }
}

impl TripleStore for IndexedStore {
    fn load_from_string(&mut self, content: &str) -> Result<()> {
        let parsed = parse_rdf_string(content)?;
        for t in parsed {
            self.subjects.insert(t.subject.clone());
            self.index
                .entry(t.subject)
                .or_default()
                .entry(t.predicate)
                .or_default()
                .push(t.object);
            self.count += 1;
        }
        Ok(())
    }

    fn triple_count(&self) -> usize {
        self.count
    }

    fn get_resource_description(&self, iri: &str) -> Vec<(String, RdfNode)> {
        match self.index.get(iri) {
            Some(predicates) => {
                let mut results = Vec::new();
                for (pred, objects) in predicates {
                    for obj in objects {
                        results.push((pred.clone(), obj.clone()));
                    }
                }
                results
            }
            None => Vec::new(),
        }
    }

    fn get_all_subjects(&self) -> Vec<String> {
        self.subjects.iter().cloned().collect()
    }
}
