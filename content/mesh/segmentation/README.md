# Mesh Segmentation

Mesh segmentation is a graph partition problem. In advance way we could use sophisticate graph partition lib like METIS (for example UE5 nanite use it) for better quality and performance, but we do not want to rely on such a dependency to do this. Graph partition is a NP hard problem, in our use case, the quality or the optimal partition is not very important, and the performance could be improve gradually by hand.

This implementation is roughly based on the <https://github.com/zeux/meshoptimizer> v 0.19.  It is pretty simple, just a greedy search solution for graph partition problem.

## related papers introduced by meshoptimizer

Graham Wihlidal. Optimizing the Graphics Pipeline with Compute. 2016

Matthaeus Chajdas. GeometryFX 1.2 - Cluster Culling. 2016

Jack Ritter. An Efficient Bounding Sphere. 1990
