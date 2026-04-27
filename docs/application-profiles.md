# Application Profiles — Linked Data over Small Web

**Profiles for**: [Linked Data over Small Web Specification](specification.md)  
**Requirements language**: RFC 2119 (MUST / SHOULD / MAY / MUST NOT)

---

## 1. Inheritance Hierarchy

```
Base Specification (§3 Core Contract)
│
├── Server Profiles
│   ├── ldsmw:GeminiReadServer       (minimal read)
│   │   ├── ldsmw:MultiReadServer    (multi-protocol read)
│   │   │   └── ldsmw:FullServer     (all features)
│   │   └── ldsmw:WriteServer        (adds submission)
│   │       └── ldsmw:FullServer
│
└── Client Profiles
    ├── ldsmw:ConsumerClient         (parse only)
    │   ├── ldsmw:BrowserClient      (navigate)
    │   └── ldsmw:PublisherClient    (submit)
```

A profile that **extends** another inherits all its requirements as MUST.

---

## 2. Server Profiles

### 2.1 `ldsmw:GeminiReadServer` — Gemini Read Server

**Extends**: Base Specification  
**Role**: Server  
**Use case**: Minimal personal capsule serving a local RDF dataset over Gemini.

| Requirement | Obligation |
|---|---|
| Gemini binding (spec §4.1) | MUST |
| Expanded serialization mode (spec §2.3) | MUST |
| Root interactive entry point returning a Gemini `10` input prompt | MUST |
| TLS — server certificate (self-signed permitted) | MUST |
| `Found` / `Not Found` / `Error` / `Debug` response outcomes (spec §3.3) | MUST |
| Condensed serialization mode (spec §2.3) | SHOULD |
| `?condensed=true` query parameter | SHOULD |
| HTTP/HTTPS proxy (spec §3.4) | SHOULD |
| Language filtering (spec §3.5) | MAY |
| Titan binding (spec §4.2) | MAY |
| Client certificate validation | MUST NOT |

---

### 2.2 `ldsmw:MultiReadServer` — Multi-Protocol Read Server

**Extends**: `ldsmw:GeminiReadServer`  
**Role**: Server  
**Use case**: Capsule/station accessible to clients of different Small Web protocols, without write support.

All requirements of `ldsmw:GeminiReadServer`, plus:

| Requirement | Obligation |
|---|---|
| At least one of: Spartan read binding (spec §4.3) or Nex binding (spec §4.4) | MUST |
| HTTP/HTTPS proxy (spec §3.4) | MUST |
| Condensed serialization mode | MUST |
| `Debug` response outcome — list known subjects when IRI not found (spec §3.3) | MUST |
| Spartan read binding (spec §4.3) | SHOULD |
| Nex binding (spec §4.4) | SHOULD |
| Language filtering (spec §3.5) | SHOULD |

---

### 2.3 `ldsmw:WriteServer` — Write-Enabled Server

**Extends**: `ldsmw:GeminiReadServer`  
**Role**: Server  
**Use case**: Server that accepts URI-bearing payloads from write-capable clients (resolution relay; data ingestion is out of scope unless separately specified).

All requirements of `ldsmw:GeminiReadServer`, plus:

| Requirement | Obligation |
|---|---|
| At least one of: Titan (spec §4.2), Spartan `/submit` (spec §4.3), or NPS (spec §4.5) | MUST |
| `40` / `4` error status on missing or malformed payload URI | MUST |
| Welcome / instruction response for empty payload | MUST |
| Accept payload whose first line is the IRI to resolve | MUST |
| Persist submitted triples | MAY (undefined in base spec; reserved for future extension) |

---

### 2.4 `ldsmw:FullServer` — Full Server

**Extends**: `ldsmw:MultiReadServer` AND `ldsmw:WriteServer`  
**Role**: Server  
**Use case**: Complete multi-protocol Linked Data server and LOD proxy.

All requirements of both parent profiles, plus:

| Requirement | Obligation |
|---|---|
| Gemini + Titan bindings | MUST |
| Spartan read + Spartan `/submit` bindings | MUST |
| Nex + NPS bindings | MUST |
| HTTP/HTTPS proxy | MUST |
| Expanded and condensed modes | MUST |
| Language filtering | MUST |
| `Debug` response listing known subjects | MUST |
| `http`↔`https` fallback during proxy resolution (spec §3.4) | MUST |

---

## 3. Client Profiles

### 3.1 `ldsmw:ConsumerClient` — Consumer Client

**Extends**: Base Specification  
**Role**: Client  
**Use case**: Application that reads and processes RDF triples delivered over a Small Web protocol (e.g. a data pipeline, indexer, or reasoner).

| Requirement | Obligation |
|---|---|
| Parse expanded RDF-in-Gemtext into triples (spec §2.3) | MUST |
| Expand QNames using registered prefixes (spec §2.3) | MUST |
| Reconstruct subject IRI from `# Resource:` heading | MUST |
| Auto-detect expanded vs. condensed mode | SHOULD |
| Parse condensed RDF-in-Gemtext into triples | SHOULD |
| Preserve blank node identity within a document | SHOULD |
| Issue requests over at least one protocol binding (spec §4) | MAY |

---

### 3.2 `ldsmw:BrowserClient` — Browser Client

**Extends**: `ldsmw:ConsumerClient`  
**Role**: Client  
**Use case**: Interactive client for navigating Linked Data (e.g. a Gemini browser with RDF awareness, or a dedicated Linked Data navigator).

All requirements of `ldsmw:ConsumerClient`, plus:

| Requirement | Obligation |
|---|---|
| Issue requests over at least one **read** protocol binding (spec §4.1, §4.3, or §4.4) | MUST |
| Follow `=> <uri> …` link lines as IRI references | MUST |
| Recognise and navigate HTTP/HTTPS IRIs by submitting them to the server (triggering spec §3.4) | SHOULD |
| Signal condensed mode preference when the protocol supports it | SHOULD |
| Display predicate QNames in human-readable form | SHOULD |
| Support language preference signalling | MAY |

---

### 3.3 `ldsmw:PublisherClient` — Publisher Client

**Extends**: `ldsmw:ConsumerClient`  
**Role**: Client  
**Use case**: Client that can submit a URI (or, in future, an RDF payload) to a write-capable server for resolution or ingestion.

All requirements of `ldsmw:ConsumerClient`, plus:

| Requirement | Obligation |
|---|---|
| Issue requests over at least one **write** protocol binding (spec §4.2, §4.3 `/submit`, or §4.5) | MUST |
| Send the IRI to resolve as the first line of the payload | MUST |
| Terminate NPS payloads with a line containing only `.` (spec §4.5) | MUST (if NPS used) |
| Include correct `size` or `length` parameter in payload header | MUST (if Titan or Spartan used) |
| Handle `40` / `4` error responses gracefully | MUST |

---

## 4. Profile Matrix

| Profile | Gemini | Titan | Spartan read | Spartan submit | Nex | NPS | HTTP proxy | Condensed | Language filter |
|---|---|---|---|---|---|---|---|---|---|
| `GeminiReadServer` | **M** | O | — | — | — | — | S | S | O |
| `MultiReadServer` | **M** | O | S | — | S | — | **M** | **M** | S |
| `WriteServer` | **M** | O¹ | — | O¹ | — | O¹ | S | S | O |
| `FullServer` | **M** | **M** | **M** | **M** | **M** | **M** | **M** | **M** | **M** |
| `ConsumerClient` | — | — | — | — | — | — | — | S | O |
| `BrowserClient` | O² | — | O² | — | O² | — | S | S | O |
| `PublisherClient` | — | O³ | — | O³ | — | O³ | — | O | O |

**Key**: **M** = MUST, S = SHOULD, O = MAY, — = not applicable  
¹ at least one of Titan, Spartan submit, or NPS MUST be implemented  
² at least one of Gemini, Spartan, or Nex MUST be implemented  
³ at least one of Titan, Spartan submit, or NPS MUST be implemented  

---

## 5. Notes on Future Extensions

The current profiles deliberately leave two areas unspecified, to be addressed in separate extensions:

- **Data submission semantics** — `ldsmw:WriteServer` and `ldsmw:PublisherClient` define *transport* for a payload but do not specify what the server does with it beyond resolution relay. A future `ldsmw:IngestionExtension` would define payload format (e.g. Turtle body), conflict resolution, and authentication.
- **Authentication and access control** — no current profile includes client certificate validation or access-restricted resources. A future `ldsmw:TLSClientAuthExtension` would profile this over Gemini/Titan.
