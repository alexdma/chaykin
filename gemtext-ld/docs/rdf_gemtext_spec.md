# RDF-to-Gemtext Serialization Specification

**Version**: 0.2 — 2026-03-21
**Implements**: [gemtext-rdf](file:///Users/adamou/workspaces/smolweb/chaykin/gemtext-rdf/src/lib.rs)

## 1. Overview

This specification defines how an RDF graph, expressed as a set of triples `(subject, predicate, object)`, is serialized into [Gemtext](https://geminiprotocol.net/docs/gemtext-specification.gmi) — the native text format of the Gemini protocol — and how such a Gemtext document is parsed back into RDF triples.

Two serialization modes are supported:

| Mode | Description |
|------|-------------|
| **Expanded** | One Gemtext line per triple. Each line encodes both the predicate and the object. |
| **Condensed** | Triples are grouped by predicate. Each predicate becomes a level-2 heading, followed by its object values. |

An RDF graph may contain triples with **multiple subjects**. Triples are grouped by subject, each group preceded by a level-1 heading identifying the subject IRI.

## 2. URI Shortening and Expansion

### 2.1 Registered Prefixes

All URIs (predicates, IRI objects, datatype URIs) may be shortened using the following well-known prefixes:

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

### 2.2 Shortening Rule (Serialization)

```
shorten(URI) :=
    if URI starts with a registered namespace N:
        replace N with its prefix → QName
    else:
        URI unchanged
```

### 2.3 Expansion Rule (Parsing)

```
expand(URI_or_QName) :=
    if URI_or_QName starts with a registered prefix P:
        replace P with its namespace → full URI
    else:
        URI_or_QName unchanged
```

> [!NOTE]
> Shortening is applied to display text only. Link targets (`=>` URLs) always use the full IRI. The parser uses the link target (full IRI) when available; QNames in display text are expanded back to full URIs using **expand**.

## 3. RDF Node Types

The serializer recognizes five RDF node types:

| Node Type | Internal Representation | Example |
|-----------|------------------------|---------|
| **IRI** | `Iri(uri)` | `http://dbpedia.org/resource/Earth` |
| **Blank Node** | `BlankNode(id)` | `_:b0` |
| **Simple Literal** | `SimpleLiteral(value)` | `"71181"` |
| **Language-Tagged Literal** | `LanguageTaggedLiteral(value, lang)` | `"videogioco del 1991"@it` |
| **Datatyped Literal** | `DatatypedLiteral(value, datatype)` | `"1991-01-01"^^xsd:dateTime` |

## 4. Serialization Rules

Let `P` = predicate URI, `shortP` = `shorten(P)`.

### 4.1 Expanded Mode

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

### 4.2 Condensed Mode

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

### 4.3 Language Filtering

An optional preferred language tag may be provided. When set, for each predicate that has `LanguageTaggedLiteral` values, if the preferred language matches one of them, only that value is kept. Non-language-tagged values and non-literal nodes are always preserved.

## 5. Document Structure

### 5.1 Multi-Subject Document

A document may contain one or more subject groups. Each group begins with a level-1 heading identifying the subject:

```
# Resource: <shorten(subject_1)>

<expanded_or_condensed_body>

# Resource: <shorten(subject_2)>

<expanded_or_condensed_body>
```

The order of subjects follows their first appearance in the input triple set.

## 6. Parsing Rules

The parser reconstructs `(subject, predicate, object)` triples from a Gemtext document.

### 6.1 Mode Detection

The parser **auto-detects** the serialization mode:
- If any line starts with `## `, the document is parsed as **condensed**.
- Otherwise, it is parsed as **expanded**.

### 6.2 Subject Tracking

The current subject is set by `# Resource: <iri>` headings. The IRI is expanded via `expand()`. All subsequent triples belong to this subject until the next subject heading.

Lines before any subject heading are ignored.

### 6.3 Expanded Mode Parsing

| Line pattern | Parsed as |
|--------------|-----------|
| `=> <target> <pred> : "<v>"^^<dt>` | `DatatypedLiteral(v, expand(dt))` with predicate `expand(pred)` |
| `=> <target> <pred> : "<v>"@<l>` | `LanguageTaggedLiteral(v, l)` with predicate `expand(pred)` |
| `=> <target> <pred> : "<v>"` | `SimpleLiteral(v)` with predicate `expand(pred)` |
| `=> <target> <pred> : <display>` | `Iri(target)` with predicate `expand(pred)` |
| `* <pred>: _:<id>` | `BlankNode(id)` with predicate `expand(pred)` |
| `* <pred>: "<v>"^^<dt>` | `DatatypedLiteral(v, expand(dt))` with predicate `expand(pred)` |
| `* <pred>: "<v>"@<l>` | `LanguageTaggedLiteral(v, l)` with predicate `expand(pred)` |
| `* <pred>: "<v>"` | `SimpleLiteral(v)` with predicate `expand(pred)` |
| `* <pred>: <uri>` | `Iri(expand(uri))` with predicate `expand(pred)` |

### 6.4 Condensed Mode Parsing

| Line pattern | Parsed as |
|--------------|-----------|
| `## <pred>` | Sets current predicate to `expand(pred)` |
| `=> <target> ↗ <display>` | Property link — **skipped** (navigational only) |
| `=> <target> "<v>"^^<dt>` | `DatatypedLiteral(v, expand(dt))` |
| `=> <target> "<v>"@<l>` | `LanguageTaggedLiteral(v, l)` |
| `=> <target> "<v>"` | `SimpleLiteral(v)` |
| `=> <target> <display>` | `Iri(target)` |
| `* _:<id>` | `BlankNode(id)` |
| `* "<v>"^^<dt>` | `DatatypedLiteral(v, expand(dt))` |
| `* "<v>"@<l>` | `LanguageTaggedLiteral(v, l)` |
| `* "<v>"` | `SimpleLiteral(v)` |
| `* <uri>` | `Iri(expand(uri))` |

## 7. Formal Grammar (EBNF)

The following grammar describes valid RDF-in-Gemtext documents produced by the serializer and accepted by the parser. Terminals in double quotes are literal strings. `LF` denotes a newline character (`\n`).

```ebnf
(* === Top-level document === *)

Document          = { SubjectGroup } ;

SubjectGroup      = SubjectHeading LF
                    Body ;

SubjectHeading    = "# Resource: " ShortURI LF ;

(* === Body: expanded or condensed === *)

Body              = ExpandedBody | CondensedBody ;

(* --- Expanded mode --- *)

ExpandedBody      = { ExpandedLine } ;

ExpandedLine      = IriLink
                  | DatatypeLink
                  | LiteralBullet ;

IriLink           = "=> " URI " " ShortPredicate " : " ShortURI LF ;

DatatypeLink      = "=> " URI " " ShortPredicate ' : "' LexicalForm '"^^' ShortURI LF ;

LiteralBullet     = "* " ShortPredicate ": " LiteralValue LF ;

(* --- Condensed mode --- *)

CondensedBody     = { PredicateGroup } ;

PredicateGroup    = PredicateHeading
                    [ PropertyLink ]
                    { CondensedLine }
                    LF ;

PredicateHeading  = "## " ShortURI LF ;

PropertyLink      = "=> " URI " ↗ " ShortURI LF ;

CondensedLine     = CondensedIriLink
                  | CondensedDtLink
                  | CondensedBullet ;

CondensedIriLink  = "=> " URI " " ShortURI LF ;

CondensedDtLink   = "=> " URI ' "' LexicalForm '"^^' ShortURI LF ;

CondensedBullet   = "* " LiteralValue LF ;

(* === Literal value rendering === *)

LiteralValue      = SimpleLit | LangLit | TypedLit | BlankNodeRef | PlainURI ;

SimpleLit         = '"' LexicalForm '"' ;

LangLit           = '"' LexicalForm '"@' LanguageTag ;

TypedLit          = '"' LexicalForm '"^^' ShortURI ;

BlankNodeRef      = "_:" BlankNodeId ;

PlainURI          = URI ;

(* === URI shortening === *)

ShortURI          = QName | URI ;

ShortPredicate    = QName | URI ;

QName             = Prefix ":" LocalName ;

Prefix            = "rdf" | "rdfs" | "xsd" | "dc" | "dcterms"
                  | "foaf" | "owl" | "schema" ;

(* === Terminals === *)

URI               = (* any valid IRI *) ;
LexicalForm       = (* any Unicode string, the literal's value *) ;
LanguageTag       = (* BCP 47 language tag, e.g. "en", "it" *) ;
BlankNodeId       = (* blank node identifier string *) ;
LocalName         = (* local part after prefix, e.g. "dateTime" *) ;
LF                = (* U+000A line feed *) ;
```

## 8. Examples

### 8.1 Expanded Mode

Given the following triples for subject `<http://example.org/Q257469>`:

```gemini
# Resource: http://example.org/Q257469

* dcterms:title: "videogioco del 1991"@it
* dcterms:title: "1991 video game"@en
* dcterms:identifier: "71181"
=> http://dbpedia.org/resource/Q257469 owl:sameAs : http://dbpedia.org/resource/Q257469
=> http://www.w3.org/2001/XMLSchema#dateTime schema:datePublished : "1991-01-01T00:00:00Z"^^xsd:dateTime
```

### 8.2 Condensed Mode

Same triples, condensed:

```gemini
# Resource: http://example.org/Q257469

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

### 8.3 Multi-Subject Document

```gemini
# Resource: http://example.org/Alice

* foaf:name: "Alice"
=> http://example.org/Bob foaf:knows : http://example.org/Bob

# Resource: http://example.org/Bob

* foaf:name: "Bob"
```

## 9. Design Rationale

### 9.1 Datatyped Literals as Links

Standard N-Triples notation uses `"value"^^<datatype_uri>` which is purely textual. In Gemtext, link lines (`=>`) are the only mechanism for clickable URIs. By emitting datatyped literals as link lines with the datatype IRI as the target, we make the datatype **navigable** — a user can follow the link to the datatype's definition (e.g., the XSD specification page) while still reading the literal value inline.

This is inspired by the W3C [RDF Plain Literal](https://www.w3.org/TR/rdf-plain-literal/) and [RDF Dir Literal](https://w3c.github.io/rdf-dir-literal/) specifications, which treat literal metadata as first-class, addressable concepts.

### 9.2 QName Shortening

Full namespace URIs are verbose and hurt readability in a text-only protocol. QName shortening uses well-known prefixes to substantially reduce visual noise (e.g., `http://www.w3.org/2001/XMLSchema#dateTime` → `xsd:dateTime`). The prefix table is fixed; dynamic `@prefix` declarations from source Turtle are not propagated.

### 9.3 Property Links in Condensed Mode

Gemtext headings (`##`) cannot be links. The `↗` link placed immediately after each heading provides a navigation affordance to the property's definition without breaking the heading's role as a visual grouping label.

### 9.4 Auto-Detection of Mode

The parser determines the serialization mode heuristically: the presence of `## ` headings indicates condensed mode. This allows a single `parse()` entry point without requiring the caller to specify the mode, and matches the fact that the two modes produce structurally distinct documents.
