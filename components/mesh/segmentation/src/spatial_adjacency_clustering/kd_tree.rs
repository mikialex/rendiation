// struct KDNode
// {
// 	union
// 	{
// 		float split;
// 		unsigned int index;
// 	};

// 	// leaves: axis = 3, children = number of extra points after this one (0 if 'index' is the only
// point) 	// branches: axis != 3, left subtree = skip 1, right subtree = skip 1+children
// 	unsigned int axis : 2;
// 	unsigned int children : 30;
// };

// static size_t kdtreePartition(unsigned int* indices, size_t count, const float* points, size_t
// stride, unsigned int axis, float pivot) {
// 	size_t m = 0;

// 	// invariant: elements in range [0, m) are < pivot, elements in range [m, i) are >= pivot
// 	for (size_t i = 0; i < count; ++i)
// 	{
// 		float v = points[indices[i] * stride + axis];

// 		// swap(m, i) unconditionally
// 		unsigned int t = indices[m];
// 		indices[m] = indices[i];
// 		indices[i] = t;

// 		// when v >= pivot, we swap i with m without advancing it, preserving invariants
// 		m += v < pivot;
// 	}

// 	return m;
// }

// static size_t kdtreeBuildLeaf(size_t offset, KDNode* nodes, size_t node_count, unsigned int*
// indices, size_t count) {
// 	assert(offset + count <= node_count);
// 	(void)node_count;

// 	KDNode& result = nodes[offset];

// 	result.index = indices[0];
// 	result.axis = 3;
// 	result.children = unsigned(count - 1);

// 	// all remaining points are stored in nodes immediately following the leaf
// 	for (size_t i = 1; i < count; ++i)
// 	{
// 		KDNode& tail = nodes[offset + i];

// 		tail.index = indices[i];
// 		tail.axis = 3;
// 		tail.children = ~0u >> 2; // bogus value to prevent misuse
// 	}

// 	return offset + count;
// }

// static size_t kdtreeBuild(size_t offset, KDNode* nodes, size_t node_count, const float* points,
// size_t stride, unsigned int* indices, size_t count, size_t leaf_size) {
// 	assert(count > 0);
// 	assert(offset < node_count);

// 	if (count <= leaf_size)
// 		return kdtreeBuildLeaf(offset, nodes, node_count, indices, count);

// 	float mean[3] = {};
// 	float vars[3] = {};
// 	float runc = 1, runs = 1;

// 	// gather statistics on the points in the subtree using Welford's algorithm
// 	for (size_t i = 0; i < count; ++i, runc += 1.f, runs = 1.f / runc)
// 	{
// 		const float* point = points + indices[i] * stride;

// 		for (int k = 0; k < 3; ++k)
// 		{
// 			float delta = point[k] - mean[k];
// 			mean[k] += delta * runs;
// 			vars[k] += delta * (point[k] - mean[k]);
// 		}
// 	}

// 	// split axis is one where the variance is largest
// 	unsigned int axis = vars[0] >= vars[1] && vars[0] >= vars[2] ? 0 : vars[1] >= vars[2] ? 1 : 2;

// 	float split = mean[axis];
// 	size_t middle = kdtreePartition(indices, count, points, stride, axis, split);

// 	// when the partition is degenerate simply consolidate the points into a single node
// 	if (middle <= leaf_size / 2 || middle >= count - leaf_size / 2)
// 		return kdtreeBuildLeaf(offset, nodes, node_count, indices, count);

// 	KDNode& result = nodes[offset];

// 	result.split = split;
// 	result.axis = axis;

// 	// left subtree is right after our node
// 	size_t next_offset = kdtreeBuild(offset + 1, nodes, node_count, points, stride, indices, middle,
// leaf_size);

// 	// distance to the right subtree is represented explicitly
// 	result.children = unsigned(next_offset - offset - 1);

// 	return kdtreeBuild(next_offset, nodes, node_count, points, stride, indices + middle, count -
// middle, leaf_size); }

// static void kdtreeNearest(KDNode* nodes, unsigned int root, const float* points, size_t stride,
// const unsigned char* emitted_flags, const float* position, unsigned int& result, float& limit) {
// 	const KDNode& node = nodes[root];

// 	if (node.axis == 3)
// 	{
// 		// leaf
// 		for (unsigned int i = 0; i <= node.children; ++i)
// 		{
// 			unsigned int index = nodes[root + i].index;

// 			if (emitted_flags[index])
// 				continue;

// 			const float* point = points + index * stride;

// 			float distance2 =
// 			    (point[0] - position[0]) * (point[0] - position[0]) +
// 			    (point[1] - position[1]) * (point[1] - position[1]) +
// 			    (point[2] - position[2]) * (point[2] - position[2]);
// 			float distance = sqrtf(distance2);

// 			if (distance < limit)
// 			{
// 				result = index;
// 				limit = distance;
// 			}
// 		}
// 	}
// 	else
// 	{
// 		// branch; we order recursion to process the node that search position is in first
// 		float delta = position[node.axis] - node.split;
// 		unsigned int first = (delta <= 0) ? 0 : node.children;
// 		unsigned int second = first ^ node.children;

// 		kdtreeNearest(nodes, root + 1 + first, points, stride, emitted_flags, position, result, limit);

// 		// only process the other node if it can have a match based on closest distance so far
// 		if (fabsf(delta) <= limit)
// 			kdtreeNearest(nodes, root + 1 + second, points, stride, emitted_flags, position, result, limit);
// 	}
// }
