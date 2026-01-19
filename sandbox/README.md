# LDOG - Linked.Data.Over.Gemini

This project has a twofold goal:

1. To explore the potential for the [Gemini](https://gemini.circumlunar.space/) "small web" protocol to publish and manage [Linked Data](https://www.w3.org/wiki/LinkedData).
2. For me to start studying the Rust language :)

Particularly, I want to explore:

* A convenient way of rendering Linked Data as Gemtext, or annotationg any Gemtext as you would RDFa or Microdata.
* Using `gemini://` URIs for identifying things, and their relation to `http://` URI.
* Bridging between the two protocols: should dereferencing `gemini://` deliver RDF formatted using `gemini://` URIs or make the leap to `http://`?
* Gemini protocol bindings for Linked Data platforms, knowing there are no POST, PUT etc. request types and implementing a REST interface can be a daunting task.

