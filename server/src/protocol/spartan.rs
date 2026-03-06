use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use percent_encoding::percent_decode_str;

use crate::store::Store;
use crate::gemtext;
use crate::protocol::{resolve_request, ResourceData};

pub async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
    hostname: Arc<String>,
    lang: Arc<Option<String>>,
) -> Result<()> {
    println!("Accepted Spartan connection from {}", peer_addr);

    stream.set_nodelay(true)?;

    // Read request line (up to \r\n)
    let mut buf = [0; 1024];
    let mut request_line = String::new();
    let nbytes_read;
    
    // Read byte by byte or just read a chunk and parse
    let n = stream.read(&mut buf).await?;
    if n == 0 { return Ok(()); }
    
    let chunk = String::from_utf8_lossy(&buf[..n]);
    if let Some(pos) = chunk.find("\r\n") {
        request_line.push_str(&chunk[..pos]);
        nbytes_read = pos + 2;
    } else {
        return Ok(());
    }

    println!("Spartan Request Line: {}", request_line);

    // Parse: host path length
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 3 {
        stream.write_all(b"4 Invalid request format\r\n").await?;
        stream.shutdown().await?;
        let mut trash = [0; 8];
        let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
        return Ok(());
    }

    let _host = parts[0];
    let req_path = parts[1];
    let length_str = parts[2];
    let length: usize = length_str.parse().unwrap_or(0);

    // Read payload if length > 0
    let mut payload = Vec::new();
    let initial_payload = &buf[nbytes_read..n];
    payload.extend_from_slice(initial_payload);
    
    while payload.len() < length {
        let mut pbuf = [0; 1024];
        let pn = stream.read(&mut pbuf).await?;
        if pn == 0 { break; }
        payload.extend_from_slice(&pbuf[..pn]);
    }
    
    let path = percent_decode_str(req_path).decode_utf8_lossy().to_string();

    let mut lookup_path = path.trim_start_matches('/').to_string();

    // If root
    if lookup_path.is_empty() {
        let mut root_response_body = String::new();
        root_response_body.push_str("# Welcome to Chaykin Spartan!\n\n");
        root_response_body.push_str("=: /submit Enter Linked Data URI to inspect\n");
        
        let response = format!("2 text/gemini\r\n{}", root_response_body);
        stream.write_all(response.as_bytes()).await?;
        stream.shutdown().await?;
        let mut trash = [0; 8];
        let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
        return Ok(());
    }

    // If submit (they submitted the prompt)
    if lookup_path == "submit" {
        let payload_str = String::from_utf8_lossy(&payload[..length]).to_string();
        lookup_path = payload_str.trim().to_string();
        
        if lookup_path.is_empty() {
            stream.write_all(b"4 Validation Error: Missing Payload URI\r\n").await?;
            stream.shutdown().await?;
            let mut trash = [0; 8];
            let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
            return Ok(());
        }
    }

    println!("Spartan querying URI: {}", lookup_path);

    let (clean_url, condensed) = gemini_parse_query_params(&lookup_path);
    let path_for_lookup = clean_url;

    let resource_data = resolve_request(&path_for_lookup, store, http_client).await?;
    let is_proxy = path_for_lookup.starts_with("http://") || path_for_lookup.starts_with("https://");

    match resource_data {
        ResourceData::Description(properties) => {
            let body = if is_proxy {
                gemtext::generate_proxy_response(&path_for_lookup, &properties, condensed, &hostname, &lang)
            } else {
                gemtext::generate_resource_response(&path_for_lookup, &properties, condensed, &hostname, &lang)
            };
            // Spartan success status is '2'
            let response = format!("2 text/gemini\r\n{}", body);
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::ProxyError(err) => {
            let body = gemtext::generate_error_response("Proxy Error", &err);
            let response = format!("5 {}\r\n", body.replace("\n", " ").trim());
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::DebugSubjects(count, subjects) => {
            let body = gemtext::generate_debug_response(&path_for_lookup, count, subjects);
            let response = format!("2 text/gemini\r\n{}", body);
            stream.write_all(response.as_bytes()).await?;
        },
        ResourceData::NotFound => {
            let response = "4 Resource not found\r\n";
            stream.write_all(response.as_bytes()).await?;
        }
    }

    stream.shutdown().await?;
    let mut trash = [0; 8];
    let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
    Ok(())
}

fn gemini_parse_query_params(url: &str) -> (String, bool) {
    if let Some(pos) = url.find('?') {
        let (base, query) = url.split_at(pos);
        let condensed = query.contains("condensed=true");
        (base.to_string(), condensed)
    } else {
        (url.to_string(), false)
    }
}
