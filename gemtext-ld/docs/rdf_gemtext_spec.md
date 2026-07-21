# RDF-to-Gemtext Serialization Specification

**Version**: 0.3 — 2026-07-21
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

> [!NOTE]
> A **candidate namespace only qualifies if the local part remaining after stripping it contains no unescaped `/`**. Turtle-style prefixed names (`PN_LOCAL`) do not permit a raw `/` in the local part, so shortening against a namespace that would leave one in place produces an invalid QName. Where more than one registered namespace qualifies — e.g. one namespace is itself a prefix of another, such as a hypothetical `.../prop/` alongside `.../prop/direct/` — the **longest (most specific) qualifying namespace** wins. This rule applies identically to the extended prefix table of §2.4.

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

### 2.4 Condensed-Mode Extended Prefixes and the `# Prefixes` Preamble

The registered prefixes of §2.1 are assumed to be known by every conformant client and are never declared explicitly. **Condensed mode** additionally supports a second, smaller table of prefixes for common Linked Data namespaces that are *not* universally registered:

| Prefix | Namespace IRI |
|--------|---------------|
| `wd:` | `http://www.wikidata.org/entity/` |
| `wdp:` | `http://www.wikidata.org/prop/` |
| `wdt:` | `http://www.wikidata.org/prop/direct/` |

Note that `wdp:` and `wdt:` overlap (`prop/direct/` is nested under `prop/`); see the longest-match/slash-safety rule in §2.2 and §2.4.1 for how this is resolved.

Because a client cannot be assumed to know these ahead of time, any of them actually used in a Condensed-mode document **must** be declared in a `# Prefixes` preamble, placed before the first `# Resource:` heading:

```
# Prefixes
* <namespace IRI> <prefix>:
* <namespace IRI> <prefix>:
```

Each line lists one namespace IRI and the prefix (with its trailing `:`) it expands to, space-separated. Prefix declaration lines are not hyperlinks, even when the namespace IRI has an HTTP or Gemini scheme. Only namespaces from the extended table that are actually used in the document are declared — the preamble is omitted entirely when a document only relies on the registered prefixes of §2.1.

#### 2.4.1 Extended Shortening Rule (Serialization, Condensed mode only)

```
shorten_condensed(URI) :=
    if shorten(URI) != URI:
        shorten(URI)                          // registered prefix — never declared
    else if a namespace N in the extended table is the
            longest slash-safe match for URI (§2.2 NOTE):
        replace N with its prefix → QName      // record N for the preamble
    else:
        URI unchanged
```

The longest-match/slash-safety rule of §2.2 matters here in particular: Wikidata's `prop/direct/` namespace (`wdt:`) is itself nested under `prop/` (`wdp:`), so a predicate URI under `prop/direct/` must resolve to `wdt:`, not `wdp:` with a stray `/` left in the local name.

#### 2.4.2 Extended Expansion Rule (Parsing)

A parser reads the `# Prefixes` preamble, if present, into a document-local prefix map before parsing the rest of the document, and expands QNames against it in preference to the registered prefixes:

```
expand_local(QName_or_URI, declared) :=
    if QName_or_URI starts with a prefix P declared locally:
        replace P with declared[P] → full URI
    else:
        expand(QName_or_URI)              // fall back to §2.3
```

A conformant parser is not limited to the extended table of §2.4: any prefix declared in a `# Prefixes` preamble, whatever its origin, must be honoured for the remainder of that document.

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

### 5.0 Optional `# Prefixes` Preamble

When serializing in Condensed mode, if any extended prefix (§2.4) is used anywhere in the document, a `# Prefixes` preamble precedes all subject groups:

```
# Prefixes
* <namespace IRI> <prefix>:

# Resource: <shorten_condensed(subject_1)>

<condensed_body>
```

The preamble is a single block for the whole document — it is not repeated per subject group — and is entirely absent from Expanded mode and from Condensed-mode documents that only use registered prefixes.

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

### 6.0 Preamble Extraction

Before mode detection, the parser checks whether the document opens with a `# Prefixes` heading (skipping only leading blank lines). If so, it consumes the block of `* <namespace IRI> <prefix>:` lines that follows, up to the next blank line, building the document-local prefix map used by `expand_local()` (§2.4.2). The remainder of the document is then parsed as usual. A document with no such heading yields an empty local map, and `expand_local` behaves exactly like `expand`.

### 6.1 Mode Detection

The parser **auto-detects** the serialization mode from the lines that follow the preamble (if any):
- If any line starts with `## `, the document is parsed as **condensed**.
- Otherwise, it is parsed as **expanded**.

### 6.2 Subject Tracking

The current subject is set by `# Resource: <iri>` headings. The IRI is expanded via `expand_local()`. All subsequent triples belong to this subject until the next subject heading.

Lines before any subject heading (other than a leading `# Prefixes` preamble) are ignored.

### 6.3 Expanded Mode Parsing

> Every `expand(...)` below denotes `expand_local(...)` (§2.4.2, §6.0): registered prefixes plus whatever `# Prefixes` declared for this document.

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

Document          = [ PrefixPreamble ] { SubjectGroup } ;

(* --- Optional preamble (Condensed mode, only when an extended
       prefix per §2.4 is used) --- *)

PrefixPreamble    = "# Prefixes" LF
                    { PrefixDecl }
                    LF ;

PrefixDecl        = "* " URI " " DeclaredPrefix ":" LF ;

DeclaredPrefix    = (* any local prefix name declared by this document *) ;

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

Prefix            = RegisteredPrefix | DeclaredPrefix ;

(* Assumed known by every client; never appear in a PrefixDecl. *)
RegisteredPrefix  = "rdf" | "rdfs" | "xsd" | "dc" | "dcterms"
                  | "foaf" | "owl" | "schema" ;

(* Only valid within a document that declares them via PrefixPreamble
   (Condensed mode only, §2.4). *)

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

### 8.4 Condensed Mode with a `# Prefixes` Preamble

Wikidata's `entity/` and `prop/direct/` namespaces (§2.4) are not registered prefixes, so a Condensed-mode document using them must declare them up front. The example below states that entity Q257469 (video game *Another World*) is an instance (`wdt:P31`) of entity Q7889 (video game):

```gemini
# Prefixes
* http://www.wikidata.org/entity/ wd:
* http://www.wikidata.org/prop/direct/ wdt:

# Resource: wd:Q257469

## wdt:P31
=> http://www.wikidata.org/prop/direct/P31 ↗ wdt:P31
=> http://www.wikidata.org/entity/Q7889 wd:Q7889
```

Had the document only used registered prefixes (§2.1), the `# Prefixes` block would be omitted entirely, as in §8.2.

## 9. Design Rationale

### 9.1 Datatyped Literals as Links

Standard N-Triples notation uses `"value"^^<datatype_uri>` which is purely textual. In Gemtext, link lines (`=>`) are the only mechanism for clickable URIs. By emitting datatyped literals as link lines with the datatype IRI as the target, we make the datatype **navigable** — a user can follow the link to the datatype's definition (e.g., the XSD specification page) while still reading the literal value inline.

This is inspired by the W3C [RDF Plain Literal](https://www.w3.org/TR/rdf-plain-literal/) and [RDF Dir Literal](https://w3c.github.io/rdf-dir-literal/) specifications, which treat literal metadata as first-class, addressable concepts.

### 9.2 QName Shortening

Full namespace URIs are verbose and hurt readability in a text-only protocol. QName shortening uses well-known prefixes to substantially reduce visual noise (e.g., `http://www.w3.org/2001/XMLSchema#dateTime` → `xsd:dateTime`). The registered prefix table (§2.1) is fixed and never declared in-document, since every conformant client is assumed to know it already.

Condensed mode relaxes this for a small, separate table of extended prefixes (§2.4) covering common Linked Data namespaces — e.g. Wikidata's `wd:`/`wdt:` — that are frequent enough to be worth shortening but not universal enough to bake into every client. Because a parser cannot be expected to know these ahead of time, any of them a document actually uses must be declared in a `# Prefixes` preamble, restoring losslessness: the mapping travels with the data instead of being assumed. This still falls short of general dynamic `@prefix` propagation from arbitrary source Turtle — only the curated extended table can be serialized this way — but it is enough to keep frequently-traversed datasets (e.g. Wikidata-backed capsules) readable in Condensed mode without inventing per-document prefixes on the fly.

### 9.3 Property Links in Condensed Mode

Gemtext headings (`##`) cannot be links. The `↗` link placed immediately after each heading provides a navigation affordance to the property's definition without breaking the heading's role as a visual grouping label.

### 9.4 Auto-Detection of Mode

The parser determines the serialization mode heuristically: the presence of `## ` headings indicates condensed mode. This allows a single `parse()` entry point without requiring the caller to specify the mode, and matches the fact that the two modes produce structurally distinct documents.
