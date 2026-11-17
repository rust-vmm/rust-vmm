// Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Adds to the state objects from `vm-superio` serialization capabilities.
//!
//! Provides wrappers over the state objects from `vm-superio` crate which
//! implement the `serde::Serialize/Deserialize` traits as well.

#![deny(missing_docs)]

pub mod rtc_pl031;
pub mod serial;

pub use rtc_pl031::RtcStateSer;
pub use serial::SerialStateSer;
