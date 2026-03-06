use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use percent_encoding::percent_decode_str;

use crate::store::Store;
use crate::protocol::{resolve_request, ResourceData};
// We can reuse parts of gemtext format for properties output
use crate::gemtext;

pub async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
    hostname: Arc<String>,
    lang: Arc<Option<String>>,
) -> Result<()> {
    println!("Accepted Nex connection from {}", peer_addr);
    stream.set_nodelay(true)?;

    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 { return Ok(()); }
    
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let request_url = request.trim(); 

    println!("Nex Request Path: {}", request_url);

    let decoded_request = percent_decode_str(request_url).decode_utf8_lossy().to_string();
    
    let path = if decoded_request.starts_with('/') {
        decoded_request[1..].to_string()
    } else {
        decoded_request.to_string()
    };

    if path.is_empty() {
        let mut root_response_body = String::new();
        root_response_body.push_str("Welcome to Chaykin Nex!\n\n");
        root_response_body.push_str("To inspect a Semantic Web URI, append it to the path.\n");
        root_response_body.push_str("NOTE: Due to clients collapsing slashes, you MUST URL-encode the URI!\n\n");
        root_response_body.push_str("=> /http%3A%2F%2Fdbpedia.org%2Fresource%2FEarth Example: Earth\n");
        root_response_body.push_str("=> /http%3A%2F%2Fxmlns.com%2Ffoaf%2F0.1%2FPerson Example: FOAF Person\n");
        root_response_body.push_str("=> /http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469 Example: Wikidata Out of This World\n");
        
        // Nex responds with raw body
        stream.write_all(root_response_body.as_bytes()).await?;
        stream.shutdown().await?;
        let mut trash = [0; 8];
        let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
        return Ok(());
    }

    let resource_data = resolve_request(&path, store, http_client).await?;
    let is_proxy = path.starts_with("http://") || path.starts_with("https://");

    match resource_data {
        ResourceData::Description(properties) => {
            let body = if is_proxy {
                gemtext::generate_proxy_response(&path, &properties, false, &hostname, &lang)
            } else {
                gemtext::generate_resource_response(&path, &properties, false, &hostname, &lang)
            };
            // Nex responses lack headers, just send body
            stream.write_all(body.as_bytes()).await?;
        },
        ResourceData::ProxyError(err) => {
            let body = gemtext::generate_error_response("Proxy Error", &err);
            stream.write_all(body.as_bytes()).await?;
        },
        ResourceData::DebugSubjects(count, subjects) => {
            let body = gemtext::generate_debug_response(&path, count, subjects);
            stream.write_all(body.as_bytes()).await?;
        },
        ResourceData::NotFound => {
            let body = gemtext::generate_not_found_response(&path);
            stream.write_all(body.as_bytes()).await?;
        }
    }

    stream.shutdown().await?;
    let mut trash = [0; 8];
    let _ = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut trash)).await;
    Ok(())
}
