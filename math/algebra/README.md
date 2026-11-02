# Rendiation's algebra math lib

Just a simple math lib. contains primitives like vec2, vec3, mat3, mat4.

Why not use the other matured math lib:

- light weight and easy to maintain and understand
- used in other rendiation infra for example shader-api

## Content

- vectors: `Vec2`, `Vec3`, `Vec4`, swizzles, `NormalizedVector` (normalized by construction)
- square matrices: `Mat2`, `Mat3`, `Mat4`
- non square matrices named as WGSL `MatCxR` (C columns, R rows): `Mat2x3`, `Mat2x4`, `Mat3x2`,
  `Mat3x4`, `Mat4x2`, `Mat4x3` (also used as the compact affine transform)
- `Quat` for rotation, `Lerp` / `Slerp`
- `PerspectiveProjection`, `OrthographicProjection` and the `NDCSpaceMapper` to target
  different NDC (OpenGL, WebGPU)
- `Deg` angle wrapper, the shader layout helper types like `Bool`, `Shader16PaddedMat3`

## Conventions

- matrices are column major, the fields `a*` `b*` `c*` `d*` are the columns, same memory layout as
  WGSL and glam
- `Mat*::new` takes the arguments column by column, so the source code looks transposed
- column vector, `a * b * v` applies `b` first
- right handed rotation, angles are in radians unless wrapped by `Deg`
- the projection matrices map the view space (looking toward -z) into the OpenGL NDC first, then
  into the target NDC by the `NDCSpaceMapper`

## Test

The implementation is cross checked against glam (dev dependency only) in `tests/algebra`:

```sh
cargo test -p rendiation-algebra
```
