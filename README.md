# Chaykin: Linked Data over the Small Web

A Linked Data server in Rust that makes Linked Data from the Semantic Web available over the Small Web (or "smolweb") through the [Gemini protocol](https://geminiprotocol.net/).

## Synopsis

Gemini is a minimalistic, inextensible, read-only Web-like protocol with cryptography support. It is intended to live alongside HTTP, but also with Gopher and other, more recent application protocols that share the same philosophy of a functional Web that doesn't assume bloated clients to run.

### Why Chaykin?

[https://another-world-game.fandom.com/wiki/The_Story_of_Lester_Knight_Chaykin](Lester Chaykin) is a fictional particle physicist and the protagonist of Eric Chahi's classic video game _Another World_ (_Out of This World_ for my American friends). Much like Dr Chaykin is transported out of this world and into another, this project attempts to bring Linked Data out of the cluttered, HTTP-based web and into the dimension of the small web.

Another World was also fascinating through its many ways of being minimalistic: in the aesthetics (using cinematic animation on top of a bare-polygon 3D engine), interaction (HUDless), and narrative (dialogue-free). Similarly, the minimalist Gemini protocol is here used to host the beauty and complexity of linked data and knowledge graphs.

Finally, Another World was first released on the Amiga computer, and there are Gemini browsers running on AmigaOS for you to explore Linked Data with.

## Features
- **Gemini Server**: Custom Tokio+Rustls implementation.
- **Linked Data Store**: Consumes RDF data in Turtle via `rio_turtle` and holds them into an in-memory store.
- **Gemtext Mapping**: A proposed serialization of RDF to the hypertext format of Gemini, offering a recursively browsable knowledge graph.
- **External Proxy**: Acts as a browser for all the Linked Open Data out there.
    - Encoded URLs in the request path are fetched via `reqwest`.
    - `Accept: text/turtle` is used for Content Negotiation.
    - Fetched RDF is parsed and rendered.
    - Links to other external resources are re-encoded to point back to the proxy.

## Setup & Running
Pretty standard stuff:
1. **Dependencies**: `tokio`, `rustls`, `rcgen`, `rio_turtle`, `rio_api`, `reqwest`, `percent-encoding`.
2. **Build**:
   ```bash
   cd server
   cargo build
   ```
3. **Run**:
   Either launch the `chaykin` executable in `server/target`, or
   ```bash
   cargo run
   ```
   The server listens on `127.0.0.1:1965`. I'll make that configurable, eventually.

## Usage & Verification
### 1. Local Resource
```bash
printf "gemini://localhost/me\r\n" | openssl s_client -connect 127.0.0.1:1965 -quiet
```
Returns data about yours truly (from [sample_data.ttl](/server/sample_data.ttl)).

### 2. External Resource (Proxy)
Browse the Palazzo Colonna data from an Art History knowledge graph:
```bash
# Encoded URL: https://data.biblhertz.it/builtwork/zuccaro/406
printf "gemini://localhost/https%%3A%%2F%%2Fdata.biblhertz.it%%2Fbuiltwork%%2Fzuccaro%%2F406\r\n" | openssl s_client -connect 127.0.0.1:1965 -quiet
```
(Note: `%%` is for printf escaping in bash).

**Output:**
```text
20 text/gemini
# Proxy: https://data.biblhertz.it/builtwork/zuccaro/406

=> gemini://localhost/http%3A%2F%2Fwww%2Ecidoc%2Dcrm%2Eorg%2Fcidoc%2Dcrm%2FE18%5FPhysical%5FThing ...
* http://www.w3.org/2000/01/rdf-schema#label: Simple { value: "Palazzo Colonna" }
...
```
If you got this, then the server successfully:
1.  Decoded the URL.
2.  Fetched the Turtle data from `data.biblhertz.it`.
3.  Parsed the triples.
4.  Found the subject (handling http/https mismatch automatically).
5.  generated links pointing back to `gemini://localhost/...`.

## TODO
Lots and lots, but mainly:
- Move to RDF support via [https://docs.rs/sophia/](Sophia) and access existing triple stores.
- Make it an extension of existing Gemini servers in Rust like [https://github.com/mbrubeck/agate](Agate).
- Better TLS support: right now it is only supported via self-signed certificates.
- SPARQL API? Only if it can respect the basic principles of the Small Web.
- Support something along the lines of the [https://transjovian.org/view/titan/](Titan protocol) if we need to have something like HTTP POST (which we would if SPARQL were to be implemented).
- Full specification of the Gemtext RDF serialization.
- Make the server configurable.
- This documentation in gemtext :)

## Rights
This is free software; see [LICENSE](LICENSE).