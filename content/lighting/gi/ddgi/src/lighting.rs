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
) -> Node<i32> {
  todo!()
}

fn DDGIGetProbeWorldPosition(
  coords: Node<i32>,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<Vec3<f32>> {
  todo!()
}

/// Computes irradiance for the given world-position using the given volume, surface bias,
/// sampling direction, and volume resources.
fn get_volume_irradiance(
  world_position: Node<Vec3<f32>>,
  surface_bias: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  //     DDGIVolumeDescGPU volume,
  volume: &ShaderPtrOf<ProbeVolumeGPUInfo>,
) -> Node<Vec3<f32>> {
  let irradiance = val(Vec3::<f32>::zero()).make_local_var();
  let accumulatedWeights = val(0.).make_local_var();

  // Bias the world space position
  let biasedWorldPosition = world_position + surface_bias;

  // Get the 3D grid coordinates of the probe nearest the biased world position (i.e. the "base" probe)
  let baseProbeCoords = DDGIGetBaseProbeGridCoords(biasedWorldPosition, volume);

  // Get the world-space position of the base probe (ignore relocation)
  let baseProbeWorldPosition = DDGIGetProbeWorldPosition(baseProbeCoords, volume);

  // Clamp the distance (in grid space) between the given point and the base probe's world position (on each axis) to [0, 1]
  let gridSpaceDistance = biasedWorldPosition - baseProbeWorldPosition;
  //     if(!IsVolumeMovementScrolling(volume)) gridSpaceDistance = RTXGIQuaternionRotate(gridSpaceDistance, RTXGIQuaternionConjugate(volume.rotation));
  let alpha =
    (gridSpaceDistance / volume.spacing().load()).clamp(val(Vec3::zero()), val(Vec3::one()));

  // Iterate over the 8 closest probes and accumulate their contributions
  //     for(int probeIndex = 0; probeIndex < 8; probeIndex++)
  //     {
  //         // Compute the offset to the adjacent probe in grid coordinates by
  //         // sourcing the offsets from the bits of the loop index: x = bit 0, y = bit 1, z = bit 2
  //         int3 adjacentProbeOffset = int3(probeIndex, probeIndex >> 1, probeIndex >> 2) & int3(1, 1, 1);

  //         // Get the 3D grid coordinates of the adjacent probe by adding the offset to
  //         // the base probe and clamping to the grid boundaries
  //         int3 adjacentProbeCoords = clamp(baseProbeCoords + adjacentProbeOffset, int3(0, 0, 0), volume.probeCounts - int3(1, 1, 1));

  //         // Get the adjacent probe's index, adjusting the adjacent probe index for scrolling offsets (if present)
  //         int adjacentProbeIndex = DDGIGetScrollingProbeIndex(adjacentProbeCoords, volume);

  //         // Early Out: don't allow inactive probes to contribute to irradiance
  //         int probeState = DDGILoadProbeState(adjacentProbeIndex, resources.probeData, volume);
  //         if (probeState == RTXGI_DDGI_PROBE_STATE_INACTIVE) continue;

  //         // Get the adjacent probe's world position
  //         float3 adjacentProbeWorldPosition = DDGIGetProbeWorldPosition(adjacentProbeCoords, volume, resources.probeData);

  //         // Compute the distance and direction from the (biased and non-biased) shading point and the adjacent probe
  //         float3 worldPosToAdjProbe = normalize(adjacentProbeWorldPosition - worldPosition);
  //         float3 biasedPosToAdjProbe = normalize(adjacentProbeWorldPosition - biasedWorldPosition);
  //         float  biasedPosToAdjProbeDist = length(adjacentProbeWorldPosition - biasedWorldPosition);

  // // Compute trilinear weights based on the distance to each adjacent probe
  // // to smoothly transition between probes. adjacentProbeOffset is binary, so we're
  // // using a 1-alpha when adjacentProbeOffset = 0 and alpha when adjacentProbeOffset = 1.
  // let trilinear = max(0.001f, lerp(1.f - alpha, alpha, adjacentProbeOffset));
  // let trilinearWeight = (trilinear.x * trilinear.y * trilinear.z);
  // let weight = val(1.).make_local_var();

  //         // A naive soft backface weight would ignore a probe when
  //         // it is behind the surface. That's good for walls, but for
  //         // small details inside of a room, the normals on the details
  //         // might rule out all of the probes that have mutual visibility
  //         // to the point. We instead use a "wrap shading" test. The small
  //         // offset at the end reduces the "going to zero" impact.
  //         float wrapShading = (dot(worldPosToAdjProbe, direction) + 1.f) * 0.5f;
  //         weight *= (wrapShading * wrapShading) + 0.2f;

  //         // Compute the octahedral coordinates of the adjacent probe
  //         float2 octantCoords = DDGIGetOctahedralCoordinates(-biasedPosToAdjProbe);

  //         // Get the texture array coordinates for the octant of the probe
  //         float3 probeTextureUV = DDGIGetProbeUV(adjacentProbeIndex, octantCoords, volume.probeNumDistanceInteriorTexels, volume);

  //         // Sample the probe's distance texture to get the mean distance to nearby surfaces
  //         float2 filteredDistance = 2.f * resources.probeDistance.SampleLevel(resources.bilinearSampler, probeTextureUV, 0).rg;

  //         // Find the variance of the mean distance
  //         float variance = abs((filteredDistance.x * filteredDistance.x) - filteredDistance.y);

  //         // Occlusion test
  //         float chebyshevWeight = 1.f;
  //         if(biasedPosToAdjProbeDist > filteredDistance.x) // occluded
  //         {
  //             // v must be greater than 0, which is guaranteed by the if condition above.
  //             float v = biasedPosToAdjProbeDist - filteredDistance.x;
  //             chebyshevWeight = variance / (variance + (v * v));

  //             // Increase the contrast in the weight
  //             chebyshevWeight = max((chebyshevWeight * chebyshevWeight * chebyshevWeight), 0.f);
  //         }

  //         // Avoid visibility weights ever going all the way to zero because
  //         // when *no* probe has visibility we need a fallback value
  //         weight *= max(0.05f, chebyshevWeight);

  //         // Avoid a weight of zero
  //         weight = max(0.000001f, weight);

  //         // A small amount of light is visible due to logarithmic perception, so
  //         // crush tiny weights but keep the curve continuous
  //         const float crushThreshold = 0.2f;
  //         if (weight < crushThreshold)
  //         {
  //             weight *= (weight * weight) * (1.f / (crushThreshold * crushThreshold));
  //         }

  //         // Apply the trilinear weights
  //         weight *= trilinearWeight;

  //         // Get the octahedral coordinates for the sample direction
  //         octantCoords = DDGIGetOctahedralCoordinates(direction);

  //         // Get the probe's texture coordinates
  //         probeTextureUV = DDGIGetProbeUV(adjacentProbeIndex, octantCoords, volume.probeNumIrradianceInteriorTexels, volume);

  //         // Sample the probe's irradiance
  //         float3 probeIrradiance = resources.probeIrradiance.SampleLevel(resources.bilinearSampler, probeTextureUV, 0).rgb;

  //         // Decode the tone curve, but leave a gamma = 2 curve to approximate sRGB blending
  //         float3 exponent = volume.probeIrradianceEncodingGamma * 0.5f;
  //         probeIrradiance = pow(probeIrradiance, exponent);

  //         // Accumulate the weighted irradiance
  //         irradiance += (weight * probeIrradiance);
  //         accumulatedWeights += weight;
  //     }

  //     if(accumulatedWeights == 0.f) return float3(0.f, 0.f, 0.f);

  //     irradiance *= (1.f / accumulatedWeights);   // Normalize by the accumulated weights
  //     irradiance *= irradiance;                   // Go back to linear irradiance
  //     irradiance *= RTXGI_2PI;                    // Multiply by the area of the integration domain (hemisphere) to complete the Monte Carlo Estimator equation

  //     // Adjust for energy loss due to reduced precision in the R10G10B10A2 irradiance texture format
  //     if (volume.probeIrradianceFormat == RTXGI_DDGI_VOLUME_TEXTURE_FORMAT_U32)
  //     {
  //         irradiance *= 1.0989f;
  //     }

  irradiance.load()
}
