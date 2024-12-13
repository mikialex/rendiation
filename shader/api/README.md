# Rendiation Shader API

Rendiation shader api is an EDSL to express shader logic in plain rust code.

Writing text based shader is a bad practice in perspective of modern graphics project:

- text string breaks development experience
  - lack of language service support
  - no compile time checking => extra coding testing feedback cycle
- hard to switch/specialize/compose/extend logic in runtime
  - relying on preprocessor for example #define, greatly affect the readability.
  - or doing string manipulation or templating in runtime, hard to maintain and reason correctness(like preprocessor is just a way to do string manipulation)

To solve these issues some developers may design and implement their own shader language and IDE support. However, these approaches are not the best practice as well.

- the development and maintain cost is ridiculously huge.
- create tremendous understanding cost for new project developer. they have to learn how to use new language and tools.
- hard to interact with the host language and application logic. the development experience is still broken.

The only correct approach is that the shader should be a programmable data structure defined and processed in host language at runtime. After years and years exploring and experimenting, I believe this is the best practice for shader programming in nowadays and future.

- same development experience as the host language
- using host language type system to type-checking shader code
- build sophisticated abstractions in shader using host language feature
