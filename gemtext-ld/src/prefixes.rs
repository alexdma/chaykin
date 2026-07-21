/// Well-known namespace prefixes for URI shortening/expansion.
///
/// These are assumed to be known by every conformant client and are never
/// declared in a `# Prefixes` preamble.
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

/// Additional namespace prefixes usable only in Condensed mode.
///
/// Unlike `PREFIXES`, these are not assumed to be known ahead of time: a
/// document using any of them must declare the ones it uses in a
/// `# Prefixes` preamble (RDF-in-Gemtext spec §2.4), so that a parser
/// unfamiliar with them can still expand the QNames back to full IRIs.
const CONDENSED_PREFIXES: &[(&str, &str)] = &[
    ("http://www.w3.org/2004/02/skos/core#", "skos:"),
    ("http://www.wikidata.org/entity/", "wd:"),
    ("http://www.wikidata.org/prop/", "wdp:"),
    ("http://www.wikidata.org/prop/direct/", "wdt:"),
];

/// Find the best namespace match for `uri` within a prefix table.
///
/// A candidate is only considered if the local part remaining after the
/// namespace is stripped contains no unescaped `/`: Turtle-style prefixed
/// names (`PN_LOCAL`) do not permit a raw `/` in the local part, so shortening
/// against a namespace that would leave one in place produces an invalid
/// QName. Among the remaining candidates, the longest (most specific)
/// namespace wins — this matters when one registered namespace is itself a
/// prefix of another, e.g. `.../prop/` vs. `.../prop/direct/`.
fn best_prefix_match<'a>(
    uri: &str,
    table: &'a [(&'static str, &'static str)],
) -> Option<&'a (&'static str, &'static str)> {
    table
        .iter()
        .filter(|(namespace, _)| {
            uri.strip_prefix(namespace)
                .is_some_and(|local| !local.contains('/'))
        })
        .max_by_key(|(namespace, _)| namespace.len())
}

/// Shorten a full URI using well-known prefixes.
///
/// If the URI begins with a registered namespace, that namespace is replaced
/// with its compact prefix (e.g. `http://schema.org/name` → `schema:name`).
/// Otherwise the URI is returned unchanged.
pub fn shorten_uri(uri: &str) -> String {
    match best_prefix_match(uri, PREFIXES) {
        Some((namespace, prefix)) => uri.replacen(namespace, prefix, 1),
        None => uri.to_string(),
    }
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

/// Shorten a URI for Condensed mode, additionally trying `CONDENSED_PREFIXES`
/// once the registered prefixes have been tried and found no match.
///
/// Returns the shortened form together with the `(namespace, prefix)` pair
/// that must be declared in a `# Prefixes` preamble, if a condensed-only
/// prefix was used.
pub fn shorten_uri_condensed(uri: &str) -> (String, Option<(&'static str, &'static str)>) {
    let core = shorten_uri(uri);
    if core != uri {
        return (core, None);
    }
    match best_prefix_match(uri, CONDENSED_PREFIXES) {
        Some(&(namespace, prefix)) => (uri.replacen(namespace, prefix, 1), Some((namespace, prefix))),
        None => (uri.to_string(), None),
    }
}

/// Expand a QName using a document-local prefix map (as declared in a
/// `# Prefixes` preamble), falling back to the registered prefixes.
///
/// `declared` maps a prefix (including its trailing `:`, e.g. `"wd:"`) to its
/// full namespace IRI.
pub fn expand_uri_with(uri: &str, declared: &std::collections::HashMap<String, String>) -> String {
    for (prefix, namespace) in declared {
        if let Some(local) = uri.strip_prefix(prefix.as_str()) {
            return format!("{}{}", namespace, local);
        }
    }
    expand_uri(uri)
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
    fn test_shorten_uri_condensed_prefers_core() {
        let (short, declared) = shorten_uri_condensed("http://schema.org/name");
        assert_eq!(short, "schema:name");
        assert_eq!(declared, None);
    }

    #[test]
    fn test_shorten_uri_condensed_declares_new_prefix() {
        let (short, declared) =
            shorten_uri_condensed("http://www.wikidata.org/entity/Q257469");
        assert_eq!(short, "wd:Q257469");
        assert_eq!(
            declared,
            Some(("http://www.wikidata.org/entity/", "wd:"))
        );
    }

    #[test]
    fn test_shorten_uri_condensed_prefers_most_specific_overlapping_namespace() {
        // ".../prop/" and ".../prop/direct/" both match; the longer, more
        // specific namespace must win so the local name stays slash-free.
        let (short, declared) =
            shorten_uri_condensed("http://www.wikidata.org/prop/direct/P31");
        assert_eq!(short, "wdt:P31");
        assert_eq!(
            declared,
            Some(("http://www.wikidata.org/prop/direct/", "wdt:"))
        );

        let (short, declared) = shorten_uri_condensed("http://www.wikidata.org/prop/P31");
        assert_eq!(short, "wdp:P31");
        assert_eq!(
            declared,
            Some(("http://www.wikidata.org/prop/", "wdp:"))
        );
    }

    #[test]
    fn test_shorten_uri_condensed_unknown_unchanged() {
        let uri = "http://example.org/Q257469";
        let (short, declared) = shorten_uri_condensed(uri);
        assert_eq!(short, uri);
        assert_eq!(declared, None);
    }

    #[test]
    fn test_expand_uri_with_declared_prefix() {
        let mut declared = std::collections::HashMap::new();
        declared.insert(
            "wd:".to_string(),
            "http://www.wikidata.org/entity/".to_string(),
        );
        assert_eq!(
            expand_uri_with("wd:Q257469", &declared),
            "http://www.wikidata.org/entity/Q257469"
        );
        // Falls back to the registered prefixes when not locally declared.
        assert_eq!(
            expand_uri_with("schema:name", &declared),
            "http://schema.org/name"
        );
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
