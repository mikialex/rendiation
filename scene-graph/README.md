# SceneGraph

Provide a 3d scene hierarchy structure for general usage. Support any backend that impl SceneGraphRenderBackend. Backend resource type and rendering specialized impl will be inject by generics trait implementation.

Provide a backend agnostic layer for scene content(geometry/material) create and update. By using this layer, you can switch any backend that impl the CAL backend.

Step One, If you have a backend API, want use it with a scene structure, just impl SceneGraphRenderBackend;
Step Two, If you want CAL support your backend, just impl CALBackend as translation layer;

Why two layers? People who use cal may not need scene(maybe will move to other crate in future); People who use scene may not want cross backend ability(or not possible)

```text
                       +---------------------------------------------+
                       |                                             |
                       |       content abstraction layer             |
                       |                                             |
                 +----+|       backend switch                        |
                 |     +----           ------------------------------+
                 |         +-----+-----+
                 |               v
                 |     +---+-----+-----+-----------------------------+
                 |     |   |           |                             |
                 |     |   |  resource |            scene            |
                 |     |   |    type   |                             |
                 |     |   |           |    +---------------------+  |
                 |     |   |   inject  |    |                     |  |
                 |     +---|           |----|   rendering impl    |--+
                 |     +---+-----------+----+---------------------+--+
                 |     |                                             |
                 +---->|                   backend                   |
                       |                                             |
                       +---------------------------------------------+
```

As planed, webgpu / webgl will impl SceneGraphRenderBackend and CALBackend;
