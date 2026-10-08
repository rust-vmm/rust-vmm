// Copyright 2026 Arthur Heymans
//
// SPDX-License-Identifier: Apache-2.0

//! Debug Port Table 2 (DBG2).

extern crate alloc;

use crate::{sdt::GenericAddress, sdt::Sdt, Aml, AmlSink};

const TABLE_HEADER_LENGTH: usize = 44;
const DEVICE_INFO_LENGTH: usize = 22;
const GAS_LENGTH: usize = 12;
const ADDRESS_SIZE_LENGTH: usize = 4;

/// PL011 debug port configuration.
pub struct Pl011Config<'a> {
    pub base_address: u64,
    pub address_size: u32,
    pub namespace: &'a str,
}

/// Debug Port Table 2.
pub struct DBG2 {
    table: Sdt,
}

impl DBG2 {
    /// Create a DBG2 containing one ARM PL011 debug port.
    pub fn pl011(
        oem_id: [u8; 6],
        oem_table_id: [u8; 8],
        oem_revision: u32,
        config: Pl011Config<'_>,
    ) -> Self {
        let namespace_length = u16::try_from(config.namespace.len() + 1).unwrap();
        let device_length = u16::try_from(
            DEVICE_INFO_LENGTH + GAS_LENGTH + ADDRESS_SIZE_LENGTH + usize::from(namespace_length),
        )
        .unwrap();
        let mut table = Sdt::new(
            *b"DBG2",
            (TABLE_HEADER_LENGTH + usize::from(device_length)) as u32,
            0,
            oem_id,
            oem_table_id,
            oem_revision,
        );

        table.write_u32(36, TABLE_HEADER_LENGTH as u32);
        table.write_u32(40, 1);

        let device = TABLE_HEADER_LENGTH;
        let base_address_offset = DEVICE_INFO_LENGTH as u16;
        let address_size_offset = base_address_offset + GAS_LENGTH as u16;
        let namespace_offset = address_size_offset + ADDRESS_SIZE_LENGTH as u16;

        table.write_u8(device, 0); // Device information revision
        table.write_u16(device + 1, device_length);
        table.write_u8(device + 3, 1); // Address count
        table.write_u16(device + 4, namespace_length);
        table.write_u16(device + 6, namespace_offset);
        table.write_u16(device + 12, 0x8000); // Serial port
        table.write_u16(device + 14, 0x0003); // ARM PL011
        table.write_u16(device + 18, base_address_offset);
        table.write_u16(device + 20, address_size_offset);
        table.write(
            device + base_address_offset as usize,
            GenericAddress::mmio_address::<u32>(config.base_address),
        );
        table.write_u32(device + address_size_offset as usize, config.address_size);
        let namespace = device + namespace_offset as usize;
        table.write_bytes(namespace, config.namespace.as_bytes());
        table.write_u8(namespace + config.namespace.len(), 0);

        Self { table }
    }
}

impl Aml for DBG2 {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.table.to_aml_bytes(sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    #[should_panic]
    fn test_dbg2_rejects_oversized_namespace() {
        let namespace = "a".repeat(u16::MAX as usize);
        let _ = DBG2::pl011(
            *b"RUSTVM",
            *b"TESTDBG2",
            1,
            Pl011Config {
                base_address: 0,
                address_size: 0x1000,
                namespace: &namespace,
            },
        );
    }

    #[test]
    fn test_dbg2_pl011() {
        let table = DBG2::pl011(
            *b"RUSTVM",
            *b"TESTDBG2",
            1,
            Pl011Config {
                base_address: 0x6000_0000,
                address_size: 0x1000,
                namespace: "\\_SB.COM0",
            },
        );
        let mut bytes = Vec::new();
        table.to_aml_bytes(&mut bytes);

        assert_eq!(
            bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 1);
        assert_eq!(
            u16::from_le_bytes(bytes[56..58].try_into().unwrap()),
            0x8000
        );
        assert_eq!(u16::from_le_bytes(bytes[58..60].try_into().unwrap()), 3);
        assert_eq!(
            u64::from_le_bytes(bytes[70..78].try_into().unwrap()),
            0x6000_0000,
        );
        assert_eq!(&bytes[82..92], b"\\_SB.COM0\0");
    }
}
