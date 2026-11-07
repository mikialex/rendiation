# viewer content c api

build the lib (the build script generates `bindings.h`):

`cargo b -p viewer-content-api`

generate `rdb.h` (the typed database access header, it includes `bindings.h`):

`cargo run -p viewer-content-api --example generate_rdb_header`

both headers are generated files and not tracked by git.

show dylib exported symbols(macos):

`nm -gU ./target/debug/libviewer_content_api.dylib`

if something not right(the build.rs not print warnings from cbindgen), you can call cbindgen manually:

cd to viewer-content-api folder, run `cbindgen --config cbindgen.toml --crate viewer-content-api --output binding.h` to see what went wrong.

## api scope

`bindings.h` only contains the viewer level api (viewer, surface, rendering, picking, derived world data query). All the scene data (scene, node, camera, light, mesh, material, model, texture...) is created and modified through `rdb.h`.

## typed database access (rdb.h)

`rdb.h` is generated from the registered database schema by reflection (see `utility/database-ffi`), it exposes every database entity and component on top of the type erased `rdb_*` entry points in `bindings.h`:

```cpp
auto light = rdb::create<rdb::entity::DirectionalLightEntity>();
rdb::write<rdb::component::DirectionalLightIlluminance>(light, {1.f, 1.f, 1.f});

// the value contains heap data (Str/Slice), the pointers are valid until the snapshot is dropped
rdb::Snapshot snapshot;
rdb::component::SceneNodeEntity_Label::Value label{};
rdb::read<rdb::component::SceneNodeEntity_Label>(node, label, snapshot);
```

the header must be regenerated when the database schema changes, `rdb::check_schema()` returns false if the header does not match the library.
