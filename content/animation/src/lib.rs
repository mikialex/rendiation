use std::ops::{Add, AddAssign, Div, Mul, Sub};

use rendiation_algebra::*;

/// https://www.youtube.com/watch?v=KPoeNZZ6H4s

#[derive(Clone, Copy, Debug)]
pub struct SpringConfig<T = f32> {
  /// `f`, the natural frequency
  ///
  /// Corresponds to:
  /// - The speed at which the system responds to changes in input
  /// - The frequency at which the system will vibrate
  pub frequency: T,
  /// `Zeta`, the damping coefficient
  ///
  /// Describes how fast the system settles to its target:
  /// - `0` (undamped): vibration never dies
  /// - `0 < Zeta <= 1` (underdamped): vibration dies
  /// - `Zeta > 1`: system does not vibrate, but approaches its target
  pub damping: T,
  /// `r`, the initial response
  ///
  /// - Negative: system anticipates motion by going in the opposite direction shortly
  /// - `0`: system takes time to begin accelerating from rest
  /// - Positive: system immediately reacts to changes
  /// - `r > 1`: system overshoots target at first
  pub initial_response: T,
}

impl<S> From<SpringConfig<S>> for SpringParameters<S>
where
  S: Scalar,
{
  fn from(description: SpringConfig<S>) -> SpringParameters<S> {
    let SpringConfig {
      frequency: f,
      damping: z,
      initial_response: r,
    } = description;

    let two_pi_f = S::two() * S::PI() * f;

    let k_1 = z / (S::PI() * f);
    let k_2 = S::one() / (two_pi_f * two_pi_f);
    let k_3 = r * z / two_pi_f;
    Self {
      k_1,
      k_2,
      k_3,
      max_safe_time_delta: (S::eval::<4>() * k_2 + k_1 * k_1).sqrt() - k_1,
    }
  }
}

struct SpringParameters<T> {
  k_1: T,
  k_2: T,
  k_3: T,
  max_safe_time_delta: T,
}

struct SpringState<T> {
  position: T,
  velocity: T,
}

/// A spring system for spring-mechanic-inspired animations
pub struct SpringSystem<T = f32, Scalar = f32> {
  parameters: SpringParameters<Scalar>,
  state: SpringState<T>,
  previous_target_position: T,
}

impl<T: Copy, S: Scalar> SpringSystem<T, S> {
  /// Construct a system with the specified parameters and initial position/velocity
  pub fn new(parameters: SpringConfig<S>, position: T, velocity: T) -> Self {
    Self {
      parameters: parameters.into(),
      state: SpringState { position, velocity },
      previous_target_position: position,
    }
  }
}

fn partial_min<T: PartialOrd>(a: T, b: T) -> T {
  if a < b {
    a
  } else {
    b
  }
}

impl<T, S> SpringSystem<T, S>
where
  T: Copy
    + Add<T, Output = T>
    + Sub<T, Output = T>
    + Mul<S, Output = T>
    + Div<S, Output = T>
    + AddAssign<T>,
  S: Copy + PartialOrd,
{
  /// Perform a step of the simulation and return the new position
  ///
  /// If the `time_delta` is too large to safely use without losing stability, it will be clamped to a safe maximum value
  pub fn step_clamped(&mut self, time_delta: S, target: T) -> T {
    let estimated_velocity = target - self.previous_target_position;
    let clamped_delta = partial_min(time_delta, self.parameters.max_safe_time_delta);

    self.step_with_target_velocity(clamped_delta, target, estimated_velocity);
    self.previous_target_position = target;

    self.state.position
  }

  fn step_with_target_velocity(&mut self, time_delta: S, target: T, target_velocity: T) {
    let SpringParameters { k_1, k_2, k_3, .. } = self.parameters;

    self.state.position += self.state.velocity * time_delta;

    let acceleration =
      (target + target_velocity * k_3 - self.state.position - self.state.velocity * k_1) / k_2;
    self.state.velocity += acceleration * time_delta;
  }
}
