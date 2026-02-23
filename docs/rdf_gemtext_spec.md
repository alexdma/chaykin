# RDF-to-Gemtext Serialization Specification

**Version**: 0.1 — 2026-02-23
**Produced from**: [gemtext.rs](file:///Users/adamou/workspaces/smolweb/chaykin/server/src/gemtext.rs)

## 1. Overview

This specification defines how an RDF graph, expressed as a set of triples `(subject, predicate, object)`, is serialized into [Gemtext](https://geminiprotocol.net/docs/gemtext-specification.gmi) — the native text format of the Gemini protocol.

Two serialization modes are supported:

| Mode | Description |
|------|-------------|
| **Expanded** | One Gemtext line per triple. Each line encodes both the predicate and the object. |
| **Condensed** | Triples are grouped by predicate. Each predicate becomes a level-2 heading, followed by its object values. |

Two operational contexts exist:

| Context | Heading | Link behavior |
|---------|---------|---------------|
| **Local** | `# Resource: <IRI>` | HTTP/Gemini IRIs rendered as direct `=>` links |
| **Proxy** | `# Proxy: <IRI>` | HTTP IRIs are percent-encoded and routed through a Gemini proxy host |

## 2. URI Shortening (QNames)

Before rendering, all URIs (predicates, IRI objects, datatype URIs) are passed through a **prefix shortening** function. If a URI begins with a registered namespace, the namespace is replaced with its compact prefix.

### 2.1 Registered Prefixes

| Prefix | Namespace IRI |
|--------|---------------|
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` |
| `dc:` | `http://purl.org/dc/elements/1.1/` |
| `dcterms:` | `http://purl.org/dc/terms/` |
| `foaf:` | `http://xmlns.com/foaf/0.1/` |
| `owl:` | `http://www.w3.org/2002/07/owl#` |
| `schema:` | `http://schema.org/` |

### 2.2 Shortening Rule

```
shorten(URI) :=
    if URI starts with a registered namespace N:
        replace N with its prefix → QName
    else:
        URI unchanged
```

> [!NOTE]
> Shortening is applied to display text only. Link targets (`=>` URLs) always use the full IRI or proxy-encoded form.

## 3. RDF Node Types

The serializer recognizes five RDF node types, mapped from the parsed RDF data:

| Node Type | Internal Representation | Example |
|-----------|------------------------|---------|
| **IRI** | `Iri(uri)` | `http://dbpedia.org/resource/Earth` |
| **Blank Node** | `BlankNode(id)` | `_:b0` |
| **Simple Literal** | `SimpleLiteral(value)` | `"71181"` |
| **Language-Tagged Literal** | `LanguageTaggedLiteral(value, lang)` | `"videogioco del 1991"@it` |
| **Datatyped Literal** | `DatatypedLiteral(value, datatype)` | `"1991-01-01"^^xsd:dateTime` |

## 4. Serialization Rules

Let `P` = predicate URI, `shortP` = `shorten(P)`.

### 4.1 Expanded Mode (Local)

Each triple `(subject, P, object)` produces exactly one Gemtext line:

| Object Type | Condition | Output |
|-------------|-----------|--------|
| `Iri(uri)` | URI is HTTP or Gemini | `=> <uri> <shortP> : <shorten(uri)>` |
| `Iri(uri)` | otherwise | `* <shortP>: <uri>` |
| `BlankNode(id)` | — | `* <shortP>: _:<id>` |
| `SimpleLiteral(v)` | — | `* <shortP>: "<v>"` |
| `LanguageTaggedLiteral(v, l)` | — | `* <shortP>: "<v>"@<l>` |
| `DatatypedLiteral(v, dt)` | datatype is HTTP or Gemini | `=> <dt> <shortP> : "<v>"^^<shorten(dt)>` |
| `DatatypedLiteral(v, dt)` | otherwise | `* <shortP>: "<v>"^^<shorten(dt)>` |

### 4.2 Condensed Mode (Local)

Triples are grouped by predicate (sorted lexicographically). Each group produces:

1. A heading: `## <shortP>`
2. If `P` is an HTTP or Gemini URI, a property link: `=> <P> ↗ <shortP>`
3. One line per object value:

| Object Type | Condition | Output |
|-------------|-----------|--------|
| `Iri(uri)` | URI is HTTP or Gemini | `=> <uri> <shorten(uri)>` |
| `Iri(uri)` | otherwise | `* <uri>` |
| `BlankNode(id)` | — | `* _:<id>` |
| `SimpleLiteral(v)` | — | `* "<v>"` |
| `LanguageTaggedLiteral(v, l)` | — | `* "<v>"@<l>` |
| `DatatypedLiteral(v, dt)` | datatype is HTTP or Gemini | `=> <dt> "<v>"^^<shorten(dt)>` |
| `DatatypedLiteral(v, dt)` | otherwise | `* "<v>"^^<shorten(dt)>` |

4. A trailing blank line after each group.

### 4.3 Expanded Mode (Proxy)

Same as §4.1, but HTTP URIs (in objects and datatype positions) are **proxy-encoded**:

```
proxy_url(uri, host) := "gemini://" + host + "/" + percent_encode(uri)
```

| Object Type | Condition | Output |
|-------------|-----------|--------|
| `Iri(uri)` | HTTP | `=> <proxy_url(uri, host)> <shortP> : <shorten(uri)>` |
| `Iri(uri)` | Gemini | `=> <uri> <shortP> : <shorten(uri)>` |
| `DatatypedLiteral(v, dt)` | HTTP datatype | `=> <proxy_url(dt, host)> <shortP> : "<v>"^^<shorten(dt)>` |
| `DatatypedLiteral(v, dt)` | Gemini datatype | `=> <dt> <shortP> : "<v>"^^<shorten(dt)>` |

All other node types behave identically to §4.1.

### 4.4 Condensed Mode (Proxy)

Same as §4.2, but with proxy encoding for HTTP URIs:

- Property link after heading: `=> <proxy_url(P, host)> ↗ <shortP>` (if HTTP) or `=> <P> ↗ <shortP>` (if Gemini)
- Object IRI links: `=> <proxy_url(uri, host)> <shorten(uri)>`
- Datatyped literal links: `=> <proxy_url(dt, host)> "<v>"^^<shorten(dt)>`

All other node types behave identically to §4.2.

## 5. Document Structure

### 5.1 Local Resource Response

```
# Resource: <shorten(subject_iri)>

<expanded_or_condensed_body>

=> gemini://<hostname>/ Home
```

### 5.2 Proxy Response

```
# Proxy: <shorten(original_url)>

<expanded_or_condensed_body>
```

### 5.3 Error Responses

```
# Not Found

Resource not found in graph:
=> <shorten(resource_iri)>
```

```
# No Data Found for <shorten(requested_iri)>

Loaded <N> triples.

## Available Subjects:
* <shorten(subject_1)>
* <shorten(subject_2)>
...
```

## 6. Formal Grammar (EBNF)

The following grammar describes the output of the serializer. Terminals in double quotes are literal strings. `LF` denotes a newline character (`\n`).

```ebnf
(* === Top-level documents === *)

LocalResponse     = "# Resource: " ShortURI LF LF
                    Body
                    LF "=> gemini://" Hostname "/" " Home" LF ;

ProxyResponse     = "# Proxy: " ShortURI LF LF
                    Body ;

NotFoundResponse  = "# Not Found" LF LF
                    "Resource not found in graph:" LF
                    "=> " ShortURI LF ;

DebugResponse     = "# No Data Found for " ShortURI LF LF
                    "Loaded " Integer " triples." LF LF
                    "## Available Subjects:" LF
                    { "* " ShortURI LF } ;

ErrorResponse     = "# " Title LF LF Message LF ;

(* === Body: expanded or condensed === *)

Body              = ExpandedBody | CondensedBody ;

(* --- Expanded mode --- *)

ExpandedBody      = { ExpandedLine } ;

ExpandedLine      = IriLink
                  | DatatypeLink
                  | LiteralBullet ;

IriLink           = "=> " LinkTarget " " ShortPredicate " : " ShortURI LF ;

DatatypeLink      = "=> " LinkTarget " " ShortPredicate ' : "' LexicalForm '"^^' ShortURI LF ;

LiteralBullet     = "* " ShortPredicate ": " LiteralValue LF ;

(* --- Condensed mode --- *)

CondensedBody     = { PredicateGroup } ;

PredicateGroup    = PredicateHeading
                    [ PropertyLink ]
                    { CondensedLine }
                    LF ;

PredicateHeading  = "## " ShortURI LF ;

PropertyLink      = "=> " LinkTarget " ↗ " ShortURI LF ;

CondensedLine     = CondensedIriLink
                  | CondensedDtLink
                  | CondensedBullet ;

CondensedIriLink  = "=> " LinkTarget " " ShortURI LF ;

CondensedDtLink   = "=> " LinkTarget ' "' LexicalForm '"^^' ShortURI LF ;

CondensedBullet   = "* " LiteralValue LF ;

(* === Literal value rendering === *)

LiteralValue      = SimpleLit | LangLit | TypedLit | BlankNodeRef | PlainURI ;

SimpleLit         = '"' LexicalForm '"' ;

LangLit           = '"' LexicalForm '"@' LanguageTag ;

TypedLit          = '"' LexicalForm '"^^' ShortURI ;

BlankNodeRef      = "_:" BlankNodeId ;

PlainURI          = URI ;

(* === Link targets === *)

LinkTarget        = URI | ProxyURI ;

ProxyURI          = "gemini://" Hostname "/" PercentEncodedURI ;

(* === URI shortening === *)

ShortURI          = QName | URI ;

ShortPredicate    = QName | URI ;

QName             = Prefix ":" LocalName ;

Prefix            = "rdf" | "rdfs" | "xsd" | "dc" | "dcterms"
                  | "foaf" | "owl" | "schema" ;

(* === Terminals === *)

URI               = (* any valid IRI *) ;
PercentEncodedURI = (* URI with all non-alphanumeric chars percent-encoded *) ;
Hostname          = (* server hostname, e.g. "localhost" *) ;
LexicalForm       = (* any Unicode string, the literal's value *) ;
LanguageTag       = (* BCP 47 language tag, e.g. "en", "it" *) ;
BlankNodeId       = (* blank node identifier string *) ;
LocalName         = (* local part after prefix, e.g. "dateTime" *) ;
Integer           = (* decimal integer *) ;
Title             = (* error title string *) ;
Message           = (* error message string *) ;
LF                = (* U+000A line feed *) ;
```

## 7. Examples

### 7.1 Expanded Mode (Local)

Given the triple `<http://example.org/Q257469> dcterms:title "videogioco del 1991"@it`:

```gemini
* dcterms:title: "videogioco del 1991"@it
```

Given `<http://example.org/Q257469> schema:datePublished "1991-01-01T00:00:00Z"^^xsd:dateTime`:

```gemini
=> http://www.w3.org/2001/XMLSchema#dateTime schema:datePublished : "1991-01-01T00:00:00Z"^^xsd:dateTime
```

Given `<http://example.org/Q257469> owl:sameAs <http://dbpedia.org/resource/Q257469>`:

```gemini
=> http://dbpedia.org/resource/Q257469 owl:sameAs : http://dbpedia.org/resource/Q257469
```

Given `<http://example.org/Q257469> dcterms:identifier "71181"`:

```gemini
* dcterms:identifier: "71181"
```

### 7.2 Condensed Mode (Local)

```gemini
## dcterms:identifier
=> http://purl.org/dc/terms/identifier ↗ dcterms:identifier
* "71181"

## dcterms:title
=> http://purl.org/dc/terms/title ↗ dcterms:title
* "videogioco del 1991"@it
* "1991 video game"@en

## owl:sameAs
=> http://www.w3.org/2002/07/owl#sameAs ↗ owl:sameAs
=> http://dbpedia.org/resource/Q257469 http://dbpedia.org/resource/Q257469

## schema:datePublished
=> http://schema.org/datePublished ↗ schema:datePublished
=> http://www.w3.org/2001/XMLSchema#dateTime "1991-01-01T00:00:00Z"^^xsd:dateTime
```

### 7.3 Expanded Mode (Proxy)

Same triples as §7.1, with proxy host `example.gemini`:

```gemini
=> gemini://example.gemini/http%3A%2F%2Fdbpedia%2Eorg%2Fresource%2FQ257469 owl:sameAs : http://dbpedia.org/resource/Q257469
=> gemini://example.gemini/http%3A%2F%2Fwww%2Ew3%2Eorg%2F2001%2FXMLSchema%23dateTime schema:datePublished : "1991-01-01T00:00:00Z"^^xsd:dateTime
```

## 8. Design Rationale

### 8.1 Datatyped Literals as Links

Standard N-Triples notation uses `"value"^^<datatype_uri>` which is purely textual. In Gemtext, link lines (`=>`) are the only mechanism for clickable URIs. By emitting datatyped literals as link lines with the datatype IRI as the target, we make the datatype **navigable** — a user can follow the link to the datatype's definition (e.g., the XSD specification page) while still reading the literal value inline.

This is inspired by the W3C [RDF Plain Literal](https://www.w3.org/TR/rdf-plain-literal/) and [RDF Dir Literal](https://w3c.github.io/rdf-dir-literal/) specifications, which treat literal metadata as first-class, addressable concepts.

### 8.2 QName Shortening

Full namespace URIs are verbose and hurt readability in a text-only protocol. QName shortening uses well-known prefixes to substantially reduce visual noise (e.g., `http://www.w3.org/2001/XMLSchema#dateTime` → `xsd:dateTime`). The prefix table is fixed; dynamic `@prefix` declarations from source Turtle are not propagated.

### 8.3 Property Links in Condensed Mode

Gemtext headings (`##`) cannot be links. The `↗` link placed immediately after each heading provides a navigation affordance to the property's definition without breaking the heading's role as a visual grouping label.
