mod store;
mod gemtext;
mod protocol;
use store::Store; // Local RDF store implementation
use std::sync::Arc; // Thread-safe reference counting for shared state
use tokio::net::TcpListener; // Async TCP listener
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, rustls}; // TLS support (Gemini mandates TLS)
use anyhow::{Result, Context}; // Error handling with context

use clap::{Parser, Subcommand}; // CLI argument parsing
use std::fs::File; 
use std::io::BufReader;
use std::path::PathBuf;
use rustls_pemfile::{certs, private_key};

const USER_AGENT: &str = concat!("Chaykin-Gemini-Proxy/", env!("CARGO_PKG_VERSION"), " (+https://github.com/alexdma/chaykin)");

const AVAILABLE_PROTOCOLS: &[&str] = &["gemini", "spartan", "nex"];

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1", global = true)]
    host: String,

    /// Port number for Gemini/Titan to bind to
    #[arg(long, alias = "port", default_value_t = 1965, global = true)]
    gemini_port: u16,

    /// Path to the RDF data file
    #[arg(long, default_value = "sample_data.ttl", global = true)]
    file: String,

    /// Path to TLS certificate (PEM)
    #[arg(long, global = true)]
    cert: Option<PathBuf>,

    /// Path to TLS private key (PEM)
    #[arg(long, global = true)]
    key: Option<PathBuf>,

    /// Hostname for generating self-referencing links
    #[arg(long, default_value = "localhost", global = true)]
    hostname: String,

    /// Spartan port number to bind to
    #[arg(long, default_value_t = 300, global = true)]
    spartan_port: u16,

    /// Nex port number to bind to
    #[arg(long, default_value_t = 1900, global = true)]
    nex_port: u16,

    /// NPS port number to bind to
    #[arg(long, default_value_t = 1915, global = true)]
    nps_port: u16,

    /// Comma-separated list of protocols to enable (e.g. gemini,spartan,nex)
    #[arg(long, value_delimiter = ',', global = true)]
    protocols: Option<Vec<String>>,

    /// Disable Titan (write protocol for Gemini)
    #[arg(long, global = true)]
    disable_titan: bool,

    /// Disable NPS (write protocol for Nex)
    #[arg(long, global = true)]
    disable_nps: bool,

    /// Preferred language for language-tagged literals (e.g. en, fr, de)
    #[arg(long, global = true)]
    lang: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the Linked Data server (default if no subcommand is given)
    Serve,
    /// List available protocols that can be passed to --protocols
    ListProtocols,
}


/// Determine which protocols are enabled based on CLI arguments.
fn enabled_protocols(cli: &Cli) -> Result<Vec<String>> {
    if let Some(ref list) = cli.protocols {
        let mut enabled = Vec::new();
        for p in list {
            let lower = p.to_lowercase();
            if !AVAILABLE_PROTOCOLS.contains(&lower.as_str()) {
                anyhow::bail!("Unknown protocol: {}. Available: {}", lower, AVAILABLE_PROTOCOLS.join(", "));
            }
            enabled.push(lower);
        }
        return Ok(enabled);
    }

    // Default: all protocols
    Ok(AVAILABLE_PROTOCOLS.iter().map(|s| s.to_string()).collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    use clap::CommandFactory;
    use clap::FromArgMatches;

    let cmd = Cli::command();
    let matches = cmd.get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    match cli.command {
        Some(Command::ListProtocols) => {
            println!("Available protocols:");
            for proto in AVAILABLE_PROTOCOLS {
                println!("  {}", proto);
            }
            Ok(())
        },
        // Default: run the server (both `chaykin` and `chaykin serve`)
        Some(Command::Serve) | None => {
            let enabled = enabled_protocols(&cli)?;
            
            // Strict Validation
            validate_cli_args(&cli, &enabled, &matches)?;
            
            run_server(cli, enabled).await
        },
    }
}

fn validate_cli_args(cli: &Cli, enabled: &[String], matches: &clap::ArgMatches) -> Result<()> {
    use clap::parser::ValueSource;

    let is_enabled = |p: &str| enabled.contains(&p.to_string());

    // 1. Disable Flag Validation
    if cli.disable_titan && !is_enabled("gemini") {
        anyhow::bail!("--disable-titan is only meaningful if gemini is enabled.");
    }
    if cli.disable_nps && !is_enabled("nex") {
        anyhow::bail!("--disable-nps is only meaningful if nex is enabled.");
    }

    // 2. Port Validation (Sloppiness check)
    // We check if the port was explicitly provided via command line
    let port_args = [
        ("gemini_port", "gemini", "gemini"),
        ("spartan_port", "spartan", "spartan"),
        ("nex_port", "nex", "nex"),
    ];

    for (arg, protocol, label) in port_args {
        if matches.value_source(arg) == Some(ValueSource::CommandLine) && !is_enabled(protocol) {
            anyhow::bail!("Argument --{} was provided, but protocol {} is not enabled.", 
                arg.replace('_', "-"), label);
        }
    }

    // Special case for NPS port
    if matches.value_source("nps_port") == Some(ValueSource::CommandLine) {
        if !is_enabled("nex") {
            anyhow::bail!("Argument --nps-port was provided, but protocol nex is not enabled.");
        }
        if cli.disable_nps {
            anyhow::bail!("Argument --nps-port was provided, but protocol nps is disabled via --disable-nps.");
        }
    }

    Ok(())
}

async fn run_server(cli: Cli, enabled: Vec<String>) -> Result<()> {
    println!("Starting Chaykin (Gemini Linked Data Server)...");
    println!("Enabled protocols: {}", enabled.join(", "));

    // Load RDF Store
    let mut store = Store::new();
    if let Err(e) = store.load_from_file(&cli.file)
    .context(format!("Could not load sample data from {}", cli.file)) {
        eprintln!("Warning: {:?}", e);
    }
    println!("Loaded {} triples from {}.", store.triple_count(), cli.file);
    let store = Arc::new(store);

    // Create a shared HTTP client for proxying
    let http_client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("Failed to create HTTP client")?;
    let http_client = Arc::new(http_client);
    let hostname = Arc::new(cli.hostname.clone());
    let lang = Arc::new(cli.lang.clone());

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();

    // Gemini & Titan Server (TLS)
    if enabled.contains(&"gemini".to_string()) {
        let (listener, acceptor) = create_tls_server(&cli.host, cli.gemini_port, cli.cert, cli.key).await?;

        let gemini_store = store.clone();
        let gemini_http = http_client.clone();
        let gemini_hostname = hostname.clone();
        let gemini_lang = lang.clone();
        let disable_titan = cli.disable_titan;
    
        tasks.push(tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let acceptor = acceptor.clone();
                        let store = gemini_store.clone();
                        let http_client = gemini_http.clone();
                        let hostname = gemini_hostname.clone();
                        let lang = gemini_lang.clone();

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
                                                if disable_titan {
                                                    // Titan disabled, ignore or return error? 
                                                    // Standard Gemini behavior: treat it as a request or error.
                                                    // For now just ignore as per instruction to not run handler.
                                                    eprintln!("Titan request ignored (Titan is disabled)");
                                                } else {
                                                    if let Err(e) = protocol::titan::handle_connection(tls_stream, request_line, buf[..n].to_vec(), store, http_client, hostname, lang).await {
                                                        eprintln!("Titan error: {:?}", e);
                                                    }
                                                }
                                            } else {
                                                if let Err(e) = protocol::gemini::handle_connection(tls_stream, request_line, store, http_client, hostname, lang).await {
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
        }));
    }

    // Spartan Server
    if enabled.contains(&"spartan".to_string()) {
        if cli.spartan_port < 1024 {
            println!("********************************************************************************");
            println!("* WARNING: Spartan is configured on port {}.                                  *", cli.spartan_port);
            if cli.spartan_port == 300 {
                println!("* This is the standard port for the Spartan protocol, however...               *");
            }
            println!("* Ports below 1024 are privileged on many systems (e.g. Linux, macOS).         *");
            println!("* Execution might fail unless you run as root or specify a higher port.        *");
            println!("********************************************************************************");
        }
        let spartan_addr = format!("{}:{}", cli.host, cli.spartan_port);
        let spartan_listener = tokio::net::TcpListener::bind(&spartan_addr).await.context("Failed to bind spartan port")?;
        println!("Listening on spartan://{}", spartan_addr);
        
        let spartan_store = store.clone();
        let spartan_http = http_client.clone();
        let spartan_hostname = hostname.clone();
        let spartan_lang = lang.clone();

        tasks.push(tokio::spawn(async move {
            loop {
                match spartan_listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let store = spartan_store.clone();
                        let http_client = spartan_http.clone();
                        let hostname = spartan_hostname.clone();
                        let lang = spartan_lang.clone();
                        tokio::spawn(async move {
                            if let Err(e) = protocol::spartan::handle_connection(stream, peer_addr, store, http_client, hostname, lang).await {
                                eprintln!("Spartan Error: {:?}", e);
                            }
                        });
                    }
                    Err(e) => eprintln!("Spartan accept error: {:?}", e),
                }
            }
        }));
    }

    // Nex & NPS Server
    if enabled.contains(&"nex".to_string()) {
        let nex_addr = format!("{}:{}", cli.host, cli.nex_port);
        let nex_listener = tokio::net::TcpListener::bind(&nex_addr).await.context("Failed to bind nex port")?;
        println!("Listening on nex://{}", nex_addr);
        
        let nex_store = store.clone();
        let nex_http = http_client.clone();
        let nex_hostname = hostname.clone();
        let nex_lang = lang.clone();

        let nex_store_2 = nex_store.clone();
        let nex_http_2 = nex_http.clone();
        let nex_hostname_2 = nex_hostname.clone();
        let nex_lang_2 = nex_lang.clone();

        tasks.push(tokio::spawn(async move {
            loop {
                match nex_listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let store = nex_store.clone();
                        let http_client = nex_http.clone();
                        let hostname = nex_hostname.clone();
                        let lang = nex_lang.clone();
                        tokio::spawn(async move {
                            if let Err(e) = protocol::nex::handle_connection(stream, peer_addr, store, http_client, hostname, lang).await {
                                eprintln!("Nex Error: {:?}", e);
                            }
                        });
                    }
                    Err(e) => eprintln!("Nex accept error: {:?}", e),
                }
            }
        }));

        // NPS is ancillary to Nex
        if !cli.disable_nps {
            let nps_addr = format!("{}:{}", cli.host, cli.nps_port);
            let nps_listener = tokio::net::TcpListener::bind(&nps_addr).await.context("Failed to bind nps port")?;
            println!("Listening on nps://{}", nps_addr);

            tasks.push(tokio::spawn(async move {
                loop {
                    match nps_listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            let store = nex_store_2.clone();
                            let http_client = nex_http_2.clone();
                            let hostname = nex_hostname_2.clone();
                            let lang = nex_lang_2.clone();
                            tokio::spawn(async move {
                                if let Err(e) = protocol::nps::handle_connection(stream, peer_addr, store, http_client, hostname, lang).await {
                                    eprintln!("NPS Error: {:?}", e);
                                }
                            });
                        }
                        Err(e) => eprintln!("NPS accept error: {:?}", e),
                    }
                }
            }));
        }
    }

    futures::future::join_all(tasks).await;
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
