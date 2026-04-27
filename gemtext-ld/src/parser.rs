use crate::model::{RdfNode, RdfTriple};
use crate::prefixes::expand_uri;

/// Unescape a literal value extracted from a Gemtext line.
///
/// Reverses the escaping applied by the serializer: `\\` → `\`, `\n` → newline,
/// `\r` → carriage return. Unknown escape sequences are passed through unchanged.
fn unescape_literal(v: &str) -> String {
    let mut result = String::with_capacity(v.len());
    let mut chars = v.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('n')  => result.push('\n'),
                Some('r')  => result.push('\r'),
                Some(other) => { result.push('\\'); result.push(other); }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse a Gemtext document back into RDF triples.
///
/// The parser auto-detects expanded vs condensed mode:
/// - **Condensed**: contains `## ` predicate headings
/// - **Expanded**: no `## ` headings; predicate is embedded in each line
///
/// Subject is determined from `# Resource: <iri>` headings. All subsequent
/// triples belong to that subject until the next subject heading.
pub fn parse(input: &str) -> Vec<RdfTriple> {
    let lines: Vec<&str> = input.lines().collect();

    // Auto-detect mode: if any line starts with "## ", it's condensed
    let is_condensed = lines.iter().any(|l| l.starts_with("## "));

    if is_condensed {
        parse_condensed(&lines)
    } else {
        parse_expanded(&lines)
    }
}

/// Parse expanded-mode Gemtext into RDF triples.
fn parse_expanded(lines: &[&str]) -> Vec<RdfTriple> {
    let mut triples = Vec::new();
    let mut current_subject: Option<String> = None;

    for line in lines {
        // Subject heading
        if let Some(rest) = line.strip_prefix("# Resource: ") {
            current_subject = Some(expand_uri(rest.trim()));
            continue;
        }

        let subject = match &current_subject {
            Some(s) => s.clone(),
            None => continue,
        };

        // Link line: => <target> <predicate> : <value>
        if let Some(rest) = line.strip_prefix("=> ") {
            if let Some(triple) = parse_expanded_link_line(rest, &subject) {
                triples.push(triple);
            }
            continue;
        }

        // Bullet line: * <predicate>: <value>
        if let Some(rest) = line.strip_prefix("* ") {
            if let Some(triple) = parse_expanded_bullet_line(rest, &subject) {
                triples.push(triple);
            }
        }
    }

    triples
}

/// Parse a `=> <target> <predicate> : <value>` link line in expanded mode.
fn parse_expanded_link_line(rest: &str, subject: &str) -> Option<RdfTriple> {
    // Format: <target_url> <predicate> : <value>
    // The target URL ends at the first space
    let (target, after_target) = split_first_space(rest)?;
    let after_target = after_target.trim();

    // Find " : " separator between predicate and value
    let colon_pos = after_target.find(" : ")?;
    let predicate = expand_uri(after_target[..colon_pos].trim());
    let value_str = after_target[colon_pos + 3..].trim();

    // Determine the object type from the value string
    let object = parse_object_value(value_str, Some(target));
    Some(RdfTriple::new(subject, predicate, object))
}

/// Parse a `* <predicate>: <value>` bullet line in expanded mode.
fn parse_expanded_bullet_line(rest: &str, subject: &str) -> Option<RdfTriple> {
    // Format: <predicate>: <value>
    let colon_pos = rest.find(": ")?;
    let predicate = expand_uri(rest[..colon_pos].trim());
    let value_str = rest[colon_pos + 2..].trim();

    let object = parse_object_value(value_str, None);
    Some(RdfTriple::new(subject, predicate, object))
}

/// Parse condensed-mode Gemtext into RDF triples.
fn parse_condensed(lines: &[&str]) -> Vec<RdfTriple> {
    let mut triples = Vec::new();
    let mut current_subject: Option<String> = None;
    let mut current_predicate: Option<String> = None;

    for line in lines {
        // Subject heading
        if let Some(rest) = line.strip_prefix("# Resource: ") {
            current_subject = Some(expand_uri(rest.trim()));
            current_predicate = None;
            continue;
        }

        // Predicate heading
        if let Some(rest) = line.strip_prefix("## ") {
            current_predicate = Some(expand_uri(rest.trim()));
            continue;
        }

        let subject = match &current_subject {
            Some(s) => s.clone(),
            None => continue,
        };
        let predicate = match &current_predicate {
            Some(p) => p.clone(),
            None => continue,
        };

        // Property link: => <target> ↗ <short_predicate>  (skip these, they're navigational)
        if let Some(rest) = line.strip_prefix("=> ") {
            if rest.contains(" ↗ ") {
                continue;
            }
            // Object link: => <target> <display_text>
            if let Some(triple) = parse_condensed_link_line(rest, &subject, &predicate) {
                triples.push(triple);
            }
            continue;
        }

        // Bullet line: * <value>
        if let Some(rest) = line.strip_prefix("* ") {
            let object = parse_object_value(rest.trim(), None);
            triples.push(RdfTriple::new(&subject, &predicate, object));
        }
    }

    triples
}

/// Parse a link line in condensed mode: `=> <target> <display_text>`.
fn parse_condensed_link_line(
    rest: &str,
    subject: &str,
    predicate: &str,
) -> Option<RdfTriple> {
    let (target, display) = split_first_space(rest)?;
    let display = display.trim();

    // Check if this is a datatyped literal: display looks like "value"^^type
    let object = parse_object_value(display, Some(target));
    Some(RdfTriple::new(subject, predicate, object))
}

/// Parse an object value string into an `RdfNode`.
///
/// `link_target` is provided when the value came from a `=> ` link line,
/// allowing us to use the actual URL rather than the display text for IRIs.
fn parse_object_value(value: &str, link_target: Option<&str>) -> RdfNode {
    // Datatyped literal: "value"^^type
    if let Some(dt_match) = parse_datatyped_literal(value) {
        return dt_match;
    }

    // Language-tagged literal: "value"@lang
    if let Some(lang_match) = parse_language_tagged_literal(value) {
        return lang_match;
    }

    // Simple literal: "value"
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        return RdfNode::SimpleLiteral(unescape_literal(&value[1..value.len() - 1]));
    }

    // Blank node: _:id
    if let Some(id) = value.strip_prefix("_:") {
        return RdfNode::BlankNode(id.to_string());
    }

    // IRI — prefer the link target (full URL) over the display text (may be shortened)
    if let Some(target) = link_target {
        RdfNode::Iri(target.to_string())
    } else {
        // The value might be a shortened URI in a bullet
        RdfNode::Iri(expand_uri(value))
    }
}

/// Try to parse `"value"^^type` into a DatatypedLiteral.
fn parse_datatyped_literal(value: &str) -> Option<RdfNode> {
    if !value.starts_with('"') {
        return None;
    }
    // Find closing quote followed by ^^
    let close_quote = find_closing_quote(value)?;
    let after_quote = &value[close_quote + 1..];
    if !after_quote.starts_with("^^") {
        return None;
    }
    let lexical = unescape_literal(&value[1..close_quote]);
    let datatype = expand_uri(after_quote[2..].trim());
    Some(RdfNode::DatatypedLiteral(lexical, datatype))
}

/// Try to parse `"value"@lang` into a LanguageTaggedLiteral.
fn parse_language_tagged_literal(value: &str) -> Option<RdfNode> {
    if !value.starts_with('"') {
        return None;
    }
    let close_quote = find_closing_quote(value)?;
    let after_quote = &value[close_quote + 1..];
    if !after_quote.starts_with('@') {
        return None;
    }
    let lexical = unescape_literal(&value[1..close_quote]);
    let lang = after_quote[1..].trim().to_string();
    if lang.is_empty() {
        return None;
    }
    Some(RdfNode::LanguageTaggedLiteral(lexical, lang))
}

/// Find the position of the closing `"` in a quoted string (starting at index 1).
fn find_closing_quote(s: &str) -> Option<usize> {
    if !s.starts_with('"') || s.len() < 2 {
        return None;
    }
    // Find the last `"` before any `^^` or `@` suffix
    // Simple approach: scan from the end inward for `"`
    let bytes = s.as_bytes();
    for i in (1..bytes.len()).rev() {
        if bytes[i] == b'"' {
            return Some(i);
        }
    }
    None
}

/// Split a string at the first space, returning (before, after).
fn split_first_space(s: &str) -> Option<(&str, &str)> {
    let pos = s.find(' ')?;
    Some((&s[..pos], &s[pos + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RdfNode, RdfTriple};
    use crate::serializer::{SerializationMode, serialize};

    #[test]
    fn test_parse_expanded_iri_link() {
        let input = "# Resource: http://example.org/x\n\n\
                      => http://dbpedia.org/resource/Q1 owl:sameAs : http://dbpedia.org/resource/Q1\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].subject, "http://example.org/x");
        assert_eq!(
            triples[0].predicate,
            "http://www.w3.org/2002/07/owl#sameAs"
        );
        assert_eq!(
            triples[0].object,
            RdfNode::Iri("http://dbpedia.org/resource/Q1".into())
        );
    }

    #[test]
    fn test_parse_expanded_simple_literal() {
        let input = "# Resource: http://example.org/x\n\n\
                      * dcterms:identifier: \"71181\"\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].predicate,
            "http://purl.org/dc/terms/identifier"
        );
        assert_eq!(
            triples[0].object,
            RdfNode::SimpleLiteral("71181".into())
        );
    }

    #[test]
    fn test_parse_expanded_language_tagged() {
        let input = "# Resource: http://example.org/x\n\n\
                      * dcterms:title: \"videogioco del 1991\"@it\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::LanguageTaggedLiteral("videogioco del 1991".into(), "it".into())
        );
    }

    #[test]
    fn test_parse_expanded_datatyped_literal() {
        let input = "# Resource: http://example.org/x\n\n\
                      => http://www.w3.org/2001/XMLSchema#dateTime schema:datePublished : \"1991-01-01\"^^xsd:dateTime\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].predicate,
            "http://schema.org/datePublished"
        );
        assert_eq!(
            triples[0].object,
            RdfNode::DatatypedLiteral(
                "1991-01-01".into(),
                "http://www.w3.org/2001/XMLSchema#dateTime".into()
            )
        );
    }

    #[test]
    fn test_parse_expanded_blank_node() {
        let input = "# Resource: http://example.org/x\n\n\
                      * foaf:knows: _:b0\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::BlankNode("b0".into())
        );
    }

    #[test]
    fn test_parse_expanded_non_http_iri() {
        let input = "# Resource: http://example.org/x\n\n\
                      * foaf:mbox: mailto:alice@example.org\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::Iri("mailto:alice@example.org".into())
        );
    }

    #[test]
    fn test_parse_condensed_basic() {
        let input = "# Resource: http://example.org/x\n\n\
                      ## dcterms:title\n\
                      => http://purl.org/dc/terms/title ↗ dcterms:title\n\
                      * \"Hello World\"\n\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].predicate,
            "http://purl.org/dc/terms/title"
        );
        assert_eq!(
            triples[0].object,
            RdfNode::SimpleLiteral("Hello World".into())
        );
    }

    #[test]
    fn test_parse_condensed_iri_link() {
        let input = "# Resource: http://example.org/x\n\n\
                      ## owl:sameAs\n\
                      => http://www.w3.org/2002/07/owl#sameAs ↗ owl:sameAs\n\
                      => http://dbpedia.org/resource/Q1 http://dbpedia.org/resource/Q1\n\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::Iri("http://dbpedia.org/resource/Q1".into())
        );
    }

    #[test]
    fn test_parse_condensed_datatyped() {
        let input = "# Resource: http://example.org/x\n\n\
                      ## schema:datePublished\n\
                      => http://schema.org/datePublished ↗ schema:datePublished\n\
                      => http://www.w3.org/2001/XMLSchema#dateTime \"1991-01-01\"^^xsd:dateTime\n\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::DatatypedLiteral(
                "1991-01-01".into(),
                "http://www.w3.org/2001/XMLSchema#dateTime".into()
            )
        );
    }

    #[test]
    fn test_parse_condensed_language_tagged() {
        let input = "# Resource: http://example.org/x\n\n\
                      ## dcterms:title\n\
                      => http://purl.org/dc/terms/title ↗ dcterms:title\n\
                      * \"videogioco del 1991\"@it\n\
                      * \"1991 video game\"@en\n\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 2);
        assert_eq!(
            triples[0].object,
            RdfNode::LanguageTaggedLiteral("videogioco del 1991".into(), "it".into())
        );
        assert_eq!(
            triples[1].object,
            RdfNode::LanguageTaggedLiteral("1991 video game".into(), "en".into())
        );
    }

    #[test]
    fn test_parse_multi_subject() {
        let input = "# Resource: http://example.org/Alice\n\n\
                      * foaf:name: \"Alice\"\n\
                      # Resource: http://example.org/Bob\n\n\
                      * foaf:name: \"Bob\"\n";
        let triples = parse(input);
        assert_eq!(triples.len(), 2);
        assert_eq!(triples[0].subject, "http://example.org/Alice");
        assert_eq!(triples[1].subject, "http://example.org/Bob");
    }

    #[test]
    fn test_parse_empty_input() {
        let triples = parse("");
        assert!(triples.is_empty());
    }

    #[test]
    fn test_roundtrip_expanded() {
        let original = vec![
            RdfTriple::new(
                "http://example.org/x",
                "http://purl.org/dc/terms/title",
                RdfNode::SimpleLiteral("Hello".into()),
            ),
            RdfTriple::new(
                "http://example.org/x",
                "http://www.w3.org/2002/07/owl#sameAs",
                RdfNode::Iri("http://dbpedia.org/resource/Q1".into()),
            ),
            RdfTriple::new(
                "http://example.org/x",
                "http://xmlns.com/foaf/0.1/name",
                RdfNode::LanguageTaggedLiteral("Ciao".into(), "it".into()),
            ),
        ];

        let gemtext = serialize(&original, SerializationMode::Expanded, &None);
        let parsed = parse(&gemtext);

        assert_eq!(parsed.len(), original.len());
        for (orig, round) in original.iter().zip(parsed.iter()) {
            assert_eq!(orig.subject, round.subject);
            assert_eq!(orig.predicate, round.predicate);
            assert_eq!(orig.object, round.object);
        }
    }

    #[test]
    fn test_roundtrip_condensed() {
        let original = vec![
            RdfTriple::new(
                "http://example.org/x",
                "http://purl.org/dc/terms/title",
                RdfNode::SimpleLiteral("Hello".into()),
            ),
            RdfTriple::new(
                "http://example.org/x",
                "http://purl.org/dc/terms/title",
                RdfNode::LanguageTaggedLiteral("Bonjour".into(), "fr".into()),
            ),
            RdfTriple::new(
                "http://example.org/x",
                "http://www.w3.org/2002/07/owl#sameAs",
                RdfNode::Iri("http://dbpedia.org/resource/Q1".into()),
            ),
        ];

        let gemtext = serialize(&original, SerializationMode::Condensed, &None);
        let parsed = parse(&gemtext);

        // In condensed mode, predicates are sorted, so order may differ
        assert_eq!(parsed.len(), original.len());
        for orig in &original {
            let found = parsed.iter().any(|p| {
                p.subject == orig.subject
                    && p.predicate == orig.predicate
                    && p.object == orig.object
            });
            assert!(found, "Missing triple: {:?}", orig);
        }
    }

    #[test]
    fn test_roundtrip_multi_subject_expanded() {
        let original = vec![
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
        ];

        let gemtext = serialize(&original, SerializationMode::Expanded, &None);
        let parsed = parse(&gemtext);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], original[0]);
        assert_eq!(parsed[1], original[1]);
    }

    #[test]
    fn test_roundtrip_blank_node() {
        let original = vec![RdfTriple::new(
            "http://example.org/x",
            "http://xmlns.com/foaf/0.1/knows",
            RdfNode::BlankNode("b42".into()),
        )];

        let gemtext = serialize(&original, SerializationMode::Expanded, &None);
        let parsed = parse(&gemtext);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], original[0]);
    }

    #[test]
    fn test_roundtrip_datatyped_literal() {
        let original = vec![RdfTriple::new(
            "http://example.org/x",
            "http://schema.org/datePublished",
            RdfNode::DatatypedLiteral(
                "1991-01-01T00:00:00Z".into(),
                "http://www.w3.org/2001/XMLSchema#dateTime".into(),
            ),
        )];

        let gemtext = serialize(&original, SerializationMode::Expanded, &None);
        let parsed = parse(&gemtext);

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], original[0]);
    }

    #[test]
    fn test_ignores_non_resource_headings() {
        let input = "# Not Found\n\nResource not found in graph:\n=> http://example.org/x\n";
        let triples = parse(input);
        assert!(triples.is_empty());
    }

    #[test]
    fn test_ignores_home_link() {
        let input = "# Resource: http://example.org/x\n\n\
                      * foaf:name: \"Alice\"\n\
                      \n=> gemini://localhost/ Home\n";
        let triples = parse(input);
        // The "Home" link has no " : " separator, so it won't parse as expanded
        assert_eq!(triples.len(), 1);
        assert_eq!(
            triples[0].object,
            RdfNode::SimpleLiteral("Alice".into())
        );
    }
}
