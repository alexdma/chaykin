use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::server::TlsStream;
use anyhow::Result;
use crate::store::Store;
use crate::gemtext;
use crate::protocol::{resolve_request, ResourceData};

pub async fn handle_connection(
    mut stream: TlsStream<TcpStream>,
    request_line: String,
    mut initial_buffer: Vec<u8>,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
    hostname: Arc<String>,
    lang: Arc<Option<String>>,
) -> Result<()> {
    println!("Titan Request Line: {}", request_line);

    let mut size: usize = 0;
    for param in request_line.split(';') {
        if let Some(s) = param.strip_prefix("size=") {
            size = s.parse().unwrap_or(0);
        }
    }

    // Find the end of the request line in initial_buffer
    let mut header_len = 0;
    for i in 0..initial_buffer.len().saturating_sub(1) {
        if initial_buffer[i] == b'\r' && initial_buffer[i+1] == b'\n' {
            header_len = i + 2;
            break;
        }
    }
    
    // We need to read until we have header_len + size bytes
    let total_required = header_len + size;

    while initial_buffer.len() < total_required {
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf).await?;
        if n == 0 { break; }
        initial_buffer.extend_from_slice(&buf[..n]);
    }

    let payload_bytes = if initial_buffer.len() >= total_required {
        &initial_buffer[header_len..total_required]
    } else {
        &initial_buffer[header_len..]
    };

    let payload = String::from_utf8_lossy(payload_bytes).to_string();
    let path = payload.trim().to_string();
    
    if path.is_empty() {
        let response = gemtext::format_gemini_response("40 Validation Error: Missing Payload URI\r\n");
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    println!("Titan Payload queried URI: {}", path);

    let resource_data = resolve_request(&path, store, http_client).await?;
    let is_proxy = path.starts_with("http://") || path.starts_with("https://");

    match resource_data {
        ResourceData::Description(properties) => {
            let body = if is_proxy {
                gemtext::generate_proxy_response(&path, &properties, false, &hostname, &lang)
            } else {
                gemtext::generate_resource_response(&path, &properties, false, &hostname, &lang)
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
