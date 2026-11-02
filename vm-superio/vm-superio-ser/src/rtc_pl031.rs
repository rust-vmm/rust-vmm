// Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Provides a wrapper over an `RtcState` that has serialization capabilities.
//!
//! This module defines the `RtcStateSer` abstraction which mirrors the
//! `RtcState` from the base crate, and adds on top of it derives for
//! the `serde::Serialize/Deserialize` traits.

use serde::{Deserialize, Serialize};
use vm_superio::RtcState;

/// Wrapper over an `RtcState` that has serialization capabilities.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RtcStateSer {
    /// The load register.
    pub lr: u32,
    /// The offset applied to the counter to get the RTC value.
    pub offset: i64,
    /// The MR register.
    pub mr: u32,
    /// The interrupt mask.
    pub imsc: u32,
    /// The raw interrupt value.
    pub ris: u32,
}

// The following `From` implementations can be used to convert from an `RtcStateSer` to the
// `RtcState` from the base crate and vice versa.
impl From<&RtcStateSer> for RtcState {
    fn from(state: &RtcStateSer) -> Self {
        RtcState {
            lr: state.lr,
            offset: state.offset,
            mr: state.mr,
            imsc: state.imsc,
            ris: state.ris,
        }
    }
}

impl From<&RtcState> for RtcStateSer {
    fn from(state: &RtcState) -> Self {
        RtcStateSer {
            lr: state.lr,
            offset: state.offset,
            mr: state.mr,
            imsc: state.imsc,
            ris: state.ris,
        }
    }
}

impl Default for RtcStateSer {
    fn default() -> Self {
        RtcStateSer::from(&RtcState::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm_superio::rtc_pl031::NoEvents;
    use vm_superio::Rtc;

    #[test]
    fn test_state_ser() {
        let mut rtc = Rtc::new();
        let mut data = [0; 4];

        // Do some operations with the RTC.
        // Get the RTC value with a load register of 0 (the initial value).
        rtc.read(0x000, &mut data);

        let data2 = [1; 4];
        // Write to LR register.
        rtc.write(0x008, &data2);

        let state = rtc.state();
        let ser_state = RtcStateSer::from(&state);

        let state_after_restore = RtcState::from(&ser_state);
        let mut rtc_after_restore = Rtc::from_state(&state_after_restore, NoEvents);

        // Reading from the LR register should return the same value as before saving the state.
        rtc_after_restore.read(0x008, &mut data);
        assert_eq!(data, data2);

        // Check that the old and the new state are identical when using the intermediate
        // `RtcStateSer` object as well.
        assert_eq!(state, state_after_restore);

        // Test the `Default` implementation of RtcStateSer.
        let default_rtc_state_ser = RtcStateSer::default();
        assert_eq!(RtcState::from(&default_rtc_state_ser), RtcState::default());
    }
}
