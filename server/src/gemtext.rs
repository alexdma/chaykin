use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use crate::store::RdfNode;

/// Shorten well-known URIs using common prefixes
pub fn shorten_uri(uri: &str) -> String {
    let prefixes = [
        ("http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rdf:"),
        ("http://www.w3.org/2000/01/rdf-schema#", "rdfs:"),
        ("http://www.w3.org/2001/XMLSchema#", "xsd:"),
        ("http://purl.org/dc/elements/1.1/", "dc:"),
        ("http://purl.org/dc/terms/", "dcterms:"),
        ("http://xmlns.com/foaf/0.1/", "foaf:"),
        ("http://www.w3.org/2002/07/owl#", "owl:"),
        ("http://schema.org/", "schema:"),
    ];
    for (prefix, replacement) in &prefixes {
        if uri.starts_with(prefix) {
            return uri.replacen(prefix, replacement, 1);
        }
    }
    uri.to_string()
}

/// Generate a Gemtext response for a resource with its properties
/// 
/// When `condensed` is true, properties are grouped together with all their objects
/// listed underneath each property heading.
pub fn generate_resource_response(
    resource_iri: &str, 
    properties: &[(String, RdfNode)],
    condensed: bool,
    hostname: &str
) -> String {
    let mut body = format!("# Resource: {}\n\n", shorten_uri(resource_iri));
    
    let formatted = if condensed {
        format_properties_condensed(properties)
    } else {
        format_properties_expanded(properties)
    };
    body.push_str(&formatted);
    
    body.push_str(&format!("\n=> gemini://{}/ Home\n", hostname));
    body
}

/// Format properties in expanded form (one line per property-object pair)
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
                if dt.starts_with("gemini://") || dt.starts_with("http") {
                    output.push_str(&format!("=> {} {} : \"{}\"^^{}\n", dt, short_pred, v, shorten_uri(dt)));
                } else {
                    output.push_str(&format!("* {}: \"{}\"^^{}\n", short_pred, v, shorten_uri(dt)));
                }
            }
        }
    }
    
    output
}

/// Format properties in condensed form (grouped by predicate)
fn format_properties_condensed(properties: &[(String, RdfNode)]) -> String {
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
    predicates.sort(); // Keep consistent ordering
    
    for predicate in predicates {
        if let Some(objects) = grouped.get(predicate) {
            output.push_str(&format!("## {}\n", shorten_uri(predicate)));
            if predicate.starts_with("gemini://") || predicate.starts_with("http") {
                output.push_str(&format!("=> {} ↗ {}\n", predicate, shorten_uri(predicate)));
            }
            
            for object in objects {
                match object {
                    RdfNode::Iri(uri) => {
                        if uri.starts_with("gemini://") || uri.starts_with("http") {
                            output.push_str(&format!("=> {} {}\n", uri, shorten_uri(uri)));
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
                        if dt.starts_with("gemini://") || dt.starts_with("http") {
                            output.push_str(&format!("=> {} \"{}\"^^{}\n", dt, v, shorten_uri(dt)));
                        } else {
                            output.push_str(&format!("* \"{}\"^^{}\n", v, shorten_uri(dt)));
                        }
                    }
                }
            }
            output.push('\n');
        }
    }
    
    output
}

/// Generate a Gemtext response for a proxied resource
/// 
/// When `condensed` is true, properties are grouped together with all their objects
/// listed underneath each property heading.
pub fn generate_proxy_response(
    original_url: &str, 
    properties: &[(String, RdfNode)],
    condensed: bool,
    hostname: &str
) -> String {
    let mut body = format!("# Proxy: {}\n\n", shorten_uri(original_url));
    
    let formatted = if condensed {
        format_proxy_properties_condensed(properties, hostname)
    } else {
        format_proxy_properties_expanded(properties, hostname)
    };
    body.push_str(&formatted);
    
    body
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
fn format_proxy_properties_condensed(properties: &[(String, RdfNode)], hostname: &str) -> String {
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
            output.push_str(&format!("## {}\n", shorten_uri(predicate)));
            if predicate.starts_with("http") {
                let encoded = utf8_percent_encode(predicate, NON_ALPHANUMERIC).to_string();
                output.push_str(&format!("=> gemini://{}/{} ↗ {}\n", hostname, encoded, shorten_uri(predicate)));
            } else if predicate.starts_with("gemini://") {
                output.push_str(&format!("=> {} ↗ {}\n", predicate, shorten_uri(predicate)));
            }
            
            for object in objects {
                match object {
                    RdfNode::Iri(uri) => {
                        if uri.starts_with("http") {
                            let encoded = utf8_percent_encode(uri, NON_ALPHANUMERIC).to_string();
                            output.push_str(&format!("=> gemini://{}/{} {}\n", hostname, encoded, shorten_uri(uri)));
                        } else if uri.starts_with("gemini://") {
                            output.push_str(&format!("=> {} {}\n", uri, shorten_uri(uri)));
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
                            output.push_str(&format!("=> gemini://{}/{} \"{}\"^^{}\n", hostname, encoded, v, shorten_uri(dt)));
                        } else if dt.starts_with("gemini://") {
                            output.push_str(&format!("=> {} \"{}\"^^{}\n", dt, v, shorten_uri(dt)));
                        } else {
                            output.push_str(&format!("* \"{}\"^^{}\n", v, shorten_uri(dt)));
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
