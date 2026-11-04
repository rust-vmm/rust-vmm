// Copyright 2023 Rivos, Inc.
//
// SPDX-License-Identifier: Apache-2.0
//

use zerocopy::{
    byteorder::{self, LE},
    Immutable, IntoBytes,
};

use crate::{assert_same_size, gas, Aml, AmlSink, Checksum, TableHeader};
use core::mem::size_of;

type U16 = byteorder::U16<LE>;
type U32 = byteorder::U32<LE>;

const PCI_VENDOR_ID_NONE: u16 = 0xffff;
const PCI_DEVICE_ID_NONE: u16 = 0xffff;

const EMPTY_NAMESPACE: [u8; 2] = [b'.', 0];

pub struct SPCR<'a> {
    header: TableHeader,
    info: SerialPortInfo,
    namespace_string: &'a [u8],
}

impl SPCR<'_> {
    fn new(
        oem_id: [u8; 6],
        oem_table_id: [u8; 8],
        oem_revision: u32,
        info: SerialPortInfo,
    ) -> Self {
        let mut header = TableHeader {
            signature: *b"SPCR",
            length: ((TableHeader::len() + SerialPortInfo::len() + EMPTY_NAMESPACE.len()) as u32)
                .into(),
            revision: 4,
            checksum: 0,
            oem_id,
            oem_table_id,
            oem_revision: oem_revision.into(),
            creator_id: crate::CREATOR_ID,
            creator_revision: crate::CREATOR_REVISION,
        };

        let mut checksum = Checksum::default();
        checksum.append(header.as_bytes());
        checksum.append(info.as_bytes());
        checksum.append(&EMPTY_NAMESPACE);
        header.checksum = checksum.value();

        Self {
            header,
            info,
            namespace_string: &EMPTY_NAMESPACE,
        }
    }

    pub fn sbi(oem_id: [u8; 6], oem_table_id: [u8; 8], oem_revision: u32) -> Self {
        Self::new(oem_id, oem_table_id, oem_revision, SerialPortInfo::sbi())
    }

    /// Create an SPCR for an ARM PL011 UART using a GIC interrupt.
    pub fn pl011(
        oem_id: [u8; 6],
        oem_table_id: [u8; 8],
        oem_revision: u32,
        base_address: u64,
        gsi: u32,
    ) -> Self {
        Self::new(
            oem_id,
            oem_table_id,
            oem_revision,
            SerialPortInfo::pl011(base_address, gsi),
        )
    }
}

impl Aml for SPCR<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.vec(self.header.as_bytes());
        sink.vec(self.info.as_bytes());
        sink.vec(self.namespace_string);
    }
}

#[allow(dead_code)]
#[repr(u16)]
enum SerialPortType {
    Serial = 0x8000,
    Uart1394 = 0x8001,
    Usb = 0x8002,
    Net = 0x8003,
}

#[allow(dead_code)]
#[repr(u8)]
enum SerialPortSubType {
    Fully16550Compatible = 0,
    Subset16550 = 1,
    Max311xESpi = 2,
    ArmPl011 = 3,
    Msm8x60 = 4,
    Nvidia16550 = 5,
    TiOmap = 6,
    // Note: 7 is reserved
    Apm88xxxx = 8,
    Msm8974 = 9,
    Sam5250 = 0xa,
    IntelUsif = 0xb,
    Imx6 = 0xc,
    ArmSbsa32BitOnlyDeprecated = 0xd,
    ArmSbsaGeneric = 0xe,
    ArmDcc = 0xf,
    Bcm2835 = 0x10,
    Sdm845_1p8432Mhz = 0x11,
    Gas16550 = 0x12,
    Sdm845_7p372Mhz = 0x13,
    IntelLpss = 0x14,
    RiscvSbi = 0x15,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, IntoBytes, Immutable)]
struct SerialPortInfo {
    interface_type: u8,
    reserved0: [u8; 3],
    base_address: gas::GAS,
    interrupt_type: u8,
    irq: u8,
    gsi: U32,
    baud_rate: u8,
    parity: u8,
    stop_bits: u8,
    flow_control: u8,
    terminal_type: u8,
    language: u8,
    pci_device_id: U16,
    pci_vendor_id: U16,
    pci_bus: u8,
    pci_device: u8,
    pci_function: u8,
    pci_flags: U32,
    pci_segment: u8,
    clock_frequency: U32,
    precise_baud: U32,
    namespace_string_len: U16,
    namespace_string_offset: U16,
}

assert_same_size!(SerialPortInfo, [u8; 52]);

impl SerialPortInfo {
    fn len() -> usize {
        size_of::<Self>()
    }

    pub fn sbi() -> Self {
        Self {
            interface_type: SerialPortSubType::RiscvSbi as u8,
            reserved0: [0, 0, 0],
            base_address: gas::GAS::default(),
            interrupt_type: 0,
            irq: 0,
            gsi: 0.into(),
            baud_rate: 0,
            parity: 0,
            stop_bits: 0,
            flow_control: 0,
            terminal_type: 0,
            language: 0,
            pci_device_id: PCI_DEVICE_ID_NONE.into(),
            pci_vendor_id: PCI_VENDOR_ID_NONE.into(),
            pci_bus: 0,
            pci_device: 0,
            pci_function: 0,
            pci_flags: 0.into(),
            pci_segment: 0,
            clock_frequency: 0.into(),
            precise_baud: 0.into(),
            namespace_string_len: 2.into(),
            namespace_string_offset: (Self::len() as u16).into(),
        }
    }

    pub fn pl011(base_address: u64, gsi: u32) -> Self {
        Self {
            interface_type: SerialPortSubType::ArmPl011 as u8,
            reserved0: [0; 3],
            base_address: gas::GAS::new(
                gas::AddressSpace::SystemMemory,
                32,
                0,
                gas::AccessSize::DwordAccess,
                base_address,
            ),
            interrupt_type: 0x08,
            irq: 0,
            gsi: gsi.into(),
            baud_rate: 7, // 115200 baud
            parity: 0,
            stop_bits: 1,
            flow_control: 0,
            terminal_type: 0,
            language: 0,
            pci_device_id: PCI_DEVICE_ID_NONE.into(),
            pci_vendor_id: PCI_VENDOR_ID_NONE.into(),
            pci_bus: 0,
            pci_device: 0,
            pci_function: 0,
            pci_flags: 0.into(),
            pci_segment: 0,
            clock_frequency: 0.into(),
            precise_baud: 0.into(),
            namespace_string_len: (EMPTY_NAMESPACE.len() as u16).into(),
            namespace_string_offset: (Self::len() as u16).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_sbi_spcr() {
        let spcr = SPCR::sbi(*b"SSPCRR", *b"SOMETHIN", 0xcafe_d00d);
        let mut bytes = Vec::new();
        spcr.to_aml_bytes(&mut bytes);

        let sum = bytes.iter().fold(0u8, |acc, x| acc.wrapping_add(*x));
        assert_eq!(sum, 0);
        assert_eq!(bytes.len(), TableHeader::len() + SerialPortInfo::len() + 2);
        assert_eq!(bytes[0..4], *b"SPCR");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 90);
    }

    #[test]
    fn test_pl011_spcr() {
        let spcr = SPCR::pl011(*b"SSPCRR", *b"SOMETHIN", 1, 0x6000_0000, 33);
        let mut bytes = Vec::new();
        spcr.to_aml_bytes(&mut bytes);

        assert_eq!(
            bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 90);
        assert_eq!(bytes[36], SerialPortSubType::ArmPl011 as u8);
        assert_eq!(
            u64::from_le_bytes(bytes[44..52].try_into().unwrap()),
            0x6000_0000
        );
        assert_eq!(u32::from_le_bytes(bytes[54..58].try_into().unwrap()), 33);
        assert_eq!(bytes[58], 7);
    }
}
