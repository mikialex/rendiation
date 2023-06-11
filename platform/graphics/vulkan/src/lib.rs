#![allow(dead_code)]

use std::ffi::{c_void, CStr, CString};

use anyhow::Result;
use ash::{extensions::ext::DebugUtils, vk, Entry, Instance as AshInstance};
use raw_window_handle::HasRawDisplayHandle;

mod instance;
pub use instance::*;

mod device;
pub use device::*;
