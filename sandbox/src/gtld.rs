/**
 * Toolkit for representing Linked Data as Gemtext.
 */
use gemini::gemtext::Builder;

/// A statement is a representation of an RDF triple: subject, predicate, object.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Statement {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// A collection of statements. The order is assumed to be unimportant.
pub struct Document {
    pub statements: Vec<Statement>,
}

impl Statement {
    pub fn new(
        s: String,
        p: String,
        o: String,
        s_label: Option<String>,
        o_label: Option<String>,
    ) -> Statement {
        Statement {
            subject: s,
            predicate: p,
            object: o,
        }
    }
}

impl Document {
    pub fn new() -> Document {
        Document {
            statements: Vec::<Statement>::new(),
        }
    }

    pub fn add(&mut self, s: Statement) -> &Self {
        self.statements.push(s);
        self
    }

    pub fn len(&self) -> usize {
        self.statements.len()
    }

    /// Exports to a representation of the RDF in gemtext.
    pub fn to_gemtext(&mut self) -> gemini::Builder {
        let mut gemtext = Builder::new();
        for stmt in self.statements.iter() {
            let s = &stmt.subject;
            let p = &stmt.predicate;
            let o = &stmt.object;
            gemtext = gemtext
                .link(s, None::<String>)
                .link(p, None::<String>)
                .link(o, None::<String>)
                .line();
        }
        gemtext
    }
}

#[test]
fn test_gemtext_object_property_assertion() {
    use crate::namespace::{NS_FRBR, NS_RDF, NS_WIKIDATA};
    let stmt = Statement::new(
        format!("{NS_WIKIDATA}Q2026413"),
        format!("{NS_RDF}type"),
        format!("{NS_FRBR}Work"),
        None,
        None,
    );
    let mut rdf = Document::new();
    rdf.add(stmt.clone());
    assert_eq!(rdf.len(), 1);
    assert_eq!(rdf.statements[0], stmt);
}
