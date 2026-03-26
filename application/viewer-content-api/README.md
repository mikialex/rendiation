# viewer content c api

build the lib:

`cargo b -p viewer-content-api`

show dylib exported symbols(macos):

`nm -gU ./target/debug/libviewer_content_api.dylib`

if something not right(the build.rs not print warnings from cbindgen), you can call cbindgen manually:

`cd to current folder`, run `cbindgen --config cbindgen.toml --crate viewer-content-api --output my_header.h` to see what went wrong.
