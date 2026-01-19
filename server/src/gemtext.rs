use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// Generate a Gemtext response for a resource with its properties
/// 
/// When `condensed` is true, properties are grouped together with all their objects
/// listed underneath each property heading.
pub fn generate_resource_response(
    resource_iri: &str, 
    properties: &[(String, String)],
    condensed: bool
) -> String {
    let mut body = format!("# Resource: {}\n\n", resource_iri);
    
    let formatted = if condensed {
        format_properties_condensed(properties)
    } else {
        format_properties_expanded(properties)
    };
    body.push_str(&formatted);
    
    body.push_str("\n=> gemini://localhost/ Home\n");
    body
}

/// Format properties in expanded form (one line per property-object pair)
fn format_properties_expanded(properties: &[(String, String)]) -> String {
    let mut output = String::new();
    
    for (predicate, object) in properties {
        if object.starts_with("gemini://") || object.starts_with("http") {
            output.push_str(&format_link(predicate, object));
        } else {
            output.push_str(&format!("* {}: {}\n", predicate, object));
        }
    }
    
    output
}

/// Format properties in condensed form (grouped by predicate)
fn format_properties_condensed(properties: &[(String, String)]) -> String {
    use std::collections::HashMap;
    
    // Group objects by predicate
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
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
            output.push_str(&format!("## {}\n", predicate));
            
            for object in objects {
                if object.starts_with("gemini://") || object.starts_with("http") {
                    output.push_str(&format!("=> {}\n", object));
                } else {
                    output.push_str(&format!("* {}\n", object));
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
    properties: &[(String, String)],
    condensed: bool
) -> String {
    let mut body = format!("# Proxy: {}\n\n", original_url);
    
    let formatted = if condensed {
        format_proxy_properties_condensed(properties)
    } else {
        format_proxy_properties_expanded(properties)
    };
    body.push_str(&formatted);
    
    body
}

/// Format proxy properties in expanded form
fn format_proxy_properties_expanded(properties: &[(String, String)]) -> String {
    let mut output = String::new();
    
    for (predicate, object) in properties {
        if object.starts_with("gemini://") || object.starts_with("http") {
            // Encode external HTTP(S) links to route back through the proxy
            if object.starts_with("http") {
                let encoded = utf8_percent_encode(object, NON_ALPHANUMERIC).to_string();
                output.push_str(&format!("=> gemini://localhost/{} {} : {}\n", encoded, predicate, object));
            } else {
                output.push_str(&format!("=> {} {} : {}\n", object, predicate, object));
            }
        } else {
            output.push_str(&format!("* {}: {}\n", predicate, object));
        }
    }
    
    output
}

/// Format proxy properties in condensed form (grouped by predicate)
fn format_proxy_properties_condensed(properties: &[(String, String)]) -> String {
    use std::collections::HashMap;
    
    // Group objects by predicate
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
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
            output.push_str(&format!("## {}\n", predicate));
            
            for object in objects {
                if object.starts_with("gemini://") || object.starts_with("http") {
                    if object.starts_with("http") {
                        let encoded = utf8_percent_encode(object, NON_ALPHANUMERIC).to_string();
                        output.push_str(&format!("=> gemini://localhost/{} {}\n", encoded, object));
                    } else {
                        output.push_str(&format!("=> {}\n", object));
                    }
                } else {
                    output.push_str(&format!("* {}\n", object));
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
        resource_iri
    )
}

/// Generate a debug response showing available subjects
pub fn generate_debug_response(requested_iri: &str, triple_count: usize, subjects: Vec<String>) -> String {
    let mut msg = format!(
        "# No Data Found for {}\r\n\r\nLoaded {} triples.\n\n## Available Subjects:\n",
        requested_iri, triple_count
    );
    
    for subject in subjects {
        msg.push_str(&format!("* {}\n", subject));
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

/// Helper to format a link line in Gemtext
fn format_link(predicate: &str, object: &str) -> String {
    format!("=> {} {} : {}\n", object, predicate, object)
}
