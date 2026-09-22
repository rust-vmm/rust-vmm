// Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! Provides a wrapper over an `SerialState` that has serialization capabilities.
//!
//! This module defines the `SerialStateSer` abstraction which mirrors the
//! `SerialState` from the base crate, and adds on top of it derives for
//! the `serde::Serialize/Deserialize` traits.

use serde::{Deserialize, Serialize};
use vm_superio::SerialState;

/// Wrapper over an `SerialState` that has serialization capabilities.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SerialStateSer {
    /// Divisor Latch Low Byte
    pub baud_divisor_low: u8,
    /// Divisor Latch High Byte
    pub baud_divisor_high: u8,
    /// Interrupt Enable Register
    pub interrupt_enable: u8,
    /// Interrupt Identification Register
    pub interrupt_identification: u8,
    /// Line Control Register
    pub line_control: u8,
    /// Line Status Register
    pub line_status: u8,
    /// Modem Control Register
    pub modem_control: u8,
    /// Modem Status Register
    pub modem_status: u8,
    /// Scratch Register
    pub scratch: u8,
    /// Transmitter Holding Buffer/Receiver Buffer
    pub in_buffer: Vec<u8>,
}

// The following `From` implementations can be used to convert from an `SerialStateSer` to the
// `SerialState` from the base crate and vice versa.
impl From<&SerialStateSer> for SerialState {
    fn from(state: &SerialStateSer) -> Self {
        SerialState {
            baud_divisor_low: state.baud_divisor_low,
            baud_divisor_high: state.baud_divisor_high,
            interrupt_enable: state.interrupt_enable,
            interrupt_identification: state.interrupt_identification,
            line_control: state.line_control,
            line_status: state.line_status,
            modem_control: state.modem_control,
            modem_status: state.modem_status,
            scratch: state.scratch,
            in_buffer: state.in_buffer.clone(),
        }
    }
}

impl From<&SerialState> for SerialStateSer {
    fn from(state: &SerialState) -> Self {
        SerialStateSer {
            baud_divisor_low: state.baud_divisor_low,
            baud_divisor_high: state.baud_divisor_high,
            interrupt_enable: state.interrupt_enable,
            interrupt_identification: state.interrupt_identification,
            line_control: state.line_control,
            line_status: state.line_status,
            modem_control: state.modem_control,
            modem_status: state.modem_status,
            scratch: state.scratch,
            in_buffer: state.in_buffer.clone(),
        }
    }
}

impl Default for SerialStateSer {
    fn default() -> Self {
        SerialStateSer::from(&SerialState::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::sink;
    use std::ops::Deref;
    use vm_superio::serial::NoEvents;
    use vm_superio::{Serial, Trigger};
    use vmm_sys_util::eventfd::EventFd;

    const RAW_INPUT_BUF: [u8; 3] = [b'a', b'b', b'c'];

    struct EventFdTrigger(EventFd);

    impl Trigger for EventFdTrigger {
        type E = std::io::Error;

        fn trigger(&self) -> std::io::Result<()> {
            self.write(1)
        }
    }

    impl Deref for EventFdTrigger {
        type Target = EventFd;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl EventFdTrigger {
        pub fn new(flag: i32) -> Self {
            EventFdTrigger(EventFd::new(flag).unwrap())
        }

        pub fn try_clone(&self) -> Self {
            EventFdTrigger((**self).try_clone().unwrap())
        }
    }

    #[test]
    fn test_state_ser_default() {
        let default_serial_state_ser = SerialStateSer::default();
        assert_eq!(
            SerialState::from(&default_serial_state_ser),
            SerialState::default()
        );
    }

    #[test]
    fn test_state_ser_idempotency() {
        let state = SerialState::default();
        let state_ser = SerialStateSer::from(&state);
        let state_from_ser = SerialState::from(&state_ser);

        assert_eq!(state, state_from_ser);
    }

    #[test]
    fn test_state_ser() {
        let intr_evt = EventFdTrigger::new(libc::EFD_NONBLOCK);
        let mut serial = Serial::new(intr_evt.try_clone(), sink());

        serial.enqueue_raw_bytes(&RAW_INPUT_BUF).unwrap();

        let state = serial.state();
        let ser_state = SerialStateSer::from(&state);

        let state_after_restore = SerialState::from(&ser_state);
        let mut serial_after_restore =
            Serial::from_state(&state_after_restore, intr_evt.try_clone(), NoEvents, sink())
                .unwrap();

        RAW_INPUT_BUF.iter().for_each(|&c| {
            assert_eq!(serial_after_restore.read(0), c);
        });
        assert_eq!(state, state_after_restore);
    }

    #[test]
    fn test_ser_der_binary() {
        let state = SerialStateSer::default();
        let state_ser = bincode::serialize(&state).unwrap();
        let state_der = bincode::deserialize(&state_ser).unwrap();

        assert_eq!(state, state_der);
    }
}
