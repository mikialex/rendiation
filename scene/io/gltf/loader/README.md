## Implementation Notes

the current gltf loader does not preserve the lossless gltf entities relationship:

* some entities and properties are not supported yet
* some shareable entities could be cloned
* but the largest data part will be correctly shared

lossless means if your load gltf and export, the file content should not change.

## links

https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html