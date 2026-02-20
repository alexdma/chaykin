use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio_rustls::server::TlsStream;
use anyhow::Result;
use percent_encoding::percent_decode_str;

use crate::store::Store;
use crate::gemtext;
use crate::protocol::{resolve_request, ResourceData};

pub fn parse_query_params(url: &str) -> (String, bool) {
    if let Some(pos) = url.find('?') {
        let (base, query) = url.split_at(pos);
        let condensed = query.contains("condensed=true");
        (base.to_string(), condensed)
    } else {
        (url.to_string(), false)
    }
}

pub async fn handle_connection(
    mut stream: TlsStream<TcpStream>,
    request_url: String,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
    hostname: Arc<String>,
) -> Result<()> {
    println!("Gemini Request: {}", request_url);

    // Provide a simple proxy root for the root path requesting URI
    if request_url == format!("gemini://{}/", hostname) || request_url == format!("gemini://{}", hostname) || request_url == "/" {
        let response = gemtext::format_gemini_response("10 Enter Linked Data URI to inspect\r\n");
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let decoded_request = percent_decode_str(&request_url).decode_utf8_lossy().to_string();
    let (clean_url, condensed) = parse_query_params(&decoded_request);
    
    // For input queries from the root
    let is_query_on_root = clean_url == format!("gemini://{}", hostname) || clean_url == format!("gemini://{}/", hostname);
    let path = if is_query_on_root {
        if let Some(pos) = request_url.find('?') {
            let (_, query) = request_url.split_at(pos + 1);
            let decoded_query = percent_decode_str(query).decode_utf8_lossy().to_string();
            decoded_query
        } else {
            "/".to_string()
        }
    } else {
        if let Some(p) = clean_url.strip_prefix("gemini://") {
            if let Some(slash_pos) = p.find('/') {
                p[slash_pos..].to_string()
            } else {
                "/".to_string()
            }
        } else {
            clean_url
        }
    };
    let path = path.trim_start_matches('/');
    
    // We treat empty path as querying empty? Or redirect to root.
    if path.is_empty() {
        let response = gemtext::format_gemini_response("10 Enter Linked Data URI to inspect\r\n");
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let resource_data = resolve_request(&path, store, http_client).await?;
    let is_proxy = path.starts_with("http://") || path.starts_with("https://");

    match resource_data {
        ResourceData::Description(properties) => {
            let body = if is_proxy {
                gemtext::generate_proxy_response(&path, &properties, condensed, &hostname)
            } else {
                gemtext::generate_resource_response(&path, &properties, condensed, &hostname)
            };
            let response = gemtext::format_gemini_response(&body);
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::ProxyError(err) => {
            let body = gemtext::generate_error_response("Proxy Error", &err);
            let response = gemtext::format_gemini_response(&body);
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::DebugSubjects(count, subjects) => {
            let body = gemtext::generate_debug_response(&path, count, subjects);
            let response = gemtext::format_gemini_response(&body);
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::NotFound => {
            let body = gemtext::generate_not_found_response(&path);
            let response = gemtext::format_gemini_response(&body);
            stream.write_all(response.as_bytes()).await?;
        }
    }

    Ok(())
}
