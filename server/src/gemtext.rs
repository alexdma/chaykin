use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use crate::store::RdfNode;
use gemtext_ld::{shorten_uri, shorten_uri_condensed};

/// Tracks which condensed-only prefixes (see `gemtext_ld::prefixes::CONDENSED_PREFIXES`)
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

/// Render a `# Prefixes` preamble for any condensed-only prefixes used.
fn render_prefixes_preamble(used: &UsedPrefixes) -> String {
    if used.is_empty() {
        return String::new();
    }
    let mut out = String::from("# Prefixes\n");
    for (namespace, prefix) in used {
        out.push_str(&format!("* {} {}\n", namespace, prefix));
    }
    out.push('\n');
    out
}

/// Generate a Gemtext response for a resource with its properties.
///
/// Delegates RDF serialization to the `gemtext_ld` library, then appends
/// the Home link.
pub fn generate_resource_response(
    resource_iri: &str,
    properties: &[(String, RdfNode)],
    condensed: bool,
    hostname: &str,
    lang: &Option<String>,
) -> String {
    let triples: Vec<gemtext_ld::RdfTriple> = properties
        .iter()
        .map(|(pred, obj)| {
            gemtext_ld::RdfTriple::new(resource_iri, pred.clone(), obj.clone())
        })
        .collect();

    let mode = if condensed {
        gemtext_ld::SerializationMode::Condensed
    } else {
        gemtext_ld::SerializationMode::Expanded
    };

    let mut body = gemtext_ld::serialize(&triples, mode, lang);
    body.push_str(&format!("\n=> gemini://{}/ Home\n", hostname));
    body
}

/// Generate a Gemtext response for a proxied resource.
///
/// When `condensed` is true, properties are grouped together with all their
/// objects listed underneath each property heading.
pub fn generate_proxy_response(
    original_url: &str,
    properties: &[(String, RdfNode)],
    condensed: bool,
    hostname: &str,
    lang: &Option<String>,
) -> String {
    let filtered = filter_by_language(properties, lang);

    if !condensed {
        let mut body = format!("# {}\n\n", shorten_uri(original_url));
        body.push_str(&format_proxy_properties_expanded(&filtered, hostname));
        return body;
    }

    let mut used_prefixes: UsedPrefixes = Vec::new();
    let mut body = format!(
        "# {}\n\n",
        shorten_for_condensed(original_url, &mut used_prefixes)
    );
    body.push_str(&format_proxy_properties_condensed(&filtered, hostname, &mut used_prefixes));

    let mut output = render_prefixes_preamble(&used_prefixes);
    output.push_str(&body);
    output
}

/// Filter properties by preferred language.
///
/// For each predicate that has `LanguageTaggedLiteral` values, if the preferred
/// language matches one of them, only that value is kept. Non-language-tagged
/// values and non-literal nodes are always preserved.
fn filter_by_language(properties: &[(String, RdfNode)], lang: &Option<String>) -> Vec<(String, RdfNode)> {
    let preferred = match lang {
        Some(l) => l,
        None => return properties.to_vec(),
    };

    use std::collections::HashMap;

    // Group language-tagged literals by predicate
    let mut lang_tags_by_pred: HashMap<&str, Vec<&str>> = HashMap::new();
    for (predicate, object) in properties {
        if let RdfNode::LanguageTaggedLiteral(_, l) = object {
            lang_tags_by_pred.entry(predicate.as_str())
                .or_insert_with(Vec::new)
                .push(l.as_str());
        }
    }

    properties.iter().filter(|(predicate, object)| {
        match object {
            RdfNode::LanguageTaggedLiteral(_, l) => {
                let tags = lang_tags_by_pred.get(predicate.as_str());
                match tags {
                    Some(available) if available.contains(&preferred.as_str()) => {
                        // This predicate has the preferred language — keep only that one
                        l == preferred
                    },
                    _ => true, // Preferred lang not available for this predicate, keep all
                }
            },
            _ => true, // Non-language-tagged nodes always kept
        }
    }).cloned().collect()
}

/// Format proxy properties in expanded form
fn format_proxy_properties_expanded(properties: &[(String, RdfNode)], hostname: &str) -> String {
    let mut output = String::new();

    for (predicate, object) in properties {
        let short_pred = shorten_uri(predicate);
        match object {
            RdfNode::Iri(uri) => {
                if uri.starts_with("http") {
                    let encoded = utf8_percent_encode(uri, NON_ALPHANUMERIC).to_string();
                    output.push_str(&format!("=> gemini://{}/{} {} : {}\n", hostname, encoded, short_pred, shorten_uri(uri)));
                } else if uri.starts_with("gemini://") {
                    output.push_str(&format!("=> {} {} : {}\n", uri, short_pred, shorten_uri(uri)));
                } else {
                    output.push_str(&format!("* {}: {}\n", short_pred, uri));
                }
            },
            RdfNode::BlankNode(id) => {
                output.push_str(&format!("* {}: _:{}\n", short_pred, id));
            },
            RdfNode::SimpleLiteral(v) => {
                output.push_str(&format!("* {}: \"{}\"\n", short_pred, v));
            },
            RdfNode::LanguageTaggedLiteral(v, l) => {
                output.push_str(&format!("* {}: \"{}\"@{}\n", short_pred, v, l));
            },
            RdfNode::DatatypedLiteral(v, dt) => {
                if dt.starts_with("http") {
                    let encoded = utf8_percent_encode(dt, NON_ALPHANUMERIC).to_string();
                    output.push_str(&format!("=> gemini://{}/{} {} : \"{}\"^^{}\n", hostname, encoded, short_pred, v, shorten_uri(dt)));
                } else if dt.starts_with("gemini://") {
                    output.push_str(&format!("=> {} {} : \"{}\"^^{}\n", dt, short_pred, v, shorten_uri(dt)));
                } else {
                    output.push_str(&format!("* {}: \"{}\"^^{}\n", short_pred, v, shorten_uri(dt)));
                }
            }
        }
    }

    output
}

/// Format proxy properties in condensed form (grouped by predicate)
fn format_proxy_properties_condensed(
    properties: &[(String, RdfNode)],
    hostname: &str,
    used: &mut UsedPrefixes,
) -> String {
    use std::collections::HashMap;

    // Group objects by predicate
    let mut grouped: HashMap<String, Vec<RdfNode>> = HashMap::new();
    for (predicate, object) in properties {
        grouped.entry(predicate.clone())
            .or_insert_with(Vec::new)
            .push(object.clone());
    }

    let mut output = String::new();
    let mut predicates: Vec<_> = grouped.keys().collect();
    predicates.sort();

    for predicate in predicates {
        if let Some(objects) = grouped.get(predicate) {
            output.push_str(&format!("## {}\n", shorten_for_condensed(predicate, used)));
            if predicate.starts_with("http") {
                let encoded = utf8_percent_encode(predicate, NON_ALPHANUMERIC).to_string();
                output.push_str(&format!("=> gemini://{}/{} ↗ {}\n", hostname, encoded, shorten_for_condensed(predicate, used)));
            } else if predicate.starts_with("gemini://") {
                output.push_str(&format!("=> {} ↗ {}\n", predicate, shorten_for_condensed(predicate, used)));
            }

            for object in objects {
                match object {
                    RdfNode::Iri(uri) => {
                        if uri.starts_with("http") {
                            let encoded = utf8_percent_encode(uri, NON_ALPHANUMERIC).to_string();
                            output.push_str(&format!("=> gemini://{}/{} {}\n", hostname, encoded, shorten_for_condensed(uri, used)));
                        } else if uri.starts_with("gemini://") {
                            output.push_str(&format!("=> {} {}\n", uri, shorten_for_condensed(uri, used)));
                        } else {
                            output.push_str(&format!("* {}\n", uri));
                        }
                    },
                    RdfNode::BlankNode(id) => {
                        output.push_str(&format!("* _:{}\n", id));
                    },
                    RdfNode::SimpleLiteral(v) => {
                        output.push_str(&format!("* \"{}\"\n", v));
                    },
                    RdfNode::LanguageTaggedLiteral(v, l) => {
                        output.push_str(&format!("* \"{}\"@{}\n", v, l));
                    },
                    RdfNode::DatatypedLiteral(v, dt) => {
                        if dt.starts_with("http") {
                            let encoded = utf8_percent_encode(dt, NON_ALPHANUMERIC).to_string();
                            output.push_str(&format!("=> gemini://{}/{} \"{}\"^^{}\n", hostname, encoded, v, shorten_for_condensed(dt, used)));
                        } else if dt.starts_with("gemini://") {
                            output.push_str(&format!("=> {} \"{}\"^^{}\n", dt, v, shorten_for_condensed(dt, used)));
                        } else {
                            output.push_str(&format!("* \"{}\"^^{}\n", v, shorten_for_condensed(dt, used)));
                        }
                    }
                }
            }
            output.push('\n');
        }
    }

    output
}

/// Generate a "not found" Gemtext response
pub fn generate_not_found_response(resource_iri: &str) -> String {
    format!(
        "# Not Found\r\n\r\nResource not found in graph:\n=> {}\n",
        shorten_uri(resource_iri)
    )
}

/// Generate a debug response showing available subjects
pub fn generate_debug_response(requested_iri: &str, triple_count: usize, subjects: Vec<String>) -> String {
    let mut msg = format!(
        "# No Data Found for {}\r\n\r\nLoaded {} triples.\n\n## Available Subjects:\n",
        shorten_uri(requested_iri), triple_count
    );

    for subject in subjects {
        msg.push_str(&format!("* {}\n", shorten_uri(&subject)));
    }

    msg
}

/// Generate an error response in Gemtext format
pub fn generate_error_response(title: &str, message: &str) -> String {
    format!("# {}\r\n\r\n{}\n", title, message)
}

/// Format a complete Gemini response with status code and body
pub fn format_gemini_response(body: &str) -> String {
    format!("20 text/gemini\r\n{}", body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_response_condensed_declares_wikidata_prefixes() {
        let properties = vec![(
            "http://www.wikidata.org/prop/direct/P31".to_string(),
            RdfNode::Iri("http://www.wikidata.org/entity/Q7889".to_string()),
        )];

        let body = generate_proxy_response(
            "http://www.wikidata.org/entity/Q257469",
            &properties,
            true,
            "localhost",
            &None,
        );

        assert!(body.starts_with("# Prefixes\n"));
        assert!(body.contains("* http://www.wikidata.org/entity/ wd:\n"));
        assert!(body.contains("* http://www.wikidata.org/prop/direct/ wdt:\n"));
        assert!(body.contains("# wd:Q257469\n"));
        assert!(body.contains("## wdt:P31\n"));
        assert!(body.contains("wd:Q7889"));
        // No lingering slash from a wrong `wdp:` match.
        assert!(!body.contains("wdp:direct"));
    }

    #[test]
    fn test_proxy_response_condensed_no_preamble_without_extended_prefixes() {
        let properties = vec![(
            "http://schema.org/name".to_string(),
            RdfNode::SimpleLiteral("Alice".to_string()),
        )];

        let body = generate_proxy_response(
            "http://example.org/Alice",
            &properties,
            true,
            "localhost",
            &None,
        );

        assert!(!body.contains("# Prefixes"));
        assert!(body.starts_with("# http://example.org/Alice\n\n"));
    }
}
