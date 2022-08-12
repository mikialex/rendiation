# Scene core

* A universal scene representation
* Not coupled to specific rendering implementation
    * The rendering or other processing are implemented in other crates
    * We trade some performance for better extensibility
* Support IO with other common file formats
    * not mapping exact format like gltf
    * keep the conversion as lossless as possible
    * support partial load for example load only a mesh