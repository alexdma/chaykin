/// An RDF node (object position in a triple).
#[derive(Debug, Clone, PartialEq)]
pub enum RdfNode {
    /// A named node (IRI reference).
    Iri(String),
    /// A blank node with an internal identifier.
    BlankNode(String),
    /// A plain literal with no language tag or datatype.
    SimpleLiteral(String),
    /// A literal with a BCP 47 language tag.
    LanguageTaggedLiteral(String, String),
    /// A literal with an explicit datatype IRI.
    DatatypedLiteral(String, String),
}

impl std::fmt::Display for RdfNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdfNode::Iri(iri) => write!(f, "{}", iri),
            RdfNode::BlankNode(id) => write!(f, "_:{}", id),
            RdfNode::SimpleLiteral(v) => write!(f, "\"{}\"", v),
            RdfNode::LanguageTaggedLiteral(v, l) => write!(f, "\"{}\"@{}", v, l),
            RdfNode::DatatypedLiteral(v, dt) => write!(f, "\"{}\"^^<{}>", v, dt),
        }
    }
}

/// A complete RDF triple.
#[derive(Debug, Clone, PartialEq)]
pub struct RdfTriple {
    /// The subject IRI (or blank node identifier).
    pub subject: String,
    /// The predicate IRI.
    pub predicate: String,
    /// The object node.
    pub object: RdfNode,
}

impl RdfTriple {
    /// Create a new triple.
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: RdfNode) -> Self {
        RdfTriple {
            subject: subject.into(),
            predicate: predicate.into(),
            object,
        }
    }
}
