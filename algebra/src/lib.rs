// Copyright 2019 The Aljabar Developers. For a full listing of authors,
// refer to the Cargo.toml file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
//! The super generic super experimental linear algebra library.
//!
//! `aljabar` is roughly compatibly with [cgmath](https://github.com/rustgd/cgmath)
//! and is intended to provide a small set of lightweight linear algebra
//! operations typically useful in interactive computer graphics.
//!
//! `aljabar` is n-dimensional, meaning that its data structures support an
//! arbitrary number of elements. If you wish to create a five-dimensional rigid
//! body simulation, `aljabar` can help you.
//!
//! ## Getting started
//!
//! All of `aljabar`'s types are exported in the root of the crate, so importing
//! them all is as easy as adding the following to the top of your source file:
//!
//! ```
//! use aljabar::*;
//! ```
//!
//! After that, you can begin using `aljabar`.
//!
//! ### Vector
//!
//! [Vectors](Vector) can be constructed from arrays of any type and size.
//! Use the [vector!] macro to easily construct a vector:
//!
//! ```
//! # use aljabar::*;
//! let a = vector![ 0u32, 1, 2, 3 ];
//! assert_eq!(
//!     a,
//!     Vector::<u32, 4>::from([ 0u32, 1, 2, 3 ])
//! );
//! ```
//!
//! [Add], [Sub], and [Neg] will be properly implemented for any `Vector<Scalar,
//! N>` for any respective implementation of such operations for `Scalar`.
//! Operations are only implemented for vectors of equal sizes.
//!
//! ```
//! # use aljabar::*;
//! let b = vector![ 0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, ];
//! let c = vector![ 1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, ] * 0.5;
//! assert_eq!(
//!     b + c,
//!     vector![ 0.5f32, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5 ]
//! );
//! ```
//!
//! If the scalar type implements [Mul] as well, then the Vector will be an
//! [InnerSpace] and have the [dot](InnerSpace::dot) product defined for it,
//! as well as the ability to find the squared distance between two vectors
//! (implements [MetricSpace]) and  the squared magnitude of a vector. If the
//! scalar type is a real number then the  distance between two vectors and
//! the magnitude of a vector can be found in addition:
//!
//! ```rust
//! # use aljabar::*;
//! let a = vector!(1i32, 1);
//! let b = vector!(5i32, 5);
//! assert_eq!(a.distance2(b), 32);       // distance method not implemented.
//! assert_eq!((b - a).magnitude2(), 32); // magnitude method not implemented.
//!
//! let a = vector!(1.0f32, 1.0);
//! let b = vector!(5.0f32, 5.0);
//! const close: f32 = 5.65685424949;
//! assert_eq!(a.distance(b), close);       // distance is implemented.
//! assert_eq!((b - a).magnitude(), close); // magnitude is implemented.
//!
//! // Vector normalization is also supported for floating point scalars.
//! assert_eq!(
//!     vector!(0.0f32, 20.0, 0.0)
//!         .normalize(),
//!     vector!(0.0f32, 1.0, 0.0)
//! );
//! ```
//!
//! ### Matrix
//!
//! [Matrices](Matrix) can be created from arrays of vectors of any size
//! and scalar type. Matrices are column-major and constructing a matrix from a
//! raw array reflects that. The [matrix!] macro can be used to construct a
//! matrix in row-major order:
//!
//! ```ignore
//! # use aljabar::*;
//! let a = Matrix::<f32, 3, 3>::from([
//!     vector!(1.0, 0.0, 0.0),
//!     vector!(0.0, 1.0, 0.0),
//!     vector!(0.0, 0.0, 1.0),
//! ]);
//!
//! let b: Matrix::<i32, 3, 3> = matrix![
//!     [ 0, -3, 5 ],
//!     [ 6, 1, -4 ],
//!     [ 2, 3, -2 ]
//! ];
//! ```
//!
//! All operations performed on matrices produce fixed-size outputs. For
//! example, taking the [transpose](Matrix::transpose) of a non-square matrix
//! will produce a matrix with the width and height swapped:
//!
//! ```ignore
//! # use aljabar::*;
//! assert_eq!(
//!     Matrix::<i32, 1, 2>::from([ vector!( 1 ), vector!(2) ])
//!         .transpose(),
//!     Matrix::<i32, 2, 1>::from([ vector!( 1, 2 ) ])
//! );
//! ```
//!
//! As with Vectors, if the underlying scalar type supports the appropriate
//! operations, a matrix will implement element-wise [Add] and [Sub] for
//! matrices of equal size:
//!
//! ```
//! # use aljabar::*;
//! let a = matrix!(1_u32);
//! let b = matrix!(2_u32);
//! let c = matrix!(3_u32);
//! assert_eq!(a + b, c);
//! ```
//!
//! And this is true for any type that implements [Add], so therefore the
//! following is possible as well:
//!
//! ```
//! # use aljabar::*;
//! let a = matrix!(matrix!(1_u32));
//! let b = matrix!(matrix!(2_u32));
//! let c = matrix!(matrix!(3_u32));
//! assert_eq!(a + b, c);
//! ```
//!
//! For a given type `T`, if `T: Clone` and `Vector<T, _>` is an [InnerSpace],
//! then multiplication is defined for `Matrix<T, N, M> * Matrix<T, M, P>`. The
//! result is a `Matrix<T, N, P>`:
//!
//! ```rust
//! # use aljabar::*;
//! let a: Matrix::<i32, 3, 3> = matrix![
//!     [ 0, -3, 5 ],
//!     [ 6, 1, -4 ],
//!     [ 2, 3, -2 ],
//! ];
//! let b: Matrix::<i32, 3, 3> = matrix![
//!     [ -1, 0, -3 ],
//!     [  4, 5,  1 ],
//!     [  2, 6, -2 ],
//! ];
//! let c: Matrix::<i32, 3, 3> = matrix![
//!     [  -2,  15, -13 ],
//!     [ -10, -19,  -9 ],
//!     [   6,   3,   1 ],
//! ];
//! assert_eq!(
//!     a * b,
//!     c
//! );
//! ```

#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(const_evaluatable_checked)]
#![feature(const_generics)]
#![feature(trivial_bounds)]
#![feature(maybe_uninit_ref)]
#![feature(maybe_uninit_uninit_array)]

use core::{
  cmp::PartialOrd,
  fmt,
  hash::{Hash, Hasher},
  iter::{FromIterator, Product},
  marker::PhantomData,
  mem::{self, transmute_copy, MaybeUninit},
  ops::{
    Add, AddAssign, Deref, DerefMut, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub,
    SubAssign,
  },
};

#[cfg(feature = "mint")]
use mint;

#[cfg(feature = "serde")]
use serde::{
  de::{Error, SeqAccess, Visitor},
  ser::SerializeTuple,
  Deserialize, Deserializer, Serialize, Serializer,
};

mod array;
mod matrix;
mod point;
mod rotation;
pub mod row_view;
mod space;
mod test;
mod vector;

pub use array::*;
pub use matrix::*;
pub use point::*;
pub use rotation::*;
use row_view::*;
pub use space::*;
pub use vector::*;
