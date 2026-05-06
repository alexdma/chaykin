# Chaykin: Linked Data over the Small Web

A Linked Data server in Rust that makes the Semantic Web available over the Small Web (or "smolweb") through a variety of protocols:
- [Gemini](https://geminiprotocol.net/)
- [Titan](https://transjovian.org/view/titan/index) (to send data to Gemini)
- [Spartan](https://spartan.mozz.us/)
- [Nex](https://nightfall.city/nex/info/)
- [NPS](https://nightfall.city/nps/info/) (to send data to Nex)

More on this in [docs/about.md](docs/about.md).

## Features
- **Multi-Protocol Server**: Custom Tokio+Rustls implementation handling Gemini and Titan natively over TLS, alongside plaintext Spartan and Nex/NPS TCP listeners. Enabled protocols and ports can be configured.
- **Linked Data Store**: Consumes RDF data in Turtle or RDF/XML via `rio` and holds them into an in-memory store.
- **Gemtext Mapping**: A proposed serialization of RDF to the hypertext format of Gemini, offering a recursively browsable knowledge graph. A condensed syntax, which groups predicates by property, is also supported. The parser and serializer are available as a separate crate at [gemtext-ld](gemtext-ld/). The specification is documented at [gemtext-ld/docs/rdf_gemtext_spec.md](gemtext-ld/docs/rdf_gemtext_spec.md).
- **External Proxy**: Acts as a browser for all the Linked Open Data out there.
    - Encoded URLs in the request path are fetched via `reqwest`.
    - `Accept: text/turtle` is used for Content Negotiation, with fallback to `application/rdf+xml`: this should cover most Linked Data services.
    - Fetched RDF is parsed and rendered.
    - Links to other external resources are re-encoded to point back to the proxy.
    - You can provide a custom TLS certificate via the `--cert` flag.

## Setup & Running
Pretty standard stuff:
1. **Dependencies**: Rust/Cargo>1.78; Mainly `tokio`, `rustls`, `rcgen`, `rio_turtle`, `rio_xml`, `rio_api`, and `reqwest`.
2. **Build**:
   ```bash
   cargo build
   ```
   Or, if you want to build it as a Docker image:
   ```bash
   docker build -t chaykin .
   ```
   (the above will build an image with a self-signed certificate)

3. **Run**:
   Either launch the `chaykin` executable in `server/target`, or
   ```bash
   cargo run
   ```
   The server listens on `127.0.0.1:1965` by default (for Gemini/Titan). Go there with your favourite Gemini client, like [Lagrange](https://gmi.skyjake.fi/lagrange/) (simple, stylish, with support for Spartan and, recently, for Titan as well) or [Alhena](https://metaloupe.com/alhena/alhena.html) (maybe not as fancy, but flexible and multi-protocol).

   To run it in a Docker container (mapping the Spartan port to one that doesn't require a super user):
   ```bash
   docker run -p 1965:1965 -p 3300:300 -p 1900:1900 -p 1915:1915 chaykin
   ```

### Configuration

You can configure the server using command-line arguments:

```bash
cargo run -- --help
```

Arguments:
- `--host`: IP address to bind to (default: 127.0.0.1)
- `--protocols`: Comma-separated list of enabled protocols. Run `cargo run -- list-protocols` to see the list (default: all)
- `--gemini-port`: Port number for Gemini/Titan to bind to (default: 1965)
- `--spartan-port`: Port number for Spartan (default: 300)
- `--nex-port`: Port number for Nex (default: 1900)
- `--nps-port`: Port number for NPS (default: 1915)
- `--disable-titan`: Disable Titan (assumes Gemini is enabled)
- `--disable-nps`: Disable NPS (assumes Nex is enabled)
- `--file`: Path to an initial RDF data file (default: sample_data.ttl)
- `--cert`: Path to TLS certificate (PEM)
- `--key`: Path to TLS private key (PEM)
- `--lang`: Preferred language for language-tagged literals in Gemtext (e.g. en, fr, de)

If no certificate/key is provided, a self-signed certificate is generated for development.

## Usage & Verification
The homepage at `localhost` for each protocol provides helpful hints on where to go from there. To try each protocol straight away:

### 1. Gemini (Port 1965)
Proxy an external resource (e.g. [Another World on Wikidata](http://www.wikidata.org/entity/Q257469)), which must be URL-encoded:
```bash
printf '%s\r\n' "gemini://localhost/http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469" | openssl s_client -connect 127.0.0.1:1965 -quiet
```

Fetch a local resource:
```bash
printf '%s\r\n' "gemini://localhost/me" | openssl s_client -connect 127.0.0.1:1965 -quiet
```

### 2. Titan (Port 1965, Gemini must be enabled)
Upload a URI to inspect via Titan payload:
```bash
{ printf "titan://localhost/;size=38;mime=text/plain\r\nhttp://www.wikidata.org/entity/Q257469"; sleep 1; } | openssl s_client -crlf -verify_quiet -connect localhost:1965
```

### 3. Spartan (Port 300)
**NOTE**: Binding to ports below 1024 (like 300) may require root privileges on Linux/Unix systems.

Connect and pass the target URI as the Spartan payload:
```bash
printf "localhost /submit 38\r\nhttp://www.wikidata.org/entity/Q257469" | nc localhost 300
```

### 4. Nex (Port 1900)
Input the URL-encoded path to get a raw text breakdown:
```bash
echo -e "/http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469\n" | nc localhost 1900
```

### 5. NPS (Port 1915, Nex must be enabled)
The following (taken from https://nightfall.city/nps/info/nps) opens a temporary file in your default editor (which you must have specified via the `$EDITOR` environment variable): enter your entity URI and save, and it will be sent to the NPS server.
```bash
T=`mktemp` && $EDITOR $T && echo "." >> $T && nc localhost 1915 < $T
```

## TODO
Lots and lots, but mainly:
- Move to RDF support via [Sophia](https://docs.rs/sophia/) and access existing triple stores.
- SPARQL API? Only if it can be implemented without compromising the basic principles of the Small Web (we're probably going to need a SPARQL engine that supports cross-protocol equivalence).
- Gemtext serialization improvements:
    - Also render statements for subjects other than the one in the request URI.
    - Context-sensitive links in Gemtext: make them `spartan://` or `gemini://` depending on the client request.
    - Where possible, add support for language-specific labels per client request (we don't have the luxury of an `Accept-Language` header here: there is a `lang` parameter in Gemini but that is set by the server in the response).
    - Support for quads, blank node expansion, RDF-star.
- Investigate whether it's worth supporting other protocols:
    - Minor [contemporary SmolNet ones](https://dbohdan.com/archive/scorpion/zzo38computer.org/smallweb.txt/): Guppy, Molerat, Scorpion, Scroll etc. The rule of thumb would be that there must be actively developed user-facing clients, such as browsers, supporting that protocol.
    - Good old ones: Gopher, Finger...
- Consider Chaykin extensions to existing Small Web servers in Rust, like [Agate](https://github.com/mbrubeck/agate).

## Rights
Copyright (c) 2023-2026 Alessandro Adamou.

Chaykin is licensed under either of:
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
