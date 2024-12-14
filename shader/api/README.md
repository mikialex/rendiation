# Rendiation Shader API

Rendiation shader api is an [EDSL](https://en.wikipedia.org/wiki/Domain-specific_language#External_and_Embedded_Domain_Specific_Languages) to express shader logic in plain Rust code.

Writing text-based shaders is a bad practice from the perspective of modern graphics projects:

- Text string editing breaks the development experience.
  - Lack of language service support.
  - No compile-time checking, which leads to an extra coding and testing feedback cycle.
- It is hard to switch, specialize, compose, or extend logic at runtime.
  - Relying on preprocessors, for example, #define, greatly affects readability.
  - Doing string manipulation or templating at runtime is hard to maintain and reason about correctness (like preprocessors, which are just a way to do string manipulation).

To solve these issues, some developers may design and implement their own shader language and IDE support. However, these approaches are not the best practices either.

- The development and maintenance cost is ridiculously huge.
- Creating tremendous understanding costs for new project developers, who have to learn how to use new languages and tools.
- It is hard to interact with the host language and application logic. The development experience is still broken.

The only correct approach is that **the shader should be a programmable data structure composed and processed in the host language at runtime**. After years of exploring and experimenting, I believe this is the best practice for shader programming nowadays and in the future.

- same development experience as the host language
- using host language type system to type-checking shader code
  - and also type-checking resource binding
- build sophisticated abstractions in shader using host language feature

Currently, the shader API is the cornerstone of the Rendiation project. Based on the powerful expressiveness of the API, layers of abstractions can be built upon it. For example, the trait you are familiar with in Rust also has a "device" version:

``` rust
/// Iterator
pub trait ShaderIterator {
  type Item;
  fn shader_next(&self) -> (Node<bool>, Self::Item);
}

/// Future (just for demonstrating, the real one may be different in detail)
pub trait ShaderFuture {
  type Output;
  fn device_poll(&self, ctx: &mut DevicePollCtx) -> ShaderPoll<Self::Output>;
}
```

Even upper layers of high-level systems totally rely on the shader API to make them extendable. For example, the material or lighting system uses high-level shader API-based traits to describe how surface shading is implemented, how lighting is computed, and how ray-medium interaction works.

## Similar projects

- <https://github.com/RayMarch/shame>
- <https://github.com/hadronized/shades>
  - <https://phaazon.net/blog/shades-edsl>
- <https://github.com/LuisaGroup/luisa-compute-rs>
- <https://github.com/leops/rasen>
- <https://github.com/leod/posh>
