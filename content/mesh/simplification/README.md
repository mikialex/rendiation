# Mesh Simplification

This implementation is roughly based on the <https://github.com/zeux/meshoptimizer> v 0.19, with the reference rust port of <https://github.com/yzsolt/meshopt-rs>.

My original algorithm reading notes is here: <https://mikialex.github.io/2020/04/04/mesh-simplification/>

## Bin Examples

Run `cargo r --package rendiation-mesh-simplification --example simplification --release`

## Paper references introduce by meshoptimizer

Michael Garland and Paul S. Heckbert. Surface simplification using quadric error metrics. 1997

Michael Garland. Quadric-based polygonal surface simplification. 1999

Peter Lindstrom. Out-of-Core Simplification of Large Polygonal Models. 2000

Matthias Teschner, Bruno Heidelberger, Matthias Mueller, Danat Pomeranets, Markus Gross. Optimized Spatial Hashing for Collision Detection of Deformable Objects. 2003

Peter Van Sandt, Yannis Chronis, Jignesh M. Patel. Efficiently Searching In-Memory Sorted Arrays: Revenge of the Interpolation Search? 2019

Hugues Hoppe. New Quadric Metric for Simplifying Meshes with Appearance Attributes. 1999
