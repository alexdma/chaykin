use std::collections::HashMap;
use crate::model::{RdfNode, RdfTriple};
use crate::prefixes::{shorten_uri, shorten_uri_condensed};

/// Tracks which condensed-only prefixes (see `prefixes::CONDENSED_PREFIXES`)
/// have been used so far in a document, in first-use order, so they can be
/// declared in a `# Prefixes` preamble.
type UsedPrefixes = Vec<(&'static str, &'static str)>;

/// Shorten a URI for Condensed mode, recording any condensed-only prefix it
/// required so the caller can later render a `# Prefixes` preamble.
fn shorten_for_condensed(uri: &str, used: &mut UsedPrefixes) -> String {
    let (short, declared) = shorten_uri_condensed(uri);
    if let Some(pair) = declared {
        if !used.contains(&pair) {
            used.push(pair);
        }
    }
    short
}

/// Escape a literal value for safe embedding in a single Gemtext line.
///
/// The line-oriented format cannot represent raw newlines or carriage returns
/// inside a quoted value. Backslashes are escaped first to avoid ambiguity.
fn escape_literal(v: &str) -> String {
    v.replace('\\', "\\\\")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
}

/// Serialization mode for RDF-to-Gemtext output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SerializationMode {
    /// One Gemtext line per triple, encoding both predicate and object.
    Expanded,
    /// Triples grouped by predicate under level-2 headings.
    Condensed,
}

/// Serialize a set of RDF triples into Gemtext format.
///
/// Triples are grouped by subject. Each subject gets a level-1 heading
/// (`# Resource: <subject>`), followed by its properties in either expanded
/// or condensed mode.
///
/// If `lang` is `Some`, language-tagged literals are filtered to prefer that
/// language (per-predicate).
pub fn serialize(
    triples: &[RdfTriple],
    mode: SerializationMode,
    lang: &Option<String>,
) -> String {
    // Group triples by subject, preserving insertion order
    let mut subjects_order: Vec<String> = Vec::new();
    let mut by_subject: HashMap<String, Vec<(String, RdfNode)>> = HashMap::new();

    for triple in triples {
        if !by_subject.contains_key(&triple.subject) {
            subjects_order.push(triple.subject.clone());
        }
        by_subject
            .entry(triple.subject.clone())
            .or_default()
            .push((triple.predicate.clone(), triple.object.clone()));
    }

    let mut used_prefixes: UsedPrefixes = Vec::new();
    let mut body = String::new();

    for subject in &subjects_order {
        let properties = by_subject.get(subject).unwrap();
        let filtered = filter_by_language(properties, lang);

        let subject_short = match mode {
            SerializationMode::Expanded => shorten_uri(subject),
            SerializationMode::Condensed => shorten_for_condensed(subject, &mut used_prefixes),
        };
        body.push_str(&format!("# Resource: {}\n\n", subject_short));

        match mode {
            SerializationMode::Expanded => {
                body.push_str(&format_properties_expanded(&filtered));
            }
            SerializationMode::Condensed => {
                body.push_str(&format_properties_condensed(&filtered, &mut used_prefixes));
            }
        }
    }

    let mut output = String::new();
    if !used_prefixes.is_empty() {
        output.push_str("# Prefixes\n");
        for (namespace, prefix) in &used_prefixes {
            output.push_str(&format!("* {} {}\n", namespace, prefix));
        }
        output.push('\n');
    }
    output.push_str(&body);
    output
}

/// Filter properties by preferred language.
///
/// For each predicate that has `LanguageTaggedLiteral` values, if the preferred
/// language matches one of them, only that value is kept. Non-language-tagged
/// values and non-literal nodes are always preserved.
fn filter_by_language(
    properties: &[(String, RdfNode)],
    lang: &Option<String>,
) -> Vec<(String, RdfNode)> {
    let preferred = match lang {
        Some(l) => l,
        None => return properties.to_vec(),
    };

    // Group language tags by predicate
    let mut lang_tags_by_pred: HashMap<&str, Vec<&str>> = HashMap::new();
    for (predicate, object) in properties {
        if let RdfNode::LanguageTaggedLiteral(_, l) = object {
            lang_tags_by_pred
                .entry(predicate.as_str())
                .or_default()
                .push(l.as_str());
        }
    }

    properties
        .iter()
        .filter(|(predicate, object)| match object {
            RdfNode::LanguageTaggedLiteral(_, l) => {
                let tags = lang_tags_by_pred.get(predicate.as_str());
                match tags {
                    Some(available) if available.contains(&preferred.as_str()) => l == preferred,
                    _ => true,
                }
            }
            _ => true,
        })
        .cloned()
        .collect()
}

/// Format properties in expanded form (one line per property-object pair).
fn format_properties_expanded(properties: &[(String, RdfNode)]) -> String {
    let mut output = String::new();

    for (predicate, object) in properties {
        let short_pred = shorten_uri(predicate);
        match object {
            RdfNode::Iri(uri) => {
                if uri.starts_with("gemini://") || uri.starts_with("http") {
                    output.push_str(&format!("=> {} {} : {}\n", uri, short_pred, shorten_uri(uri)));
                } else {
                    output.push_str(&format!("* {}: {}\n", short_pred, uri));
                }
            }
            RdfNode::BlankNode(id) => {
                output.push_str(&format!("* {}: _:{}\n", short_pred, id));
            }
            RdfNode::SimpleLiteral(v) => {
                output.push_str(&format!("* {}: \"{}\"\n", short_pred, escape_literal(v)));
            }
            RdfNode::LanguageTaggedLiteral(v, l) => {
                output.push_str(&format!("* {}: \"{}\"@{}\n", short_pred, escape_literal(v), l));
            }
            RdfNode::DatatypedLiteral(v, dt) => {
                if dt.starts_with("gemini://") || dt.starts_with("http") {
                    output.push_str(&format!(
                        "=> {} {} : \"{}\"^^{}\n",
                        dt,
                        short_pred,
                        escape_literal(v),
                        shorten_uri(dt)
                    ));
                } else {
                    output.push_str(&format!(
                        "* {}: \"{}\"^^{}\n",
                        short_pred,
                        escape_literal(v),
                        shorten_uri(dt)
                    ));
                }
            }
        }
    }

    output
}

/// Format properties in condensed form (grouped by predicate).
fn format_properties_condensed(properties: &[(String, RdfNode)], used: &mut UsedPrefixes) -> String {
    // Group objects by predicate, preserving insertion order
    let mut pred_order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, Vec<RdfNode>> = HashMap::new();
    for (predicate, object) in properties {
        if !grouped.contains_key(predicate) {
            pred_order.push(predicate.clone());
        }
        grouped
            .entry(predicate.clone())
            .or_default()
            .push(object.clone());
    }

    pred_order.sort(); // Keep consistent ordering

    let mut output = String::new();

    for predicate in &pred_order {
        if let Some(objects) = grouped.get(predicate) {
            output.push_str(&format!("## {}\n", shorten_for_condensed(predicate, used)));
            if predicate.starts_with("gemini://") || predicate.starts_with("http") {
                output.push_str(&format!(
                    "=> {} ↗ {}\n",
                    predicate,
                    shorten_for_condensed(predicate, used)
                ));
            }

            for object in objects {
                match object {
                    RdfNode::Iri(uri) => {
                        if uri.starts_with("gemini://") || uri.starts_with("http") {
                            output
                                .push_str(&format!("=> {} {}\n", uri, shorten_for_condensed(uri, used)));
                        } else {
                            output.push_str(&format!("* {}\n", uri));
                        }
                    }
                    RdfNode::BlankNode(id) => {
                        output.push_str(&format!("* _:{}\n", id));
                    }
                    RdfNode::SimpleLiteral(v) => {
                        output.push_str(&format!("* \"{}\"\n", escape_literal(v)));
                    }
                    RdfNode::LanguageTaggedLiteral(v, l) => {
                        output.push_str(&format!("* \"{}\"@{}\n", escape_literal(v), l));
                    }
                    RdfNode::DatatypedLiteral(v, dt) => {
                        if dt.starts_with("gemini://") || dt.starts_with("http") {
                            output.push_str(&format!(
                                "=> {} \"{}\"^^{}\n",
                                dt,
                                escape_literal(v),
                                shorten_for_condensed(dt, used)
                            ));
                        } else {
                            output.push_str(&format!(
                                "* \"{}\"^^{}\n",
                                escape_literal(v),
                                shorten_for_condensed(dt, used)
                            ));
                        }
                    }
                }
            }
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RdfNode, RdfTriple};

    fn sample_triples() -> Vec<RdfTriple> {
        vec![
            RdfTriple::new(
                "http://example.org/Q257469",
                "http://purl.org/dc/terms/title",
                RdfNode::LanguageTaggedLiteral("videogioco del 1991".into(), "it".into()),
            ),
            RdfTriple::new(
                "http://example.org/Q257469",
                "http://purl.org/dc/terms/title",
                RdfNode::LanguageTaggedLiteral("1991 video game".into(), "en".into()),
            ),
            RdfTriple::new(
                "http://example.org/Q257469",
                "http://purl.org/dc/terms/identifier",
                RdfNode::SimpleLiteral("71181".into()),
            ),
            RdfTriple::new(
                "http://example.org/Q257469",
                "http://www.w3.org/2002/07/owl#sameAs",
                RdfNode::Iri("http://dbpedia.org/resource/Q257469".into()),
            ),
            RdfTriple::new(
                "http://example.org/Q257469",
                "http://schema.org/datePublished",
                RdfNode::DatatypedLiteral(
                    "1991-01-01T00:00:00Z".into(),
                    "http://www.w3.org/2001/XMLSchema#dateTime".into(),
                ),
            ),
        ]
    }

    #[test]
    fn test_expanded_single_subject() {
        let triples = sample_triples();
        let output = serialize(&triples, SerializationMode::Expanded, &None);

        assert!(output.starts_with("# Resource: http://example.org/Q257469\n\n"));
        assert!(output.contains("* dcterms:title: \"videogioco del 1991\"@it\n"));
        assert!(output.contains("* dcterms:title: \"1991 video game\"@en\n"));
        assert!(output.contains("* dcterms:identifier: \"71181\"\n"));
        assert!(output.contains(
            "=> http://dbpedia.org/resource/Q257469 owl:sameAs : http://dbpedia.org/resource/Q257469\n"
        ));
        assert!(output.contains(
            "=> http://www.w3.org/2001/XMLSchema#dateTime schema:datePublished : \"1991-01-01T00:00:00Z\"^^xsd:dateTime\n"
        ));
    }

    #[test]
    fn test_condensed_single_subject() {
        let triples = sample_triples();
        let output = serialize(&triples, SerializationMode::Condensed, &None);

        assert!(output.starts_with("# Resource: http://example.org/Q257469\n\n"));
        assert!(output.contains("## dcterms:identifier\n"));
        assert!(output.contains("## dcterms:title\n"));
        assert!(output.contains("## owl:sameAs\n"));
        assert!(output.contains("## schema:datePublished\n"));
        // Property links
        assert!(output.contains("=> http://purl.org/dc/terms/identifier ↗ dcterms:identifier\n"));
        assert!(output.contains("=> http://purl.org/dc/terms/title ↗ dcterms:title\n"));
    }

    #[test]
    fn test_multi_subject() {
        let triples = vec![
            RdfTriple::new(
                "http://example.org/Alice",
                "http://xmlns.com/foaf/0.1/name",
                RdfNode::SimpleLiteral("Alice".into()),
            ),
            RdfTriple::new(
                "http://example.org/Bob",
                "http://xmlns.com/foaf/0.1/name",
                RdfNode::SimpleLiteral("Bob".into()),
            ),
            RdfTriple::new(
                "http://example.org/Alice",
                "http://xmlns.com/foaf/0.1/knows",
                RdfNode::Iri("http://example.org/Bob".into()),
            ),
        ];

        let output = serialize(&triples, SerializationMode::Expanded, &None);
        // Should have two subject headings
        assert!(output.contains("# Resource: http://example.org/Alice\n"));
        assert!(output.contains("# Resource: http://example.org/Bob\n"));

        // Alice section should come first (insertion order)
        let alice_pos = output.find("# Resource: http://example.org/Alice").unwrap();
        let bob_pos = output.find("# Resource: http://example.org/Bob").unwrap();
        assert!(alice_pos < bob_pos);
    }

    #[test]
    fn test_language_filtering() {
        let triples = sample_triples();
        let output =
            serialize(&triples, SerializationMode::Expanded, &Some("en".into()));

        assert!(output.contains("\"1991 video game\"@en"));
        assert!(!output.contains("\"videogioco del 1991\"@it"));
    }

    #[test]
    fn test_blank_node() {
        let triples = vec![RdfTriple::new(
            "http://example.org/x",
            "http://xmlns.com/foaf/0.1/knows",
            RdfNode::BlankNode("b0".into()),
        )];

        let output = serialize(&triples, SerializationMode::Expanded, &None);
        assert!(output.contains("* foaf:knows: _:b0\n"));
    }

    #[test]
    fn test_non_http_iri_as_bullet() {
        let triples = vec![RdfTriple::new(
            "http://example.org/x",
            "http://xmlns.com/foaf/0.1/mbox",
            RdfNode::Iri("mailto:alice@example.org".into()),
        )];

        let output = serialize(&triples, SerializationMode::Expanded, &None);
        assert!(output.contains("* foaf:mbox: mailto:alice@example.org\n"));
    }

    #[test]
    fn test_gemini_iri_as_link() {
        let triples = vec![RdfTriple::new(
            "http://example.org/x",
            "http://www.w3.org/2000/01/rdf-schema#seeAlso",
            RdfNode::Iri("gemini://example.org/info".into()),
        )];

        let output = serialize(&triples, SerializationMode::Expanded, &None);
        assert!(output.contains("=> gemini://example.org/info rdfs:seeAlso : gemini://example.org/info\n"));
    }

    #[test]
    fn test_empty_triples() {
        let output = serialize(&[], SerializationMode::Expanded, &None);
        assert!(output.is_empty());
    }

    #[test]
    fn test_condensed_trailing_blank_lines() {
        let triples = vec![
            RdfTriple::new(
                "http://example.org/x",
                "http://xmlns.com/foaf/0.1/name",
                RdfNode::SimpleLiteral("Alice".into()),
            ),
            RdfTriple::new(
                "http://example.org/x",
                "http://xmlns.com/foaf/0.1/age",
                RdfNode::DatatypedLiteral("30".into(), "http://www.w3.org/2001/XMLSchema#integer".into()),
            ),
        ];

        let output = serialize(&triples, SerializationMode::Condensed, &None);
        // Each predicate group should end with a blank line
        let groups: Vec<&str> = output.split("## ").skip(1).collect();
        for group in &groups {
            assert!(group.ends_with("\n\n") || output.ends_with(group.trim_end()));
        }
    }
}
