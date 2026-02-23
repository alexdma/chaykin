# Chaykin: Linked Data over the Small Web

A Linked Data server in Rust that makes the Semantic Web available over the Small Web (or "smolweb") through a variety of protocols, including [Gemini](https://geminiprotocol.net/), [Titan](https://transjovian.org/view/titan/index), [Spartan](https://spartan.mozz.us/), and [Nex](https://nightfall.city/nex/info/).

## Synopsis

Several internet communities, from retro-computing enthusiasts to advocates for IndieWeb, decentralization and data sovereignty, are proposing and implementing application protocols that are intended to live alongside HTTP (or the seminal Gopher, for that matter) but are simpler and allow the user to focus on the information sought, rather than the presentation. [Gemini](https://geminiprotocol.net/) is the best-known of these, and possibly the most thoroughly specified: a minimalist, inextensible, read-only application protocol with cryptography support and even its own hypertext format, [Gemtext](https://geminiprotocol.net/docs/gemtext-specification.gmi). Other protocols provide different features, such as data upload (Titan, Spartan) or a more flexible approach to content negotiation (Nex), but all ultimately share the same philosophy of a functional Web that doesn't need bloated clients to run.

Chaykin tries to follow that spirit by offering an application that can act as a Linked Data server in its own right, or as a proxy to existing Linked Open Data (note the "Open" here, though I very much hope that, if you choose to serve data through it, they will indeed be open data). With one instance of Chaykin, you can host a site (or _capsule_ in Gemini parlance, or _station_ in Nex-talk) that does both.

### Why bother?

The contemporary Web is a far cry from the one we were promised: that should be before all our eyes by now. At the same time, [Gopher site count is increasing](https://en.wikipedia.org/wiki/Gopher_(protocol)#Server_census). This suggests that those true to the traditional spirit of the Web are attempting to remediate the current situation through alternative protocols, without losing the best of the HTTP world. These brilliant minds, however, seem largely focused on the vernacular Web of Information, and yet the HTTP web has done _something_ good in its evolution, such as enabling the Semantic Web of Data. Is it not worth doing something with it, without all that clutter coming from social media, monetisation and the like?

This is my way of answering "yes" to that question. With this project, I want to help bring the Web of Data philosophy into that world that stays true to the spirit of the Web of Information, whilst at the same time being a bridge to the better part of the HTTP web.

### Why Chaykin?

[Lester Chaykin](https://another-world-game.fandom.com/wiki/The_Story_of_Lester_Knight_Chaykin) is a fictional particle physicist and the protagonist of Eric Chahi's classic video game [Another World](http://www.wikidata.org/entity/Q257469) ("Out of This World" for my American friends). Much like Dr Chaykin is transported out of this world and into another, this project attempts to bring Linked Data out of the cluttered, HTTP-based web and into the dimension of the small web.

Another World was also fascinating through its many ways of being minimalistic: in the aesthetic (using cinematic animation on top of a bare-polygon 3D engine), interface (HUDless), and narrative (dialogue-free). Similarly, minimalist Web protocols are here used to host the beauty and complexity of linked data and knowledge graphs.

Finally, Another World was first released on the Amiga computer, and there are Gemini browsers running on AmigaOS with which you can now explore Linked Data too (running this _Rust server_ on the Amiga might be trickier though).

### Why Rust?

Because I've wanted to learn it for long; because a significant chunk of [Gemini software](https://geminiprotocol.net/software/) is in Rust; and because such a minimalist protocol calls for an optimised implementation, which I very much hope to someday deliver in this blazing fast language.

### Why you?

Because I do research on the applicability of the Semantic Web and because, if that wasn't clear already, I am a retrogamer--and a gaming historian in training, I daresay--and miss the thrill of the 1990's Web that was largely there to support the likes of me. So there.

DOI of a panel at Hypertext 2023 where I made my case: [10.1145/3603163.3609074](https://doi.org/10.1145/3603163.3609074)

## Features
- **Multi-Protocol Server**: Custom Tokio+Rustls implementation handling Gemini and Titan natively over TLS, alongside plaintext Spartan and Nex TCP listeners.
- **Linked Data Store**: Consumes RDF data in Turtle via `rio_turtle` and holds them into an in-memory store.
- **Gemtext Mapping**: A proposed serialization of RDF to the hypertext format of Gemini, offering a recursively browsable knowledge graph.  A condensed syntax, which groups predicates by property, is also supported. The specification is documented at [docs/rdf_gemtext_spec.md](docs/rdf_gemtext_spec.md).
- **External Proxy**: Acts as a browser for all the Linked Open Data out there.
    - Encoded URLs in the request path are fetched via `reqwest`.
    - `Accept: text/turtle` is used for Content Negotiation.
    - Fetched RDF is parsed and rendered.
    - Links to other external resources are re-encoded to point back to the proxy.
    - You can provide a custom TLS certificate via the `--cert` flag.

## Setup & Running
Pretty standard stuff:
1. **Dependencies**: `tokio`, `rustls`, `rcgen`, `rio_turtle`, `rio_api`, `reqwest`, `percent-encoding`.
2. **Build**:
   ```bash
   cd server
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
   The server listens on `127.0.0.1:1965` by default (for Gemini/Titan). Go there with your favourite Gemini client, like [Lagrange](https://gmi.skyjake.fi/lagrange/) (simple, stylish but without support for Titan) or [Alhena](https://metaloupe.com/alhena/alhena.html) (not as fancy, but flexible and multi-protocol).

   To run it in a Docker container (mapping the Spartan port to one that doesn't require a super user):
   ```bash
   docker run -p 1965:1965 -p 3300:300 -p 1900:1900 chaykin
   ```

### Configuration

You can configure the server using command-line arguments:

```bash
cargo run -- --help
```

Arguments:
- `--host`: IP address to bind to (default: 127.0.0.1)
- `--port`: Port number for Gemini/Titan to bind to (default: 1965)
- `--spartan-port`: Port number for Spartan (default: 300)
- `--nex-port`: Port number for Nex (default: 1900)
- `--file`: Path to the RDF data file (default: sample_data.ttl)
- `--cert`: Path to TLS certificate (PEM)
- `--key`: Path to TLS private key (PEM)

If no certificate/key is provided, a self-signed certificate is generated for development.

## Usage & Verification
The homepage at `localhost` for each protocol provides helpful hints on where to go from there. To try each protocol straight away:

### 1. Gemini (Port 1965)
Fetch a local resource:
```bash
printf "gemini://localhost/me\r\n" | openssl s_client -connect 127.0.0.1:1965 -quiet
```

Proxy an external resource (e.g. [Another World on Wikidata](http://www.wikidata.org/entity/Q257469)), which must be URL-encoded:
```bash
printf "gemini://localhost/http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469\r\n" | openssl s_client -connect 127.0.0.1:1965 -quiet
```

### 2. Titan (Port 1965)
Upload a URI to inspect via Titan payload:
```bash
{ printf "titan://localhost/;size=33;mime=text/plain\r\nhttp://www.wikidata.org/entity/Q257469"; sleep 1; } | openssl s_client -crlf -verify_quiet -connect localhost:1965
```

### 3. Spartan (Port 300)
**NOTE**: Binding to ports below 1024 (like 300) may require root privileges on Linux/Unix systems.

Connect and pass the target URI as the Spartan payload:
```bash
printf "localhost / 35\r\nhttp://www.wikidata.org/entity/Q257469" | nc localhost 300
```

### 4. Nex (Port 1900)
Input the URL-encoded path to get a raw text breakdown:
```bash
echo -e "/http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469\n" | nc localhost 1900
```

## TODO
Lots and lots, but mainly:
- Allow selective enabling of protocols that a server instance should support.
- Move to RDF support via [Sophia](https://docs.rs/sophia/) and access existing triple stores.
- SPARQL API? Only if it can respect the basic principles of the Small Web.
- Investigate whether it's worth supporting good old Gopher, too.
- Where possible, add support for language-specific labels in the Gemtext RDF serialization (we don't have the luxury of an `Accept-Language` header here).
- Consider Chaykin extensions to existing Small Web servers in Rust, like [Agate](https://github.com/mbrubeck/agate).

## Rights
This is free software; see [LICENSE](LICENSE).