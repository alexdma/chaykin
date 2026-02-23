use std::sync::Arc;
use crate::store::{Store, RdfNode};
use anyhow::Result;

pub mod gemini;
pub mod titan;
pub mod spartan;
pub mod nex;

/// Common data structures and abstractions shared among protocols

pub enum ResourceData {
    Description(Vec<(String, RdfNode)>),
    ProxyError(String),
    DebugSubjects(usize, Vec<String>),
    NotFound,
}

/// Resolves a requested lookup IRI, either by fetching it from the local store
/// or proxying it if it's an HTTP/HTTPS URL.
pub async fn resolve_request(
    path: &str,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
) -> Result<ResourceData> {
    // If it looks like a URL
    if path.starts_with("http://") || path.starts_with("https://") {
        println!("Proxying request to: {}", path);
        
        let resp = http_client.get(path)
            .header("Accept", "text/turtle, application/x-turtle") 
            .send()
            .await;
            
        match resp {
            Ok(r) => {
                if r.status().is_success() {
                    let body = r.text().await.unwrap_or_default();
                    
                    // Create transient store
                    let mut temp_store = Store::new();
                    if let Err(e) = temp_store.load_from_string(&body) {
                         return Ok(ResourceData::ProxyError(format!("Error parsing RDF: {:?}", e)));
                    }
                    
                    // Render for the requested subject (path)
                    let mut properties = temp_store.get_resource_description(path);
                    
                    if properties.is_empty() {
                         // Fallback: Try swapping http/https
                         let alt_path = if path.starts_with("https://") {
                             path.replace("https://", "http://")
                         } else {
                             path.replace("http://", "https://")
                         };
                         properties = temp_store.get_resource_description(&alt_path);
                    }
                    
                     if properties.is_empty() {
                        return Ok(ResourceData::DebugSubjects(
                            temp_store.triple_count(),
                            temp_store.get_all_subjects()
                        ));
                    } else {
                        return Ok(ResourceData::Description(properties));
                    }

                } else {
                    return Ok(ResourceData::ProxyError(format!("HTTP Status: {}", r.status())));
                }
            },
            Err(e) => {
                return Ok(ResourceData::ProxyError(format!("Network Error: {:?}", e)));
            }
        }
    }

    // Local resolution fallback
    // Hack: Replace 127.0.0.1 with localhost to match sample data
    let lookup_iri = path.replace("127.0.0.1", "localhost").replace(":1965", "").replace(":300", "").replace(":1900", "");
    let properties = store.get_resource_description(&lookup_iri);

    if properties.is_empty() {
        Ok(ResourceData::NotFound)
    } else {
        Ok(ResourceData::Description(properties))
    }
}
