# parametric data normalization

Lowers CAD parametric data into one source independent representation, the
`ParametricNormalizedModel`.

| Crate | Content |
|---|---|
| `parametric-normalized-model` | the normalized model (trimmed rational Bézier patches, instances, source info) and its `.pnm` file format |
| `step-reader` | STEP (ISO 10303-21) parsing into an entity table |
| `step-to-bezier` | STEP B-rep to `ParametricNormalizedModel` conversion, see `step-to-bezier/DESIGN.md` and `TODO.md` |
| `parametric-tessellation` | CPU reference of the view dependent adaptive GPU tessellation (plan stage 0) |
| `cli` (`parametric-normalization-cli`) | convert STEP / pnm to pnm / glb (also view dependent tessellation), batch testing, SVG trim plots |
