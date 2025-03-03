use crate::*;

/// Computes a weight value in the range [0, 1] for a world position and volume pair.
/// All positions inside the given volume receive a weight of 1.
/// Positions outside the volume receive a weight in [0, 1] that
/// decreases as the position moves away from the volume.
///
/// This is to improve the visual appearance of the edge of the volume.
pub fn get_volume_blend_weight(
  position_world: Node<Vec3<f32>>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<f32> {
  // Get the volume's origin and extent
  let spacing = volume.spacing().load();
  let counts = volume.counts().load();
  let origin = volume.origin().load() + (volume.scroll_offsets().load().into_f32() * spacing);
  let extent = spacing * (counts - val(Vec3::splat(1))).into_f32() * val(Vec3::splat(0.5));

  //     // Get the delta between the (rotated volume) and the world-space position
  //     float3 position = (worldPosition - origin);
  //     position = abs(RTXGIQuaternionRotate(position, RTXGIQuaternionConjugate(volume.rotation)));

  let position = position_world - origin;
  let delta = position - extent;

  let volume_blend_weight = val(1.0).make_local_var();

  let in_volume = delta
    .x()
    .less_than(0.0)
    .and(delta.y().less_than(0.0))
    .and(delta.z().less_than(0.0));

  if_by(in_volume.not(), || {
    volume_blend_weight.store(
      (val(1.) - (delta.x() / spacing.x()).saturate())
        * (val(1.) - (delta.y() / spacing.y()).saturate())
        * (val(1.) - (delta.z() / spacing.z()).saturate()),
    )
  });

  volume_blend_weight.load()
}

fn DDGIGetBaseProbeGridCoords(
  world_position: Node<Vec3<f32>>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<Vec3<i32>> {
  todo!()
}

fn DDGIGetProbeWorldPosition(
  coords: Node<Vec3<i32>>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<Vec3<f32>> {
  todo!()
}

fn DDGIGetProbeWorldPositionWithProbeRelocationOffset(
  coords: Node<Vec3<i32>>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
  relocation_offsets: ShaderPtrOf<[Vec3<f32>]>,
) -> Node<Vec3<f32>> {
  todo!()
}

struct ProbeVolumeDataInvocation {
  sampler: BindingNode<ShaderSampler>,
  irradiance: BindingNode<ShaderTexture2DArray>, // srgb
  distance: BindingNode<ShaderTexture2DArray>,
  data: BindingNode<ShaderTexture2DArray>,
}

/// Computes the normalized texture UVs within the Probe Irradiance and Probe Distance texture arrays
/// given the probe index and 2D normalized octant coordinates [-1, 1]. Used when sampling the texture arrays.
///
/// When infinite scrolling is enabled, probeIndex is expected to be the scroll adjusted probe index.
/// Obtain the adjusted index with DDGIGetScrollingProbeIndex().
fn DDGIGetProbeUV(
  probeIndex: Node<u32>,
  octantCoordinates: Node<Vec2<f32>>,
  numProbeInteriorTexels: Node<u32>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<Vec3<f32>> {
  //     // Get the probe's texel coordinates, assuming one texel per probe
  //     uint3 coords = DDGIGetProbeTexelCoords(probeIndex, volume);

  //     // Add the border texels to get the total texels per probe
  //     float numProbeTexels = (numProbeInteriorTexels + 2.f);

  // #if RTXGI_COORDINATE_SYSTEM == RTXGI_COORDINATE_SYSTEM_LEFT || RTXGI_COORDINATE_SYSTEM == RTXGI_COORDINATE_SYSTEM_RIGHT
  //     float textureWidth = numProbeTexels * volume.probeCounts.x;
  //     float textureHeight = numProbeTexels * volume.probeCounts.z;
  // #elif RTXGI_COORDINATE_SYSTEM == RTXGI_COORDINATE_SYSTEM_LEFT_Z_UP
  //     float textureWidth = numProbeTexels * volume.probeCounts.y;
  //     float textureHeight = numProbeTexels * volume.probeCounts.x;
  // #elif RTXGI_COORDINATE_SYSTEM == RTXGI_COORDINATE_SYSTEM_RIGHT_Z_UP
  //     float textureWidth = numProbeTexels * volume.probeCounts.x;
  //     float textureHeight = numProbeTexels * volume.probeCounts.y;
  // #endif

  //     // Move to the center of the probe and move to the octant texel before normalizing
  //     float2 uv = float2(coords.x * numProbeTexels, coords.y * numProbeTexels) + (numProbeTexels * 0.5f);
  //     uv += octantCoordinates.xy * ((float)numProbeInteriorTexels * 0.5f);
  //     uv /= float2(textureWidth, textureHeight);
  //     return float3(uv, coords.z);
  todo!()
}

/// Computes irradiance for the given world-position using the given volume, surface bias,
/// sampling direction, and volume resources.
fn get_volume_irradiance(
  world_position: Node<Vec3<f32>>,
  surface_bias: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  volume_metadata: &ShaderPtrOf<ProbeVolumeGPUInfo>,
  relocation_offsets: ShaderPtrOf<[Vec3<f32>]>,
  data_invocation: ProbeVolumeDataInvocation,
) -> Node<Vec3<f32>> {
  let irradiance = val(Vec3::<f32>::zero()).make_local_var();
  let accumulatedWeights = val(0.).make_local_var();

  // Bias the world space position
  let biasedWorldPosition = world_position + surface_bias;

  // Get the 3D grid coordinates of the probe nearest the biased world position (i.e. the "base" probe)
  let baseProbeCoords = DDGIGetBaseProbeGridCoords(biasedWorldPosition, volume_metadata);

  // Get the world-space position of the base probe (ignore relocation)
  let baseProbeWorldPosition = DDGIGetProbeWorldPosition(baseProbeCoords, volume_metadata);

  // Clamp the distance (in grid space) between the given point and the base probe's world position (on each axis) to [0, 1]
  let gridSpaceDistance = biasedWorldPosition - baseProbeWorldPosition;
  //     if(!IsVolumeMovementScrolling(volume)) gridSpaceDistance = RTXGIQuaternionRotate(gridSpaceDistance, RTXGIQuaternionConjugate(volume.rotation));
  let alpha = (gridSpaceDistance / volume_metadata.spacing().load())
    .clamp(val(Vec3::zero()), val(Vec3::one()));

  // Iterate over the 8 closest probes and accumulate their contributions
  val(8_i32).into_shader_iter().for_each(|probeIndex, cx| {
    // Compute the offset to the adjacent probe in grid coordinates by
    // sourcing the offsets from the bits of the loop index: x = bit 0, y = bit 1, z = bit 2
    let adjacentProbeOffset_: Node<Vec3<i32>> =
      (probeIndex, probeIndex >> val(1), probeIndex >> val(2)).into();
    let adjacentProbeOffset = adjacentProbeOffset_ & val(Vec3::splat(1_i32));

    // Get the 3D grid coordinates of the adjacent probe by adding the offset to
    // the base probe and clamping to the grid boundaries
    let adjacentProbeCoords = adjacentProbeOffset.clamp(
      val(Vec3::splat(0_i32)),
      volume_metadata.counts().load().into_i32() - val(Vec3::splat(1_i32)),
    );

    // Get the adjacent probe's index, adjusting the adjacent probe index for scrolling offsets (if present)
    let adjacentProbeIndex = DDGIGetScrollingProbeIndex(adjacentProbeCoords, volume_metadata);

    //         // Early Out: don't allow inactive probes to contribute to irradiance
    //         int probeState = DDGILoadProbeState(adjacentProbeIndex, resources.probeData, volume);
    //         if (probeState == RTXGI_DDGI_PROBE_STATE_INACTIVE) continue;

    // Get the adjacent probe's world position
    let adjacentProbeWorldPosition = DDGIGetProbeWorldPositionWithProbeRelocationOffset(
      adjacentProbeCoords,
      volume_metadata,
      relocation_offsets,
    );

    // Compute the distance and direction from the (biased and non-biased) shading point and the adjacent probe
    let worldPosToAdjProbe = (adjacentProbeWorldPosition - world_position).normalize();
    let biasedPosToAdjProbe = (adjacentProbeWorldPosition - biasedWorldPosition).normalize();
    let biasedPosToAdjProbeDist = (adjacentProbeWorldPosition - biasedWorldPosition).length();

    // Compute trilinear weights based on the distance to each adjacent probe
    // to smoothly transition between probes. adjacentProbeOffset is binary, so we're
    // using a 1-alpha when adjacentProbeOffset = 0 and alpha when adjacentProbeOffset = 1.
    let trilinear = (val(Vec3::splat(1.0)) - alpha).mix(alpha, adjacentProbeOffset.into_f32());
    let trilinearWeight = (trilinear.x() * trilinear.y() * trilinear.z());
    let weight = val(1.);

    // A naive soft backface weight would ignore a probe when
    // it is behind the surface. That's good for walls, but for
    // small details inside of a room, the normals on the details
    // might rule out all of the probes that have mutual visibility
    // to the point. We instead use a "wrap shading" test. The small
    // offset at the end reduces the "going to zero" impact.
    let wrapShading = (worldPosToAdjProbe.dot(direction) + val(1.)) * val(0.5);
    let weight = weight * ((wrapShading * wrapShading) + val(0.2));

    // Compute the octahedral coordinates of the adjacent probe
    let octantCoords = direction_to_octahedral_coordinate(-biasedPosToAdjProbe);

    // Get the texture array coordinates for the octant of the probe
    let probeTextureUV: Node<Vec3<f32>> = DDGIGetProbeUV(
      adjacentProbeIndex.into_u32(),
      octantCoords,
      volume_metadata.numDistanceInteriorTexels().load(),
      volume_metadata,
    );

    // // Sample the probe's distance texture to get the mean distance to nearby surfaces
    // let filteredDistance: Node<Vec2<f32>> = val(Vec2::splat(2.0))
    //   * resources
    //     .probeDistance
    //     .SampleLevel(resources.bilinearSampler, probeTextureUV, 0)
    //     .rg;

    // // Find the variance of the mean distance
    // let variance = abs((filteredDistance.x * filteredDistance.x) - filteredDistance.y);

    // // Occlusion test
    // let chebyshevWeight = val(1.);
    // if(biasedPosToAdjProbeDist > filteredDistance.x()) // occluded
    // {
    //     // v must be greater than 0, which is guaranteed by the if condition above.
    //     float v = biasedPosToAdjProbeDist - filteredDistance.x;
    //     chebyshevWeight = variance / (variance + (v * v));

    //     // Increase the contrast in the weight
    //     chebyshevWeight = max((chebyshevWeight * chebyshevWeight * chebyshevWeight), 0.f);
    // }

    // // Avoid visibility weights ever going all the way to zero because
    // // when *no* probe has visibility we need a fallback value
    // let weight = weight * chebyshevWeight.max(0.05);

    // Avoid a weight of zero
    let weight = weight.max(0.000001);

    // A small amount of light is visible due to logarithmic perception, so
    // crush tiny weights but keep the curve continuous
    let crushThreshold = 0.2;
    if_by(weight.less_than(crushThreshold), || {
      // weight *= (weight * weight) * (1.f / (crushThreshold * crushThreshold));
    });

    // Apply the trilinear weights
    let weight = weight * trilinearWeight;

    // Get the octahedral coordinates for the sample direction
    let octantCoords = direction_to_octahedral_coordinate(direction);

    // Get the probe's texture coordinates
    // probeTextureUV = DDGIGetProbeUV(
    //   adjacentProbeIndex,
    //   octantCoords,
    //   volume.probeNumIrradianceInteriorTexels,
    //   volume,
    // );

    // // Sample the probe's irradiance
    // let probeIrradiance = resources
    //   .probeIrradiance
    //   .SampleLevel(resources.bilinearSampler, probeTextureUV, 0)
    //   .rgb;

    // // Decode the tone curve, but leave a gamma = 2 curve to approximate sRGB blending
    // let exponent = volume.probeIrradianceEncodingGamma * 0.5f;
    // probeIrradiance = pow(probeIrradiance, exponent);

    // Accumulate the weighted irradiance
    // irradiance.store(irradiance.load() + (weight * probeIrradiance));
    accumulatedWeights.store(accumulatedWeights.load() + weight);
  });

  // check to avoid div by 0
  if_by(accumulatedWeights.load().equals(0.), || {
    irradiance.store(val(Vec3::zero()));
  })
  .else_by(|| {
    let irr = irradiance.load() * (val(1.) / accumulatedWeights.load()); // Normalize by the accumulated weights
    let irr = irr * irr; // Go back to linear irradiance
    let irr = irr * val(f32::PI() * 2.0); // Multiply by the area of the integration domain (hemisphere) to complete the Monte Carlo Estimator equation

    //     // Adjust for energy loss due to reduced precision in the R10G10B10A2 irradiance texture format
    //     if (volume.probeIrradianceFormat == RTXGI_DDGI_VOLUME_TEXTURE_FORMAT_U32)
    //     {
    //         irradiance *= 1.0989f;
    //     }
    irradiance.store(irr);
  });

  irradiance.load()
}

/// Adjusts the probe index for when infinite scrolling is enabled.
/// This can run when scrolling is disabled since zero offsets result
/// in the same probe index.
fn DDGIGetScrollingProbeIndex(
  probeCoords: Node<Vec3<i32>>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<i32> {
  todo!()
  // return DDGIGetProbeIndex(
  //   ((probeCoords + volume.scroll_offsets + volume.counts.into_i32()) % volume.counts.into_i32()),
  //   volume,
  // );
}

/// Computes the probe index from 3D grid coordinates.
/// The opposite of DDGIGetProbeCoords(probeIndex,...).
fn DDGIGetProbeIndex(
  probeCoords: Node<Vec3<i32>>,
  volume: &ENode<ProbeVolumeGPUInfo>,
) -> Node<i32> {
  todo!()
  // int probesPerPlane = DDGIGetProbesPerPlane(volume.probeCounts);
  // int planeIndex = DDGIGetPlaneIndex(probeCoords);
  // int probeIndexInPlane = DDGIGetProbeIndexInPlane(probeCoords, volume.probeCounts);

  // return (planeIndex * probesPerPlane) + probeIndexInPlane;
}

/// Computes the probe index from 3D (Texture2DArray) texture coordinates.
fn DDGIGetProbeIndex_(
  probeCoords: Node<Vec3<i32>>,
  probeNumTexels: Node<i32>,
  volume: &ENode<ProbeVolumeGPUInfo>,
) -> Node<i32> {
  todo!()
  // int probesPerPlane = DDGIGetProbesPerPlane(volume.probeCounts);
  // int probeIndexInPlane = DDGIGetProbeIndexInPlane(texCoords, volume.probeCounts, probeNumTexels);

  // return (texCoords.z * probesPerPlane) + probeIndexInPlane;
}

// /**
//  * Clears probe irradiance and distance data for a plane of probes that have been scrolled to new positions.
//  */
// bool DDGIClearScrolledPlane(int3 probeCoords, int planeIndex, DDGIVolumeDescGPU volume)
// {
//     if (volume.probeScrollClear[planeIndex])
//     {
//         int offset = volume.probeScrollOffsets[planeIndex];
//         int probeCount = volume.probeCounts[planeIndex];
//         int direction = volume.probeScrollDirections[planeIndex];

//         int coord = 0;
//         if(direction) coord = (probeCount + (offset - 1)) % probeCount; // scrolling in positive direction
//         else coord = (probeCount + (offset % probeCount)) % probeCount; // scrolling in negative direction

//         // Probe has scrolled and needs to be cleared
//         if (probeCoords[planeIndex] == coord) return true;
//     }
//     return false;
// }
