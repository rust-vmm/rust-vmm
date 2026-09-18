// Copyright 2026 Arthur Heymans
//
// SPDX-License-Identifier: Apache-2.0

//! [`AmlSink`] implementations.

use crate::AmlSink;

/// AML sink backed by a fixed-size byte buffer.
///
/// Writes bytes sequentially and panics if the buffer is too small.
pub struct FixedBufSink<'a> {
    buffer: &'a mut [u8],
    position: usize,
}

impl<'a> FixedBufSink<'a> {
    /// Create a sink that writes at the start of `buffer`.
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Return the number of bytes written.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Return the remaining capacity.
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }

    /// Return the bytes written so far.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.position]
    }
}

impl AmlSink for FixedBufSink<'_> {
    fn byte(&mut self, byte: u8) {
        assert!(
            self.position < self.buffer.len(),
            "FixedBufSink overflow: wrote {} bytes into {}-byte buffer",
            self.position + 1,
            self.buffer.len(),
        );
        self.buffer[self.position] = byte;
        self.position += 1;
    }

    fn vec(&mut self, bytes: &[u8]) {
        let end = self.position + bytes.len();
        assert!(
            end <= self.buffer.len(),
            "FixedBufSink overflow: need {} bytes, buffer has {} remaining",
            bytes.len(),
            self.remaining(),
        );
        self.buffer[self.position..end].copy_from_slice(bytes);
        self.position = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Aml;

    #[test]
    fn fixed_buffer_sink_writes_aml() {
        let mut buffer = [0u8; 8];
        let mut sink = FixedBufSink::new(&mut buffer);

        0x1234u16.to_aml_bytes(&mut sink);

        assert_eq!(sink.as_slice(), b"\x0b\x34\x12");
        assert_eq!(sink.position(), 3);
        assert_eq!(sink.remaining(), 5);
    }

    #[test]
    #[should_panic(expected = "FixedBufSink overflow")]
    fn fixed_buffer_sink_panics_on_overflow() {
        let mut buffer = [0u8; 2];
        let mut sink = FixedBufSink::new(&mut buffer);
        sink.vec(&[1, 2, 3]);
    }
}
