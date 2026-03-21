/// Well-known namespace prefixes for URI shortening/expansion.
const PREFIXES: &[(&str, &str)] = &[
    ("http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rdf:"),
    ("http://www.w3.org/2000/01/rdf-schema#", "rdfs:"),
    ("http://www.w3.org/2001/XMLSchema#", "xsd:"),
    ("http://purl.org/dc/elements/1.1/", "dc:"),
    ("http://purl.org/dc/terms/", "dcterms:"),
    ("http://xmlns.com/foaf/0.1/", "foaf:"),
    ("http://www.w3.org/2002/07/owl#", "owl:"),
    ("http://schema.org/", "schema:"),
];

/// Shorten a full URI using well-known prefixes.
///
/// If the URI begins with a registered namespace, that namespace is replaced
/// with its compact prefix (e.g. `http://schema.org/name` → `schema:name`).
/// Otherwise the URI is returned unchanged.
pub fn shorten_uri(uri: &str) -> String {
    for (namespace, prefix) in PREFIXES {
        if uri.starts_with(namespace) {
            return uri.replacen(namespace, prefix, 1);
        }
    }
    uri.to_string()
}

/// Expand a (possibly shortened) URI back to its full form.
///
/// If the URI starts with a known compact prefix (e.g. `schema:`), the prefix
/// is replaced with the full namespace IRI. Otherwise the URI is returned
/// unchanged.
pub fn expand_uri(uri: &str) -> String {
    for (namespace, prefix) in PREFIXES {
        if uri.starts_with(prefix) {
            return uri.replacen(prefix, namespace, 1);
        }
    }
    uri.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorten_known_prefix() {
        assert_eq!(
            shorten_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            "rdf:type"
        );
        assert_eq!(
            shorten_uri("http://schema.org/name"),
            "schema:name"
        );
        assert_eq!(
            shorten_uri("http://purl.org/dc/terms/title"),
            "dcterms:title"
        );
    }

    #[test]
    fn test_shorten_unknown_uri() {
        let uri = "http://example.org/something";
        assert_eq!(shorten_uri(uri), uri);
    }

    #[test]
    fn test_expand_known_prefix() {
        assert_eq!(
            expand_uri("rdf:type"),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
        );
        assert_eq!(
            expand_uri("schema:name"),
            "http://schema.org/name"
        );
    }

    #[test]
    fn test_expand_unknown_uri() {
        let uri = "http://example.org/something";
        assert_eq!(expand_uri(uri), uri);
    }

    #[test]
    fn test_roundtrip() {
        let original = "http://www.w3.org/2001/XMLSchema#dateTime";
        let shortened = shorten_uri(original);
        assert_eq!(shortened, "xsd:dateTime");
        assert_eq!(expand_uri(&shortened), original);
    }

    #[test]
    fn test_all_prefixes_roundtrip() {
        let uris = vec![
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            "http://www.w3.org/2000/01/rdf-schema#label",
            "http://www.w3.org/2001/XMLSchema#integer",
            "http://purl.org/dc/elements/1.1/creator",
            "http://purl.org/dc/terms/title",
            "http://xmlns.com/foaf/0.1/name",
            "http://www.w3.org/2002/07/owl#sameAs",
            "http://schema.org/datePublished",
        ];
        for uri in uris {
            let short = shorten_uri(uri);
            assert_ne!(short, uri, "Expected shortening for {}", uri);
            assert_eq!(expand_uri(&short), uri, "Roundtrip failed for {}", uri);
        }
    }
}
