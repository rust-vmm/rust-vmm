// Copyright © 2020, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//

use serde::de::{Deserialize, Deserializer};
use serde::{Serialize, Serializer};

use super::regs::*;

impl<'de> Deserialize<'de> for LapicState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let regs: Vec<::std::os::raw::c_char> = Vec::deserialize(deserializer)?;
        let mut val = LapicState::default();
        // This panics if the source and destination have different lengths.
        val.regs.copy_from_slice(&regs[..]);
        Ok(val)
    }
}

impl Serialize for LapicState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let regs = &self.regs[..];
        regs.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for XSave {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data_buffer: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut val = XSave::default();
        // This panics if the source and destination have different lengths.
        val.buffer.copy_from_slice(&data_buffer[..]);
        Ok(val)
    }
}

impl Serialize for XSave {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data_buffer = &self.buffer[..];
        data_buffer.serialize(serializer)
    }
}

impl Serialize for AllVpStateComponents {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data_buffer = &self.buffer[..];
        data_buffer.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AllVpStateComponents {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data_buffer: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut val = AllVpStateComponents::default();
        // This panics if the source and destination have different lengths.
        val.buffer.copy_from_slice(&data_buffer[..]);
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use random_number::random;

    #[test]
    fn test_lapic_state_serialization_deserialization() {
        let mut state = LapicState::default();
        for i in 0..1024 {
            state.regs[i] = random!();
        }
        let serialized = serde_json::to_string(&state).expect("err ser");
        let d_state: LapicState = serde_json::from_str(&serialized).expect("err unser");
        assert!(state
            .regs
            .iter()
            .zip(d_state.regs.iter())
            .all(|(a, b)| a == b));
    }

    #[test]
    fn test_xsave_serialization_deserialization() {
        let mut xsave = XSave {
            ..Default::default()
        };
        for i in 0..4096 {
            xsave.buffer[i] = random!();
        }
        let serialized = serde_json::to_string(&xsave).expect("err ser");
        let d_xsave: XSave = serde_json::from_str(&serialized).expect("err unser");
        assert!(xsave
            .buffer
            .iter()
            .zip(d_xsave.buffer.iter())
            .all(|(a, b)| a == b));
    }

    #[test]
    fn test_vp_state_components_serialization_deserialization() {
        let mut states = AllVpStateComponents {
            ..Default::default()
        };
        for i in 0..VP_STATE_COMPONENTS_BUFFER_SIZE {
            states.buffer[i] = 0xC8;
        }
        let serialized = serde_json::to_string(&states).expect("err ser");
        let d_states: AllVpStateComponents = serde_json::from_str(&serialized).expect("err unser");
        assert!(states
            .buffer
            .iter()
            .zip(d_states.buffer.iter())
            .all(|(a, b)| a == b));
    }
}
