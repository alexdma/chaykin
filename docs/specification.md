# Linked Data over Small Web — Specification

**Derived from**: Chaykin v0.x implementation  
**Basis**: Gemini, Titan, Spartan, Nex, NPS protocol behaviours; RDF-in-Gemtext spec v0.2

---

## 1. Scope

This document specifies a contract for publishing and consuming [RDF](https://www.w3.org/TR/rdf11-concepts/) Linked Data over Small Web protocols. Section 3 defines the **Core Contract** that all conformant protocol bindings must satisfy. Section 4 defines per-protocol bindings, some of which are optional (e.g. data submission).

---

## 2. Data Model and Serialization

### 2.1 Information Unit

The atom of information is the **RDF triple**: `(subject IRI, predicate IRI, object)`, as defined in the [RDF 1.1 Concepts and Abstract Syntax](https://www.w3.org/TR/rdf11-concepts/) W3C Recommendation (§3.1). In a valid triple:

- the **subject** is an IRI or blank node;
- the **predicate** is an IRI;
- the **object** is an IRI, blank node, or literal.

A **resource description** is the set of all triples sharing a subject IRI.

### 2.2 Storage Format (Server Side)

Servers MUST store data as [Turtle](https://www.w3.org/TR/turtle/). Servers MAY additionally support [RDF/XML](https://www.w3.org/TR/rdf-syntax-grammar/) as a fallback parse format.

### 2.3 Wire Format: RDF-in-Gemtext

All protocol bindings use **[Gemtext](https://geminiprotocol.net/docs/gemtext-specification.gmi)** as the sole wire format for RDF data. Gemtext is a line-oriented, prefix-dispatched markup format with a one-line-one-purpose constraint: each line carries exactly one syntactic role, determined by its prefix, and no inline markup is permitted. The serialization rules are fully defined in the **RDF-in-Gemtext Specification** (see [gemtext-rdf/docs/rdf_gemtext_spec.md](../gemtext-rdf/docs/rdf_gemtext_spec.md)). A summary follows.

> **Note on Nex**: although this implementation returns Gemtext over the Nex protocol, Nex itself only mandates a plaintext format in which lines beginning with `=>` are treated as link lines. A Nex client will follow `=>` links but treat all other lines as plain text. Gemtext is therefore a valid Nex payload, but Nex does not assume Gemtext semantics beyond link lines.

**Document structure** — a response body for a resource description:

```
# Resource: <subject IRI or QName>

<one property block per predicate>
```

Two modes:

- **Expanded** (default): one Gemtext line per triple.
- **Condensed** (optional): triples grouped under `## <predicate>` headings.

**Object encoding (expanded mode)**:

| Object type | Output line |
|---|---|
| IRI (HTTP or Gemini scheme) | `=> <uri> <shortPredicate> : <shortUri>` |
| IRI (other scheme) | `* <shortPredicate>: <uri>` |
| Blank node | `* <shortPredicate>: _:<id>` |
| Simple literal | `* <shortPredicate>: "<value>"` |
| Language-tagged literal | `* <shortPredicate>: "<value>"@<lang>` |
| Datatyped literal (HTTP/Gemini datatype URI) | `=> <datatypeUri> <shortPredicate> : "<value>"^^<shortType>` |
| Datatyped literal (other datatype URI) | `* <shortPredicate>: "<value>"^^<shortType>` |

**Registered prefixes** (QName shortening):

| Prefix | Namespace |
|---|---|
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` |
| `dc:` | `http://purl.org/dc/elements/1.1/` |
| `dcterms:` | `http://purl.org/dc/terms/` |
| `foaf:` | `http://xmlns.com/foaf/0.1/` |
| `owl:` | `http://www.w3.org/2002/07/owl#` |
| `schema:` | `http://schema.org/` |

Link targets (`=>` URLs) always carry the **full IRI**; QName shortening applies to display text only, enabling round-trip parsing.

---

## 3. Core Contract (All Protocol Bindings)

These rules apply regardless of transport protocol.

### 3.1 Resource Identification

Every resource is identified by an IRI. IRIs may use any scheme, but the server's own resources use the native scheme of the serving protocol (e.g. `gemini://`, `spartan://`). The server strips its own port numbers and normalises `127.0.0.1` → `localhost` before IRI lookup.

### 3.2 Request

A client sends exactly one IRI (or a path from which the IRI is derived) per request. The IRI MUST be percent-decoded before lookup.

### 3.3 Response

The server responds with a **Gemtext body** encoding the resource description of the requested IRI. Four outcome types are defined:

| Outcome | Body content |
|---|---|
| **Found** | `# Resource: <iri>` heading followed by property lines |
| **Not Found** | `# Not Found` heading with the requested IRI |
| **Error** | `# <Error Title>` heading with an error message |
| **Debug** | List of all known subject IRIs (returned when no subject matches but the store is non-empty) |

### 3.4 HTTP/HTTPS Proxy Behaviour

If the requested IRI begins with `http://` or `https://`, the server acts as a **proxy**:

1. Fetch the IRI via HTTP GET with `Accept: text/turtle, application/x-turtle`.
2. Parse the response body as Turtle (falling back to RDF/XML).
3. Look up the requested IRI as subject in the parsed graph.
4. If not found, retry with the `http`↔`https` scheme swapped.
5. Return a Gemtext description as per §3.3.

This allows a single client request to traverse the HTTP Linked Open Data web without the client needing HTTP support.

### 3.5 Language Filtering

Servers MAY support a preferred language tag. When set, for each predicate that has language-tagged literal values, only the value matching the preferred language is emitted. Non-language-tagged values and non-literals are always preserved.

### 3.6 Display Mode

Servers MUST support **expanded mode**. Servers MAY support **condensed mode**, enabled by a protocol-specific signal from the client (see §4).

---

## 4. Protocol Bindings

### 4.1 Gemini (Read — mandatory)

- **Transport**: TLS 1.2+, port 1965
- **Request line**: `gemini://<host>[:<port>]/<path>[?<query>]\r\n`
- **Response**: `<status> <meta>\r\n<body>`

**IRI derivation**: the full request URL, with port stripped, is the lookup IRI.

**Condensed mode trigger**: append `?condensed=true` to the request URL.

**Interactive entry point**: a request to `/` with no query string returns a Gemini input prompt (status `10`). The user's input is the IRI to resolve.

**Error response**: status `40` (temporary failure) for validation errors.

**Success response**: status `20`, MIME `text/gemini`.

### 4.2 Titan (Submit — optional)

Titan runs on the same TLS port (1965) as Gemini. A Titan request is distinguished by the `titan://` scheme in the request line.

- **Request line**: `titan://<host>/<path>;size=<n>;\r\n`
- **Payload**: exactly `<n>` bytes; the **first line** of the payload is the IRI to resolve.
- **Response**: same as Gemini (`20 text/gemini` or `40` on error).

Use case: clients that cannot construct arbitrary URL query strings (e.g. terminal editors) can POST a URI as a Titan payload to trigger resolution.

### 4.3 Spartan (Read and Submit — optional)

- **Transport**: plaintext TCP, port 300
- **Request line**: `<host> <path> <length>\r\n`
- **Payload** (if `length > 0`): exactly `length` bytes

**Path semantics**:

| Path | Behaviour |
|---|---|
| `/` | Returns a welcome/instruction page |
| `/submit` | Reads payload; first line is the IRI to resolve |
| `/<anything>` | Treated as the lookup IRI directly |

**Condensed mode trigger**: append `?condensed=true` to the path.

**Response line**: `<status> <meta>\r\n<body>`

| Outcome | Status |
|---|---|
| Success | `2 text/gemini` |
| Temporary failure | `4 <message>` |
| Permanent failure | `5 <message>` |

### 4.4 Nex (Read — optional)

- **Transport**: plaintext TCP, port 1900
- **Request**: a single line `/<percent-encoded-IRI>\n`
- **Response**: raw plaintext body, **no status line**

The path `/` returns a help page with example URIs. The IRI is percent-decoded before lookup. Because Nex has no status header, all outcomes (success, not found, error) are communicated entirely within the body. As noted in §2.3, this implementation returns a Gemtext body, but Nex clients treat only `=>` lines specially.

**Condensed mode**: not supported (client cannot signal preferences within the Nex request format).

### 4.5 NPS (Submit — optional, depends on Nex)

NPS is the write companion to Nex, as Titan is to Gemini.

- **Transport**: plaintext TCP, port 1915
- **Request**: multiple text lines terminated by a line containing only `.`
- **Response**: raw plaintext body, **no status line**

The first non-empty line of the payload is the IRI to resolve. An empty payload returns a welcome message. Multi-line payloads allow editor-friendly submission: the user composes text, appends a terminator line, and pipes to the server.

---

## 5. Optional Feature Summary

| Feature | Gemini | Titan | Spartan | Nex | NPS |
|---|---|---|---|---|---|
| Read (IRI lookup) | **required** | — | **required** | **required** | — |
| Submit (IRI as payload) | — | optional | optional (`/submit`) | — | optional |
| TLS | yes | yes | no | no | no |
| Protocol status codes | yes | yes | yes | no | no |
| Condensed mode | yes | no | yes | no | no |
| Interactive entry point | yes (`10` prompt) | — | yes (root instructions) | yes (root help) | yes (welcome msg) |

---

## 6. Conformance

A **conformant server** MUST:
- Implement at least one protocol binding from §4.
- Serve resource descriptions as Gemtext per §2.3.
- Satisfy the Core Contract (§3): IRI-based lookup, four response outcomes, HTTP proxy for `http(s)://` IRIs.

A **conformant server** SHOULD:
- Implement the Gemini binding (§4.1) as the primary interface.
- Support language filtering (§3.5) for multilingual datasets.

A **conformant server** MAY:
- Implement any combination of Titan, Spartan, Nex, NPS bindings.
- Support condensed mode.

A **conformant client** MUST:
- Accept Gemtext responses and parse them as RDF-in-Gemtext (§2.3).
- Treat `=> <uri> …` link lines as navigable IRI references.

A **conformant client** SHOULD:
- Follow object IRI links by issuing new requests (enabling Linked Data traversal).
- Support round-trip parsing (Gemtext → RDF triples) for data consumers.

---

## 7. Design Rationale

- **Gemtext as universal wire format**: Gemtext is the only format all five protocols can express without friction. It is human-readable, navigable (links are first-class), and round-trip-parseable to RDF. No alternative content negotiation is needed.
- **Proxy transparency**: Clients need no HTTP stack. The server bridges the HTTP Linked Open Data web, so a Gemini browser can dereference a `http://dbpedia.org/…` URI natively.
- **No persistence in write protocols**: Titan, Spartan `/submit`, and NPS currently use their payloads as *query inputs* (the IRI to resolve), not as RDF to ingest. A future extension would define payload semantics for actual triple submission (e.g. Turtle payload → store update).
- **Lowest common denominator**: IRI-in → Gemtext-out is the irreducible contract that every protocol can satisfy, including Nex which has no status headers at all.
