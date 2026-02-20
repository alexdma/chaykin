mod store;
mod gemtext;
mod protocol;
use store::Store; // Local RDF store implementation
use std::sync::Arc; // Thread-safe reference counting for shared state
use tokio::net::TcpListener; // Async TCP listener
use tokio_rustls::{TlsAcceptor, rustls}; // TLS support (Gemini mandates TLS)
use anyhow::{Result, Context}; // Error handling with context

use clap::Parser; // CLI argument parsing
use std::fs::File; 
use std::io::BufReader;
use std::path::PathBuf;
use rustls_pemfile::{certs, private_key};

const USER_AGENT: &str = "Chaykin-Gemini-Proxy/0.1.0 (+https://github.com/alexdma/chaykin)";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to bind to
    #[arg(long, default_value_t = 1965)]
    port: u16,

    /// Path to the RDF data file
    #[arg(long, default_value = "sample_data.ttl")]
    file: String,

    /// Path to TLS certificate (PEM)
    #[arg(long)]
    cert: Option<PathBuf>,

    /// Path to TLS private key (PEM)
    #[arg(long)]
    key: Option<PathBuf>,

    /// Hostname for generating self-referencing links
    #[arg(long, default_value = "localhost")]
    hostname: String,

    /// Spartan Port number to bind to
    #[arg(long, default_value_t = 300)]
    spartan_port: u16,

    /// Nex Port number to bind to
    #[arg(long, default_value_t = 1900)]
    nex_port: u16,
}


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("Starting Chaykin (Gemini Linked Data Server)...");

    // Load RDF Store
    let mut store = Store::new();
    if let Err(e) = store.load_from_file(&args.file)
    .context(format!("Failed to load sample data from {}", args.file)) {
        eprintln!("Warning: {:?}", e);
    }
    println!("Loaded {} triples from {}.", store.triple_count(), args.file);
    let store = Arc::new(store);

    // Create a shared HTTP client for proxying
    let http_client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("Failed to create HTTP client")?;
    let http_client = Arc::new(http_client);
    let hostname = Arc::new(args.hostname.clone());


    // Create and bind TLS server for Gemini & Titan
    let (listener, acceptor) = create_tls_server(&args.host, args.port, args.cert, args.key).await?;

    let gemini_store = store.clone();
    let gemini_http = http_client.clone();
    let gemini_hostname = hostname.clone();
    
    let gemini_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let acceptor = acceptor.clone();
                    let store = gemini_store.clone();
                    let http_client = gemini_http.clone();
                    let hostname = gemini_hostname.clone();

                    tokio::spawn(async move {
                        // Accept TLS
                        match acceptor.accept(stream).await {
                            Ok(mut tls_stream) => {
                                use tokio::io::AsyncReadExt;
                                let mut buf = [0; 1024];
                                if let Ok(n) = tls_stream.read(&mut buf).await {
                                    if n > 0 {
                                        let request = String::from_utf8_lossy(&buf[..n]).to_string();
                                        let request_line = request.split("\r\n").next().unwrap_or("").to_string();
                                        
                                        if request_line.starts_with("titan://") {
                                            if let Err(e) = protocol::titan::handle_connection(tls_stream, request_line, buf[..n].to_vec(), store, http_client, hostname).await {
                                                eprintln!("Titan error: {:?}", e);
                                            }
                                        } else {
                                            if let Err(e) = protocol::gemini::handle_connection(tls_stream, request_line, store, http_client, hostname).await {
                                                eprintln!("Gemini error: {:?}", e);
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => eprintln!("TLS accept error {}: {:?}", peer_addr, e),
                        }
                    });
                }
                Err(e) => eprintln!("TCP accept error: {:?}", e),
            }
        }
    });

    // Spartan Server
    let spartan_addr = format!("{}:{}", args.host, args.spartan_port);
    let spartan_listener = tokio::net::TcpListener::bind(&spartan_addr).await.context("Failed to bind spartan port")?;
    println!("Listening on spartan://{}", spartan_addr);
    
    let spartan_store = store.clone();
    let spartan_http = http_client.clone();
    let spartan_hostname = hostname.clone();

    let spartan_task = tokio::spawn(async move {
        loop {
            match spartan_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let store = spartan_store.clone();
                    let http_client = spartan_http.clone();
                    let hostname = spartan_hostname.clone();
                    tokio::spawn(async move {
                        if let Err(e) = protocol::spartan::handle_connection(stream, peer_addr, store, http_client, hostname).await {
                            eprintln!("Spartan Error: {:?}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Spartan accept error: {:?}", e),
            }
        }
    });

    // Nex Server
    let nex_addr = format!("{}:{}", args.host, args.nex_port);
    let nex_listener = tokio::net::TcpListener::bind(&nex_addr).await.context("Failed to bind nex port")?;
    println!("Listening on nex://{}", nex_addr);
    
    let nex_store = store.clone();
    let nex_http = http_client.clone();
    let nex_hostname = hostname.clone();

    let nex_task = tokio::spawn(async move {
        loop {
            match nex_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let store = nex_store.clone();
                    let http_client = nex_http.clone();
                    let hostname = nex_hostname.clone();
                    tokio::spawn(async move {
                        if let Err(e) = protocol::nex::handle_connection(stream, peer_addr, store, http_client, hostname).await {
                            eprintln!("Nex Error: {:?}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Nex accept error: {:?}", e),
            }
        }
    });

    let _ = tokio::join!(gemini_task, spartan_task, nex_task);
    Ok(())
}

async fn create_tls_server(ip: &str, port: u16, cert_path: Option<PathBuf>, key_path: Option<PathBuf>) -> Result<(TcpListener, TlsAcceptor)> {
    let (certs, key) = if let (Some(c), Some(k)) = (cert_path, key_path) {
        println!("Loading TLS certificate from {:?} and key from {:?}", c, k);
        let cert_file = File::open(&c).context("Failed to open certificate file")?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificates")?;

        let key_file = File::open(&k).context("Failed to open key file")?;
        let mut key_reader = BufReader::new(key_file);
        let key = private_key(&mut key_reader)
            .context("Failed to parse private key")?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;
        
        (certs, key)
    } else {
        println!("=== Generating self-signed certificate (dev mode) ===");
        
        // Generate self-signed certificate
        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string(), ip.to_string()];
        let cert = rcgen::generate_simple_self_signed(subject_alt_names)
            .context("Failed to generate certificate")?;
        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();
        
        println!("Certificate generated for: localhost, 127.0.0.1, {}", ip);
        println!("This certificate will be re-used for all connections this session.");

        let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];
        let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).unwrap();
        
        (certs, key)       
    };

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

