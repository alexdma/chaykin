# Chaykin: Linked Data over the Small Web

A Linked Data server in Rust that makes the Semantic Web available over the Small Web (or "smolweb") through the [Gemini protocol](https://geminiprotocol.net/).

## Synopsis

Gemini is a minimalistic, inextensible, read-only application protocol with cryptography support. It is intended to live alongside HTTP, but also with Gopher and other, more recent application protocols that share the same philosophy of a functional Web that doesn't assume bloated clients to run.

Chaykin tries to follow that spirit by offering an application that can act as a Linked Data server in its own right, or as a proxy to existing Linked Open Data (note the difference here, though I very much hope that, if you choose to serve data through it, they will still be open data). With one instance of Chaykin, you can host a Gemini capsule that does both.

### Why bother?

The contemporary Web is a far cry from the one TimBL (was) promised. At the same time, [Gopher site count is increasing](https://en.wikipedia.org/wiki/Gopher_(protocol)#Server_census). This suggests that those true to the traditional spirit of the Web are attempting to remediate the current situation through alternative protocols, without losing the best of the HTTP world.

With this project, I want to help bring the Web of Data philosophy into that world that stays true to the spirit of the Web of Information, whilst at the same time being a bridge to the better part of the HTTP web.

### Why Chaykin?

[Lester Chaykin](https://another-world-game.fandom.com/wiki/The_Story_of_Lester_Knight_Chaykin) is a fictional particle physicist and the protagonist of Eric Chahi's classic video game [Another World](http://www.wikidata.org/entity/Q257469) ("Out of This World" for my American friends). Much like Dr Chaykin is transported out of this world and into another, this project attempts to bring Linked Data out of the cluttered, HTTP-based web and into the dimension of the small web.

Another World was also fascinating through its many ways of being minimalistic: in the aesthetic (using cinematic animation on top of a bare-polygon 3D engine), interface (HUDless), and narrative (dialogue-free). Similarly, the minimalist Gemini protocol is here used to host the beauty and complexity of linked data and knowledge graphs.

Finally, Another World was first released on the Amiga computer, and there are Gemini browsers running on AmigaOS with which you can now explore Linked Data too (running this _Rust server_ on the Amiga might be trickier though).

### Why Rust?

Because I've wanted to learn it for long; because a significant chunk of [Gemini software](https://geminiprotocol.net/software/) is in Rust; and because such a minimalist protocol calls for an optimised implementation, which I very much hope to someday deliver in this blazing fast language.

### Why you?

Because I do research on the applicability of the Semantic Web and because, if that wasn't clear already, I am a retrogamer--and a gaming historian in training, I daresay--and miss the thrill of the 1990's Web that was largely there to support the likes of me. So there.

DOI of a panel at Hypertext 2023 where I made my case: [10.1145/3603163.3609074](https://doi.org/10.1145/3603163.3609074)

## Features
- **Gemini Server**: Custom Tokio+Rustls implementation.
- **Linked Data Store**: Consumes RDF data in Turtle via `rio_turtle` and holds them into an in-memory store.
- **Gemtext Mapping**: A proposed serialization of RDF to the hypertext format of Gemini, offering a recursively browsable knowledge graph.
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
See [Another World on Wikidata](http://www.wikidata.org/entity/Q257469):
```bash
# Encoded URL: http://www.wikidata.org/entity/Q257469
printf "gemini://localhost/http%3A%2F%2Fwww.wikidata.org%2Fentity%2FQ257469\r\n" | openssl s_client -connect 127.0.0.1:1965 -quiet
```
(Note: you will need to replace every `%` with `%%` for printf escaping in bash).

**Output:**
```text
20 text/gemini
# Proxy: http://www.wikidata.org/entity/Q257469
...
=> gemini://localhost/http%3A%2F%2Fdata%2Ebnf%2Efr%2Fark%3A%2F12148%2Fcb169157795%23about http://www.wikidata.org/prop/direct-normalized/P268 : http://data.bnf.fr/ark:/12148/cb169157795#about 
...
* http://www.wikidata.org/prop/direct/P1476: LanguageTaggedString { value: "Out of This World", language: "en" }
...
```
If you got this, then the server successfully:
1.  Decoded the URL.
2.  Fetched the Turtle data from `www.wikidata.org`.
3.  Parsed the triples.
4.  Found the subject (handling http/https mismatch automatically).
5.  generated links pointing back to `gemini://localhost/...`.

## TODO
Lots and lots, but mainly:
- Move to RDF support via [Sophia](https://docs.rs/sophia/) and access existing triple stores.
- Make it an extension of existing Gemini servers in Rust like [Agate](https://github.com/mbrubeck/agate).
- SPARQL API? Only if it can respect the basic principles of the Small Web.
- Support something along the lines of the [Titan protocol](https://transjovian.org/view/titan/) if we need to have something like HTTP POST (which we would if SPARQL were to be implemented). Could also be useful if we want a form-like frontend for the user to enter a custom Linked Data URI.
- Full specification, including grammar, of the Gemtext RDF serialization (with support for labels!).

## Rights
This is free software; see [LICENSE](LICENSE).