# Step parameter convert tester

End to end test harness for `rendiation-step-to-bezier`: converts STEP files, prints per file
conversion and validation summaries, and optionally writes glTF (CPU triangulated trimmed
patches) and per face SVG plots of the trim loops.

```sh
cargo run --release -p parametric-step-convert-tester --example batch_test -- --gltf --svg temp/step-test/steam-controller.stp
```

other (useable) reference online step viewer:

- <https://www.mattkeeter.com/projects/foxtrot/demo/>
- <https://sharecad.org/>
