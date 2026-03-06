use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use anyhow::Result;

use crate::store::Store;
use crate::gemtext;
use crate::protocol::{resolve_request, ResourceData};

/// Handle an NPS connection.
///
/// The NPS protocol is: client connects, sends lines of text,
/// finishes with a line containing only ".", server responds with text
/// and closes the connection.
pub async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    store: Arc<Store>,
    http_client: Arc<reqwest::Client>,
    hostname: Arc<String>,
    lang: Arc<Option<String>>,
) -> Result<()> {
    println!("Accepted NPS connection from {}", peer_addr);
    stream.set_nodelay(true)?;

    // Read lines until we get a line that is just "."
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut payload = String::new();

    loop {
        let mut line = String::new();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            // Connection closed before "." terminator
            break;
        }
        let trimmed = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if trimmed == "." {
            break;
        }
        payload.push_str(trimmed);
        payload.push('\n');
    }

    let path = payload.trim().to_string();

    if path.is_empty() {
        let mut response = String::new();
        response.push_str("Welcome to Chaykin NPS!\n\n");
        response.push_str("Send a Linked Data URI followed by a line containing only '.' to inspect it.\n");
        response.push_str("Example:\n");
        response.push_str("  http://www.wikidata.org/entity/Q257469\n");
        response.push_str("  .\n");
        writer.write_all(response.as_bytes()).await?;
        writer.shutdown().await?;
        return Ok(());
    }

    println!("NPS querying URI: {}", path);

    let resource_data = resolve_request(&path, store, http_client).await?;
    let is_proxy = path.starts_with("http://") || path.starts_with("https://");

    let response = match resource_data {
        ResourceData::Description(properties) => {
            if is_proxy {
                gemtext::generate_proxy_response(&path, &properties, false, &hostname, &lang)
            } else {
                gemtext::generate_resource_response(&path, &properties, false, &hostname, &lang)
            }
        },
        ResourceData::ProxyError(err) => {
            gemtext::generate_error_response("Proxy Error", &err)
        },
        ResourceData::DebugSubjects(count, subjects) => {
            gemtext::generate_debug_response(&path, count, subjects)
        },
        ResourceData::NotFound => {
            gemtext::generate_not_found_response(&path)
        }
    };

    writer.write_all(response.as_bytes()).await?;
    writer.shutdown().await?;
    Ok(())
}
