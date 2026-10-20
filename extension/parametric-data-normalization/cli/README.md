# parametric-normalization-cli

Command line front end of the parametric data normalization crates: converts STEP files into
`ParametricNormalizedModel` (`.pnm` files), and writes inspection outputs: glTF (CPU triangulated
trimmed patches) and per face SVG plots of the trim loops.

```sh
# STEP or pnm in, pnm and/or glb out, optional per face SVG plots
cargo run --release -p parametric-normalization-cli -- convert in.stp -o out.pnm -o out.glb --svg svg-dir
cargo run --release -p parametric-normalization-cli -- convert out.pnm -o out.glb

# end to end test over the repo test sets (temp/step-test, step-generated-sample) or given paths:
# per file conversion and validation summary, optional glTF / SVG / pnm round trip check
cargo run --release -p parametric-normalization-cli -- batch --gltf --svg --pnm temp/step-test/steam-controller.stp
```

`batch` writes to `test-output/` in this crate unless `--out DIR` is given.

other (useable) reference online step viewer:

- <https://www.mattkeeter.com/projects/foxtrot/demo/>
- <https://sharecad.org/>
