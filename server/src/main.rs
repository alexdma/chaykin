mod store;
mod gemtext;
use store::Store; // Local RDF store implementation
use std::sync::Arc; // Thread-safe reference counting for shared state
use std::net::SocketAddr; // Network address handling
use tokio::net::TcpListener; // Async TCP listener
use tokio_rustls::{TlsAcceptor, rustls}; // TLS support (Gemini mandates TLS)
use anyhow::{Result, Context}; // Error handling with context

const SAMPLE_DATA_PATH: &str = "sample_data.ttl";
const IP: &str = "127.0.0.1";
const PORT: u16 = 1965;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Chaykin (Gemini Linked Data Server)...");

    // Load RDF Store
    let mut store = Store::new();
    if let Err(e) = store.load_from_file(SAMPLE_DATA_PATH)
    .context("Failed to load sample data") {
        eprintln!("Warning: {:?}", e);
    }
    println!("Loaded {} triples.", store.triple_count());
    let store = Arc::new(store);

    // Create and bind TLS server
    let (listener, acceptor) = create_tls_server(IP, PORT).await?;

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let store = store.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, acceptor, peer_addr, store).await {
                eprintln!("Error handling connection from {}: {:?}", peer_addr, e);
            }
        });
    }
}

async fn create_tls_server(ip: &str, port: u16) -> Result<(TcpListener, TlsAcceptor)> {
    println!("=== Generating self-signed certificate (once at startup) ===");
    
    // Generate self-signed certificate
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)
        .context("Failed to generate certificate")?;
    let cert_der = cert.cert.der().to_vec();
    let key_der = cert.key_pair.serialize_der();
    
    println!("Certificate generated with SANs: localhost, 127.0.0.1");
    println!("This certificate will be re-used for all connections.");

    let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).unwrap();

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Failed to create TLS config")?;

    let acceptor = TlsAcceptor::from(Arc::new(config));
    let addr = format!("{}:{}", ip, port);
    let listener = TcpListener::bind(&addr).await
        .context(format!("Failed to bind to {}", addr))?;

    println!("Listening on gemini://{}", addr);

    Ok((listener, acceptor))
}

/// Parse query parameters from a Gemini URL
/// Returns (clean_url, condensed_flag)
fn parse_query_params(url: &str) -> (String, bool) {
    if let Some(pos) = url.find('?') {
        let (base, query) = url.split_at(pos);
        let condensed = query.contains("condensed=true");
        (base.to_string(), condensed)
    } else {
        (url.to_string(), false)
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    acceptor: TlsAcceptor,
    peer_addr: SocketAddr,
    store: Arc<Store>,
) -> Result<()> {
    let mut stream = acceptor.accept(stream).await?;
    println!("Accepted TLS connection from {}", peer_addr);

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    // Read URL (limit size for security)
    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 { return Ok(()); }
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let request_url = request.trim();
    println!("Request: {}", request_url);

    // Hack logic:
    // 1. If path starts with 'http', assume encoded proxy request.
    // 2. Else look up in local store.

    // Decode path
    use percent_encoding::percent_decode_str;
    let decoded_request = percent_decode_str(request_url).decode_utf8_lossy().to_string();
    
    // Check for query parameters (e.g., ?condensed=true)
    let (clean_url, condensed) = parse_query_params(&decoded_request);
    
    // Check if it's an external URL 
    // We check if it starts with http, or if the request_url was encoded.
    // Simple heuristic: if decoded starts with http(s)://, use fetch mode.
    
    // Note: Gemini requests are full URLs: gemini://host/path
    // If user requests gemini://host/http%3A%2F%2F...\n    // The path part is /http...\n    
    // We need to strip the gemini prefix first to see the path.
    let path = if let Some(p) = clean_url.strip_prefix("gemini://") {
        // p is host[:port]/path
        if let Some(slash_pos) = p.find('/') {
            &p[slash_pos..]
        } else {
            // It was just gemini://host[:port]
            "/"
        }
    } else {
        &clean_url
    };
    
    // Clean leading slash
    let path = path.trim_start_matches('/');
    
    // If it looks like a URL
    if path.starts_with("http://") || path.starts_with("https://") {
        println!("Proxying request to: {}", path);
        
        let client = reqwest::Client::new();
        let resp = client.get(path)
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
                         let error_body = gemtext::generate_error_response("Error parsing RDF", &format!("{:?}", e));
                         let response = gemtext::format_gemini_response(&error_body);
                         stream.write_all(response.as_bytes()).await?;
                         return Ok(());
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
                        // DEBUG: Print all subjects in store to see what we got
                        let debug_body = gemtext::generate_debug_response(
                            path,
                            temp_store.triple_count(),
                            temp_store.get_all_subjects()
                        );
                        let response = gemtext::format_gemini_response(&debug_body);
                        stream.write_all(response.as_bytes()).await?;
                    } else {
                        let body = gemtext::generate_proxy_response(path, &properties, condensed);
                        let response = gemtext::format_gemini_response(&body);
                        stream.write_all(response.as_bytes()).await?;
                    }

                } else {
                    let body = gemtext::generate_error_response(
                        "Fetch Error",
                        &format!("HTTP Status: {}", r.status())
                    );
                    let response = gemtext::format_gemini_response(&body);
                    stream.write_all(response.as_bytes()).await?;
                }
            },
            Err(e) => {
                let body = gemtext::generate_error_response("Network Error", &format!("{:?}", e));
                let response = gemtext::format_gemini_response(&body);
                stream.write_all(response.as_bytes()).await?;
            }
        }
        return Ok(());
    }

    // Hack: Replace 127.0.0.1 with localhost to match sample data
    // Also strip query params for the lookup
    let (clean_lookup_iri, _) = parse_query_params(request_url);
    let lookup_iri = clean_lookup_iri.replace("127.0.0.1", "localhost").replace(":1965", "");

    let properties = store.get_resource_description(&lookup_iri);

    if properties.is_empty() {
        let body = gemtext::generate_not_found_response(&lookup_iri);
        let response = gemtext::format_gemini_response(&body);
        stream.write_all(response.as_bytes()).await?;
    } else {
        let body = gemtext::generate_resource_response(&lookup_iri, &properties, condensed);
        let response = gemtext::format_gemini_response(&body);
        stream.write_all(response.as_bytes()).await?;
    }

    Ok(())
}
