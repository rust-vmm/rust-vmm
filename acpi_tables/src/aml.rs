// Copyright © 2019 Intel Corporation
// Copyright © 2023 Rivos, Inc.
//
// SPDX-License-Identifier: Apache-2.0
//

extern crate alloc;

use crate::{gas, Aml, AmlSink};
use alloc::string::String;
use alloc::{vec, vec::Vec};

// AML byte stream defines
const ZEROOP: u8 = 0x00;
const ONEOP: u8 = 0x01;
const NAMEOP: u8 = 0x08;
const BYTEPREFIX: u8 = 0x0a;
const WORDPREFIX: u8 = 0x0b;
const DWORDPREFIX: u8 = 0x0c;
const STRINGOP: u8 = 0x0d;
const QWORDPREFIX: u8 = 0x0e;
const SCOPEOP: u8 = 0x10;
const BUFFEROP: u8 = 0x11;
const PACKAGEOP: u8 = 0x12;
const VARPACKAGEOP: u8 = 0x13;
const METHODOP: u8 = 0x14;
const IRQNOFLAGSDESC: u8 = 0x22;
const IRQDESC: u8 = 0x23;
const DUALNAMEPREFIX: u8 = 0x2e;
const MULTINAMEPREFIX: u8 = 0x2f;
const NAMECHARBASE: u8 = 0x40;

const EXTOPPREFIX: u8 = 0x5b;
const MUTEXOP: u8 = 0x01;
const CONDREFOFOP: u8 = 0x12;
const CREATEFIELDOP: u8 = 0x13;
const STALLOP: u8 = 0x21;
const SLEEPOP: u8 = 0x22;
const ACQUIREOP: u8 = 0x23;
const RELEASEOP: u8 = 0x27;
const OPREGIONOP: u8 = 0x80;
const FIELDOP: u8 = 0x81;
const DEVICEOP: u8 = 0x82;
const POWERRESOURCEOP: u8 = 0x84;
const THERMALZONEOP: u8 = 0x85;

const LOCAL0OP: u8 = 0x60;
const ARG0OP: u8 = 0x68;
const STOREOP: u8 = 0x70;
const REFOFOP: u8 = 0x71;
const ADDOP: u8 = 0x72;
const CONCATOP: u8 = 0x73;
const SUBTRACTOP: u8 = 0x74;
const INCREMENTOP: u8 = 0x75;
const DECREMENTOP: u8 = 0x76;
const MULTIPLYOP: u8 = 0x77;
const DIVIDEOP: u8 = 0x78;
const SHIFTLEFTOP: u8 = 0x79;
const SHIFTRIGHTOP: u8 = 0x7a;
const ANDOP: u8 = 0x7b;
const NANDOP: u8 = 0x7c;
const OROP: u8 = 0x7d;
const NOROP: u8 = 0x7e;
const XOROP: u8 = 0x7f;
const DEREFOFOP: u8 = 0x83;
const CONCATRESOP: u8 = 0x84;
const MODOP: u8 = 0x85;
const NOTIFYOP: u8 = 0x86;
const SIZEOFOP: u8 = 0x87;
const INDEXOP: u8 = 0x88;
const CREATEDWFIELDOP: u8 = 0x8a;
const OBJECTTYPEOP: u8 = 0x8e;
const CREATEQWFIELDOP: u8 = 0x8f;
const LANDOP: u8 = 0x90;
const LOROP: u8 = 0x91;
const LNOTOP: u8 = 0x92;
const LEQUALOP: u8 = 0x93;
const LGREATEROP: u8 = 0x94;
const LLESSOP: u8 = 0x95;
const TOBUFFEROP: u8 = 0x96;
const TOINTEGEROP: u8 = 0x99;
const TOSTRINGOP: u8 = 0x9c;
const MIDOP: u8 = 0x9e;
const IFOP: u8 = 0xa0;
const ELSEOP: u8 = 0xa1;
const WHILEOP: u8 = 0xa2;
const RETURNOP: u8 = 0xa4;
const BREAKOP: u8 = 0xa5;
const ONESOP: u8 = 0xff;

// AML resouce data fields
const IOPORTDESC: u8 = 0x47;
const ENDTAG: u8 = 0x79;
const REGDESC: u8 = 0x82;
const MEMORY32FIXEDDESC: u8 = 0x86;
const DWORDADDRSPACEDESC: u8 = 0x87;
const WORDADDRSPACEDESC: u8 = 0x88;
const EXTIRQDESC: u8 = 0x89;
const QWORDADDRSPACEDESC: u8 = 0x8A;
const GPIOCONNECTIONDESC: u8 = 0x8C;
const SERIALBUSCONNECTIONDESC: u8 = 0x8E;

/// Zero object in ASL.
pub const ZERO: Zero = Zero {};
pub struct Zero {}

impl Aml for Zero {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(ZEROOP);
    }
}

/// One object in ASL.
pub const ONE: One = One {};
pub struct One {}

impl Aml for One {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(ONEOP);
    }
}

/// Ones object represents all bits 1.
pub const ONES: Ones = Ones {};
pub struct Ones {}

impl Aml for Ones {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(ONESOP);
    }
}

/// Represents Namestring to construct ACPI objects like
/// Name/Device/Method/Scope and so on...
pub struct Path {
    root: bool,
    parent: bool,
    name_parts: Vec<[u8; 4]>,
}

impl Aml for Path {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        if self.root {
            sink.byte(b'\\');
        } else if self.parent {
            sink.byte(b'^');
        }

        match self.name_parts.len() {
            0 => panic!("Name cannot be empty"),
            1 => {}
            2 => {
                sink.byte(DUALNAMEPREFIX);
            }
            n => {
                sink.byte(MULTINAMEPREFIX);
                sink.byte(n as u8);
            }
        };

        for part in self.name_parts.clone().iter_mut() {
            sink.vec(part);
        }
    }
}

impl Path {
    /// Per ACPI Spec, the Namestring split by "." has 4 bytes long. So any name
    /// not has 4 bytes will not be accepted.
    pub fn new(name: &str) -> Self {
        let root = name.starts_with('\\');
        let parent = name.starts_with('^');
        let offset = (root || parent) as usize;

        let mut name_parts = Vec::new();
        for part in name[offset..].split('.') {
            assert_eq!(part.len(), 4);
            let mut name_part = [0u8; 4];
            name_part.copy_from_slice(part.as_bytes());
            name_parts.push(name_part);
        }

        Path {
            root,
            parent,
            name_parts,
        }
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Path::new(s)
    }
}

pub type Byte = u8;

impl Aml for Byte {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        match *self {
            0 => ZERO.to_aml_bytes(sink),
            1 => ONE.to_aml_bytes(sink),
            _ => {
                sink.byte(BYTEPREFIX);
                sink.byte(*self);
            }
        }
    }
}

pub type Word = u16;

impl Aml for Word {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        if *self <= Byte::MAX.into() {
            (*self as Byte).to_aml_bytes(sink);
        } else {
            sink.byte(WORDPREFIX);
            sink.word(*self);
        }
    }
}

pub type DWord = u32;

impl Aml for DWord {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        if *self <= Word::MAX.into() {
            (*self as Word).to_aml_bytes(sink);
        } else {
            sink.byte(DWORDPREFIX);
            sink.dword(*self);
        }
    }
}

pub type QWord = u64;

impl Aml for QWord {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        if *self <= DWord::MAX.into() {
            (*self as DWord).to_aml_bytes(sink);
        } else {
            sink.byte(QWORDPREFIX);
            sink.qword(*self);
        }
    }
}

/// Name object. bytes represents the raw AML data for it.
pub struct Name {
    bytes: Vec<u8>,
}

impl Aml for Name {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.vec(&self.bytes);
    }
}

impl Name {
    /// Create Name object:
    ///
    /// * `path` - The namestring.
    /// * `inner` - AML objects contained in this namespace.
    pub fn new(path: Path, inner: &dyn Aml) -> Self {
        let mut bytes = vec![NAMEOP];
        path.to_aml_bytes(&mut bytes);
        inner.to_aml_bytes(&mut bytes);
        Name { bytes }
    }

    /// Create Field name object
    ///
    /// * 'field_name' - name string
    pub fn new_field_name(field_name: &str) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(field_name.as_bytes());
        Name { bytes }
    }
}

/// Package object. 'children' represents the ACPI objects contained in this package.
pub struct Package<'a> {
    children: Vec<&'a dyn Aml>,
}

impl Aml for Package<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = vec![self.children.len() as u8];
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(PACKAGEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

impl<'a> Package<'a> {
    /// Create Package object:
    pub fn new(children: Vec<&'a dyn Aml>) -> Self {
        Package { children }
    }
}

/// Package object, but can be built dynamically
pub struct PackageBuilder {
    data: Vec<u8>,
    elements: usize,
}

impl Aml for PackageBuilder {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let pkg_length = create_pkg_length(self.data.len() + 1, true);

        sink.byte(PACKAGEOP);
        sink.vec(&pkg_length);
        sink.byte(self.elements as u8);
        sink.vec(&self.data);
    }
}

impl AmlSink for PackageBuilder {
    fn byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

    fn vec(&mut self, v: &[u8]) {
        self.data.extend_from_slice(v);
    }
}

impl PackageBuilder {
    /// Create new PackageBuilder
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            elements: 0,
        }
    }

    pub fn add_element(&mut self, aml: &dyn Aml) {
        aml.to_aml_bytes(self);
        self.elements += 1;
    }
}

impl Default for PackageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable Package Term
pub struct VarPackageTerm<'a> {
    data: &'a dyn Aml,
}

impl Aml for VarPackageTerm<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.data.to_aml_bytes(&mut bytes);

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(VARPACKAGEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

impl<'a> VarPackageTerm<'a> {
    /// Create Variable Package Term
    pub fn new(data: &'a dyn Aml) -> Self {
        VarPackageTerm { data }
    }
}

/*

From the ACPI spec for PkgLength:

"The high 2 bits of the first byte reveal how many follow bytes are in the PkgLength. If the
PkgLength has only one byte, bit 0 through 5 are used to encode the package length (in other
words, values 0-63). If the package length value is more than 63, more than one byte must be
used for the encoding in which case bit 4 and 5 of the PkgLeadByte are reserved and must be zero.
If the multiple bytes encoding is used, bits 0-3 of the PkgLeadByte become the least significant 4
bits of the resulting package length value. The next ByteData will become the next least
significant 8 bits of the resulting value and so on, up to 3 ByteData bytes. Thus, the maximum
package length is 2**28."

*/

/* Also used for NamedField but in that case the length is not included in itself */
fn create_pkg_length(len: usize, include_self: bool) -> Vec<u8> {
    let mut result = Vec::with_capacity(4);

    /* PkgLength is inclusive and includes the length bytes */
    let length_length = if len < (2usize.pow(6) - 1) {
        1
    } else if len < (2usize.pow(12) - 2) {
        2
    } else if len < (2usize.pow(20) - 3) {
        3
    } else {
        4
    };

    let length = len + if include_self { length_length } else { 0 };

    match length_length {
        1 => result.push(length as u8),
        2 => {
            result.push((1u8 << 6) | (length & 0xf) as u8);
            result.push((length >> 4) as u8)
        }
        3 => {
            result.push((2u8 << 6) | (length & 0xf) as u8);
            result.push((length >> 4) as u8);
            result.push((length >> 12) as u8);
        }
        _ => {
            result.push((3u8 << 6) | (length & 0xf) as u8);
            result.push((length >> 4) as u8);
            result.push((length >> 12) as u8);
            result.push((length >> 20) as u8);
        }
    }

    result
}

/// EISAName object. 'value' means the encoded u32 EisaIdString.
pub struct EISAName {
    value: DWord,
}

impl EISAName {
    /// Per ACPI Spec, the EisaIdString must be a String
    /// object of the form UUUNNNN, where U is an uppercase letter
    /// and N is a hexadecimal digit. No asterisks or other characters
    /// are allowed in the string.
    pub fn new(name: &str) -> Self {
        assert_eq!(name.len(), 7);

        let data = name.as_bytes();

        let value: u32 = ((u32::from(data[0].checked_sub(NAMECHARBASE).unwrap()) << 26)
            | (u32::from(data[1].checked_sub(NAMECHARBASE).unwrap()) << 21)
            | (u32::from(data[2].checked_sub(NAMECHARBASE).unwrap()) << 16)
            | (name.chars().nth(3).unwrap().to_digit(16).unwrap() << 12)
            | (name.chars().nth(4).unwrap().to_digit(16).unwrap() << 8)
            | (name.chars().nth(5).unwrap().to_digit(16).unwrap() << 4)
            | name.chars().nth(6).unwrap().to_digit(16).unwrap())
        .swap_bytes();

        EISAName { value }
    }
}

impl Aml for EISAName {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.value.to_aml_bytes(sink);
    }
}

pub type Usize = usize;

impl Aml for Usize {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        #[cfg(target_pointer_width = "16")]
        (*self as u16).to_aml_bytes(sink);
        #[cfg(target_pointer_width = "32")]
        (*self as u32).to_aml_bytes(sink);
        #[cfg(target_pointer_width = "64")]
        (*self as u64).to_aml_bytes(sink);
    }
}

fn create_aml_string(v: &str, sink: &mut dyn AmlSink) {
    sink.byte(STRINGOP);
    sink.vec(v.as_bytes());
    sink.byte(0x0); /* NullChar */
}

/// implement Aml trait for 'str' so that 'str' can be directly append to the aml vector
pub type AmlStr = &'static str;

impl Aml for AmlStr {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        create_aml_string(self, sink);
    }
}

/// implement Aml trait for 'String'. So purpose with str.
pub type AmlString = String;

impl Aml for AmlString {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        create_aml_string(self, sink);
    }
}

/// ResouceTemplate object. 'children' represents the ACPI objects in it.
pub struct ResourceTemplate<'a> {
    children: Vec<&'a dyn Aml>,
}

impl Aml for ResourceTemplate<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();

        // Add buffer data
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        // Mark with end and mark checksum as as always valid
        bytes.push(ENDTAG);
        bytes.push(0); /* zero checksum byte */

        // Buffer length is an encoded integer including buffer data
        // and EndTag and checksum byte
        let mut buffer_length = Vec::with_capacity(4);
        bytes.len().to_aml_bytes(&mut buffer_length);

        // PkgLength is everything else
        let pkg_length = create_pkg_length(bytes.len() + buffer_length.len(), true);

        sink.byte(BUFFEROP);
        sink.vec(&pkg_length);
        sink.vec(&buffer_length);
        sink.vec(&bytes);
    }
}

impl<'a> ResourceTemplate<'a> {
    /// Create ResouceTemplate object
    pub fn new(children: Vec<&'a dyn Aml>) -> Self {
        ResourceTemplate { children }
    }
}

/// Memory32Fixed object with read_write accessing type, and the base address/length.
pub struct Memory32Fixed {
    read_write: bool, /* true for read & write, false for read only */
    base: u32,
    length: u32,
}

impl Memory32Fixed {
    /// Create Memory32Fixed object.
    pub fn new(read_write: bool, base: u32, length: u32) -> Self {
        Memory32Fixed {
            read_write,
            base,
            length,
        }
    }
}

impl Aml for Memory32Fixed {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(MEMORY32FIXEDDESC); /* 32bit Fixed Memory Range Descriptor */
        for byte in 9u16.to_le_bytes() {
            sink.byte(byte);
        }

        // 9 bytes of payload
        sink.byte(self.read_write as u8);
        sink.dword(self.base);
        sink.dword(self.length);
    }
}

#[derive(Copy, Clone)]
enum AddressSpaceType {
    Memory,
    IO,
    BusNumber,
}

/// AddressSpaceCacheable represent cache types for AddressSpace object
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressSpaceCacheable {
    NotCacheable,
    Cacheable,
    WriteCombining,
    PreFetchable,
}

#[deprecated = "Spelling error - use AddressSpaceCacheable"]
pub type AddressSpaceCachable = AddressSpaceCacheable;

/// AddressSpace structure with type, resouce range and flags to
/// construct Memory/IO/BusNumber objects
pub struct AddressSpace<T> {
    type_: AddressSpaceType,
    min: T,
    max: T,
    type_flags: u8,
    translation: Option<T>,
}

impl<T: Default> AddressSpace<T> {
    /// Create DWordMemory/QWordMemory object
    pub fn new_memory(
        cacheable: AddressSpaceCacheable,
        read_write: bool,
        min: T,
        max: T,
        translation: Option<T>,
    ) -> Self {
        AddressSpace {
            type_: AddressSpaceType::Memory,
            min,
            max,
            type_flags: ((cacheable as u8) << 1) | read_write as u8,
            translation,
        }
    }

    /// Create WordIO/DWordIO/QWordIO object
    pub fn new_io(min: T, max: T, translation: Option<T>) -> Self {
        AddressSpace {
            type_: AddressSpaceType::IO,
            min,
            max,
            type_flags: 3, /* EntireRange */
            translation,
        }
    }

    /// Create WordBusNumber object
    pub fn new_bus_number(min: T, max: T) -> Self {
        AddressSpace {
            type_: AddressSpaceType::BusNumber,
            min,
            max,
            type_flags: 0,
            translation: None,
        }
    }

    fn push_header(&self, sink: &mut dyn AmlSink, descriptor: u8, length: usize) {
        sink.byte(descriptor); /* Word Address Space Descriptor */
        for byte in (length as u16).to_le_bytes() {
            sink.byte(byte);
        }
        sink.byte(self.type_ as u8); /* type */
        let generic_flags = (1 << 2) /* Min Fixed */ | (1 << 3); /* Max Fixed */
        sink.byte(generic_flags);
        sink.byte(self.type_flags);
    }
}

impl Aml for AddressSpace<u16> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.push_header(
            sink,
            WORDADDRSPACEDESC,                   /* Word Address Space Descriptor */
            3 + 5 * core::mem::size_of::<u16>(), /* 3 bytes of header + 5 u16 fields */
        );

        sink.word(0); /* Granularity */
        sink.word(self.min); /* Min */
        sink.word(self.max); /* Max */
        sink.word(self.translation.unwrap_or(0));
        let len = self.max - self.min + 1;
        sink.word(len); /* Length */
    }
}

impl Aml for AddressSpace<u32> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.push_header(
            sink,
            DWORDADDRSPACEDESC, /* DWord Address Space Descriptor */
            3 + 5 * core::mem::size_of::<u32>(), /* 3 bytes of header + 5 u32 fields */
        );

        sink.dword(0); /* Granularity */
        sink.dword(self.min); /* Min */
        sink.dword(self.max); /* Max */
        sink.dword(self.translation.unwrap_or(0)); /* Translation */
        let len = self.max - self.min + 1;
        sink.dword(len); /* Length */
    }
}

impl Aml for AddressSpace<u64> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.push_header(
            sink,
            QWORDADDRSPACEDESC, /* QWord Address Space Descriptor */
            3 + 5 * core::mem::size_of::<u64>(), /* 3 bytes of header + 5 u64 fields */
        );

        sink.qword(0); /* Granularity */
        sink.qword(self.min); /* Min */
        sink.qword(self.max); /* Max */
        sink.qword(self.translation.unwrap_or(0)); /* Translation */
        let len = self.max - self.min + 1;
        sink.qword(len); /* Length */
    }
}

/// IO resouce object with the IO range, alignment and length
pub struct IO {
    min: u16,
    max: u16,
    alignment: u8,
    length: u8,
}

impl IO {
    /// Create IO object
    pub fn new(min: u16, max: u16, alignment: u8, length: u8) -> Self {
        IO {
            min,
            max,
            alignment,
            length,
        }
    }
}

impl Aml for IO {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(IOPORTDESC); /* IO Port Descriptor */
        sink.byte(1); /* IODecode16 */
        sink.word(self.min);
        sink.word(self.max);
        sink.byte(self.alignment);
        sink.byte(self.length);
    }
}

/// Interrupt resouce object with the interrupt characters.
pub struct Interrupt {
    consumer: bool,
    edge_triggered: bool,
    active_low: bool,
    shared: bool,
    numbers: Vec<u32>,
}

impl Interrupt {
    /// Create Interrupt object with a single interrupt number.
    pub fn new(
        consumer: bool,
        edge_triggered: bool,
        active_low: bool,
        shared: bool,
        number: u32,
    ) -> Self {
        Self::new_multiple(consumer, edge_triggered, active_low, shared, vec![number])
    }

    /// Create Interrupt object with multiple interrupt numbers.
    pub fn new_multiple(
        consumer: bool,
        edge_triggered: bool,
        active_low: bool,
        shared: bool,
        numbers: Vec<u32>,
    ) -> Self {
        Interrupt {
            consumer,
            edge_triggered,
            active_low,
            shared,
            numbers,
        }
    }
}

impl Aml for Interrupt {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTIRQDESC); /* Extended IRQ Descriptor */
        sink.word(2 + 4 * self.numbers.len() as u16);
        let flags = ((self.shared as u8) << 3)
            | ((self.active_low as u8) << 2)
            | ((self.edge_triggered as u8) << 1)
            | self.consumer as u8;
        sink.byte(flags);
        sink.byte(self.numbers.len() as u8); /* count */
        self.numbers.iter().for_each(|n| sink.dword(*n));
    }
}

/// IRQ resource object.
pub struct Irq {
    edge_triggered: bool,
    active_low: bool,
    shared: bool,
    number: u8,
}

impl Irq {
    /// Create IRQ object
    pub fn new(edge_triggered: bool, active_low: bool, shared: bool, number: u8) -> Self {
        Self {
            edge_triggered,
            active_low,
            shared,
            number,
        }
    }
}

impl Aml for Irq {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(IRQDESC); /* IRQ Descriptor */
        write_irq_mask_bytes(self.number, sink);
        let flags = ((self.shared as u8) << 4)
            | ((self.active_low as u8) << 3)
            | (self.edge_triggered as u8);
        sink.byte(flags);
    }
}

/// IRQNoFlags resource object.
pub struct IrqNoFlags {
    number: u8,
}

impl IrqNoFlags {
    /// Create IRQNoFlags object
    pub fn new(number: u8) -> Self {
        Self { number }
    }
}

impl Aml for IrqNoFlags {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(IRQNOFLAGSDESC); /* IRQNoFlags Descriptor */
        write_irq_mask_bytes(self.number, sink);
    }
}

/// GPIO interrupt trigger mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpioIntMode {
    Level = 0,
    Edge = 1,
}

/// GPIO interrupt polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpioIntPolarity {
    ActiveHigh = 0,
    ActiveLow = 1,
    ActiveBoth = 2,
}

/// GPIO pin pull configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpioPinConfig {
    Default = 0,
    PullUp = 1,
    PullDown = 2,
    PullNone = 3,
}

/// GPIO I/O restriction mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpioIoRestriction {
    None = 0,
    InputOnly = 1,
    OutputOnly = 2,
    Preserve = 3,
}

/// GPIO Interrupt Connection resource descriptor.
pub struct GpioInt<'a> {
    pub resource_source: &'a str,
    pub pins: &'a [u16],
    pub consumer: bool,
    pub mode: GpioIntMode,
    pub polarity: GpioIntPolarity,
    pub shared: bool,
    pub pin_config: GpioPinConfig,
    pub debounce: u16,
}

impl Aml for GpioInt<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        const PIN_TABLE_OFFSET: u16 = 23;

        let pin_table_size = u16::try_from(self.pins.len() * 2).unwrap();
        let resource_source_size = u16::try_from(self.resource_source.len() + 1).unwrap();
        let resource_source_offset = PIN_TABLE_OFFSET.checked_add(pin_table_size).unwrap();
        let vendor_data_offset = resource_source_offset
            .checked_add(resource_source_size)
            .unwrap();

        sink.byte(GPIOCONNECTIONDESC);
        sink.word(vendor_data_offset - 3);
        sink.byte(1); // Revision ID
        sink.byte(0); // Interrupt connection
        sink.word(self.consumer as u16);
        sink.word(self.mode as u16 | ((self.polarity as u16) << 1) | ((self.shared as u16) << 3));
        sink.byte(self.pin_config as u8);
        sink.word(0); // Output drive strength
        sink.word(self.debounce);
        sink.word(PIN_TABLE_OFFSET);
        sink.byte(0); // Resource source index
        sink.word(resource_source_offset);
        sink.word(vendor_data_offset);
        sink.word(0); // Vendor data length
        for &pin in self.pins {
            sink.word(pin);
        }
        sink.vec(self.resource_source.as_bytes());
        sink.byte(0);
    }
}

/// GPIO I/O Connection resource descriptor.
pub struct GpioIo<'a> {
    pub resource_source: &'a str,
    pub pins: &'a [u16],
    pub consumer: bool,
    pub io_restriction: GpioIoRestriction,
    pub pin_config: GpioPinConfig,
    pub drive_strength: u16,
    pub debounce: u16,
}

impl Aml for GpioIo<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        const PIN_TABLE_OFFSET: u16 = 23;

        let pin_table_size = u16::try_from(self.pins.len() * 2).unwrap();
        let resource_source_size = u16::try_from(self.resource_source.len() + 1).unwrap();
        let resource_source_offset = PIN_TABLE_OFFSET.checked_add(pin_table_size).unwrap();
        let vendor_data_offset = resource_source_offset
            .checked_add(resource_source_size)
            .unwrap();

        sink.byte(GPIOCONNECTIONDESC);
        sink.word(vendor_data_offset - 3);
        sink.byte(1); // Revision ID
        sink.byte(1); // I/O connection
        sink.word(self.consumer as u16);
        sink.word(self.io_restriction as u16);
        sink.byte(self.pin_config as u8);
        sink.word(self.drive_strength);
        sink.word(self.debounce);
        sink.word(PIN_TABLE_OFFSET);
        sink.byte(0); // Resource source index
        sink.word(resource_source_offset);
        sink.word(vendor_data_offset);
        sink.word(0); // Vendor data length
        for &pin in self.pins {
            sink.word(pin);
        }
        sink.vec(self.resource_source.as_bytes());
        sink.byte(0);
    }
}

/// I2C Serial Bus Connection resource descriptor.
pub struct I2cSerialBus<'a> {
    pub resource_source: &'a str,
    pub slave_address: u16,
    pub connection_speed: u32,
    pub address_10bit: bool,
    pub consumer: bool,
}

impl Aml for I2cSerialBus<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        const TYPE_DATA_LENGTH: u16 = 6;

        let data_length = 9 + TYPE_DATA_LENGTH as usize + self.resource_source.len() + 1;
        sink.byte(SERIALBUSCONNECTIONDESC);
        sink.word(u16::try_from(data_length).unwrap());
        sink.byte(1); // Revision ID
        sink.byte(0); // Resource source index
        sink.byte(1); // I2C
        sink.byte((self.consumer as u8) << 1);
        sink.word(self.address_10bit as u16);
        sink.byte(1); // Type-specific revision ID
        sink.word(TYPE_DATA_LENGTH);
        sink.dword(self.connection_speed);
        sink.word(self.slave_address);
        sink.vec(self.resource_source.as_bytes());
        sink.byte(0);
    }
}

/// SPI clock phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiClockPhase {
    First = 0,
    Second = 1,
}

/// SPI clock polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiClockPolarity {
    Low = 0,
    High = 1,
}

/// SPI wire mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiWireMode {
    FourWire = 0,
    ThreeWire = 1,
}

/// SPI chip-select polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiDevicePolarity {
    ActiveLow = 0,
    ActiveHigh = 1,
}

/// SPI Serial Bus Connection resource descriptor.
pub struct SpiSerialBus<'a> {
    pub resource_source: &'a str,
    pub connection_speed: u32,
    pub data_bit_length: u8,
    pub clock_phase: SpiClockPhase,
    pub clock_polarity: SpiClockPolarity,
    pub wire_mode: SpiWireMode,
    pub device_polarity: SpiDevicePolarity,
    pub device_selection: u16,
    pub consumer: bool,
}

impl Aml for SpiSerialBus<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        const TYPE_DATA_LENGTH: u16 = 9;

        let data_length = 9 + TYPE_DATA_LENGTH as usize + self.resource_source.len() + 1;
        sink.byte(SERIALBUSCONNECTIONDESC);
        sink.word(u16::try_from(data_length).unwrap());
        sink.byte(1); // Revision ID
        sink.byte(0); // Resource source index
        sink.byte(2); // SPI
        sink.byte((self.consumer as u8) << 1);
        sink.word(self.wire_mode as u16 | ((self.device_polarity as u16) << 1));
        sink.byte(1); // Type-specific revision ID
        sink.word(TYPE_DATA_LENGTH);
        sink.dword(self.connection_speed);
        sink.byte(self.data_bit_length);
        sink.byte(self.clock_phase as u8);
        sink.byte(self.clock_polarity as u8);
        sink.word(self.device_selection);
        sink.vec(self.resource_source.as_bytes());
        sink.byte(0);
    }
}

fn write_irq_mask_bytes(number: u8, sink: &mut dyn AmlSink) {
    assert!(number <= 15);
    if number < 8 {
        sink.byte(1 << number);
        sink.byte(0);
    } else {
        sink.byte(0);
        sink.byte(1 << (number - 8));
    }
}

/// Register resource object
pub struct Register {
    reg: gas::GAS,
}

impl Register {
    pub fn new(reg: gas::GAS) -> Self {
        Self { reg }
    }
}

impl Aml for Register {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(REGDESC); /* Register Descriptor */
        sink.word(0x12); // length
        self.reg.to_aml_bytes(sink);
    }
}

/// Device object with its device name and children objects in it.
pub struct Device<'a> {
    path: Path,
    children: Vec<&'a dyn Aml>,
}

impl Aml for Device<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.path.to_aml_bytes(&mut bytes);
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(EXTOPPREFIX); /* ExtOpPrefix */
        sink.byte(DEVICEOP); /* DeviceOp */
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

impl<'a> Device<'a> {
    /// Create Device object
    pub fn new(path: Path, children: Vec<&'a dyn Aml>) -> Self {
        Device { path, children }
    }
}

/// Scope object with its name and children objects in it.
pub struct Scope<'a> {
    path: Path,
    children: Vec<&'a dyn Aml>,
}

impl Aml for Scope<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.path.to_aml_bytes(&mut bytes);
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(SCOPEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

impl<'a> Scope<'a> {
    /// Create Scope object
    pub fn new(path: Path, children: Vec<&'a dyn Aml>) -> Self {
        Scope { path, children }
    }

    /// Create raw bytes representing a Scope from its children in raw bytes
    pub fn raw(path: Path, mut children: Vec<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(SCOPEOP);
        path.to_aml_bytes(&mut bytes);
        bytes.append(&mut children);

        let n = bytes.len(); // n >= 1
        let pkg_length = create_pkg_length(n - 1, true);
        let m = pkg_length.len();

        // move everything after the SCOPEOP over and copy in pkg_length
        bytes.resize(n + m, 0xFF);
        bytes.as_mut_slice().copy_within(1..n, m + 1);
        bytes.as_mut_slice()[1..m + 1].copy_from_slice(pkg_length.as_slice());

        bytes
    }
}

/// Method object with its name, children objects, arguments and serialized character.
pub struct Method<'a> {
    path: Path,
    children: Vec<&'a dyn Aml>,
    args: u8,
    serialized: bool,
}

impl<'a> Method<'a> {
    /// Create Method object.
    pub fn new(path: Path, args: u8, serialized: bool, children: Vec<&'a dyn Aml>) -> Self {
        Method {
            path,
            children,
            args,
            serialized,
        }
    }
}

impl Aml for Method<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.path.to_aml_bytes(&mut bytes);
        let flags: u8 = (self.args & 0x7) | ((self.serialized as u8) << 3);
        bytes.push(flags);
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(METHODOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// FieldAccessType defines the field accessing types.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldAccessType {
    Any,
    Byte,
    Word,
    DWord,
    QWord,
    Buffer,
}

/// FieldLockRule defines the rules whether to use the Global Lock.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldLockRule {
    NoLock = 0,
    Lock = 1,
}

/// FieldUpdateRule defines the rules to update the field.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldUpdateRule {
    Preserve = 0,
    WriteAsOnes = 1,
    WriteAsZeroes = 2,
}

/// FieldEntry defines the field entry.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldEntry {
    Named([u8; 4], usize),
    Reserved(usize),
}

/// Field object with the region name, field entries, access type and update rules.
pub struct Field {
    path: Path,

    fields: Vec<FieldEntry>,
    access_type: FieldAccessType,
    lock_rule: FieldLockRule,
    update_rule: FieldUpdateRule,
}

impl Field {
    /// Create Field object
    pub fn new(
        path: Path,
        access_type: FieldAccessType,
        lock_rule: FieldLockRule,
        update_rule: FieldUpdateRule,
        fields: Vec<FieldEntry>,
    ) -> Self {
        Field {
            path,
            access_type,
            lock_rule,
            update_rule,
            fields,
        }
    }
}

impl Aml for Field {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.path.to_aml_bytes(&mut bytes);

        let flags: u8 = self.access_type as u8
            | ((self.lock_rule as u8) << 4)
            | ((self.update_rule as u8) << 5);
        bytes.push(flags);

        for field in self.fields.iter() {
            match field {
                FieldEntry::Named(name, length) => {
                    bytes.extend_from_slice(name);
                    bytes.append(&mut create_pkg_length(*length, false));
                }
                FieldEntry::Reserved(length) => {
                    bytes.push(0x0);
                    bytes.append(&mut create_pkg_length(*length, false));
                }
            }
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(EXTOPPREFIX);
        sink.byte(FIELDOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// The space type for OperationRegion object
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OpRegionSpace {
    SystemMemory,
    SystemIO,
    PCIConfig,
    EmbeddedControl,
    SMBus,
    SystemCMOS,
    PciBarTarget,
    IPMI,
    GeneralPurposeIO,
    GenericSerialBus,
}

/// OperationRegion object with region name, region space type, its offset and length.
pub struct OpRegion<'a> {
    path: Path,
    space: OpRegionSpace,
    offset: &'a dyn Aml,
    length: &'a dyn Aml,
}

impl<'a> OpRegion<'a> {
    /// Create OperationRegion object.
    pub fn new(path: Path, space: OpRegionSpace, offset: &'a dyn Aml, length: &'a dyn Aml) -> Self {
        OpRegion {
            path,
            space,
            offset,
            length,
        }
    }
}

impl Aml for OpRegion<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(OPREGIONOP);
        self.path.to_aml_bytes(sink);
        sink.byte(self.space as u8);
        self.offset.to_aml_bytes(sink);
        self.length.to_aml_bytes(sink);
    }
}

/// If object with the if condition(predicate) and the body presented by the if_children objects.
pub struct If<'a> {
    predicate: &'a dyn Aml,
    if_children: Vec<&'a dyn Aml>,
}

impl<'a> If<'a> {
    /// Create If object.
    pub fn new(predicate: &'a dyn Aml, if_children: Vec<&'a dyn Aml>) -> Self {
        If {
            predicate,
            if_children,
        }
    }
}

impl Aml for If<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.predicate.to_aml_bytes(&mut bytes);
        for child in self.if_children.iter() {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(IFOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// Else object
pub struct Else<'a> {
    body: Vec<&'a dyn Aml>,
}

impl<'a> Else<'a> {
    /// Create Else object.
    pub fn new(body: Vec<&'a dyn Aml>) -> Self {
        Else { body }
    }
}

impl Aml for Else<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        for child in self.body.iter() {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(ELSEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

macro_rules! compare_op {
    ($name:ident, $opcode:expr, $invert:expr) => {
        /// Compare object with its right part and left part, which are both ACPI Object.
        pub struct $name<'a> {
            right: &'a dyn Aml,
            left: &'a dyn Aml,
        }

        impl<'a> $name<'a> {
            /// Create the compare object method.
            pub fn new(left: &'a dyn Aml, right: &'a dyn Aml) -> Self {
                $name { left, right }
            }
        }

        impl<'a> Aml for $name<'a> {
            fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
                if $invert {
                    sink.byte(LNOTOP);
                }
                sink.byte($opcode);
                self.left.to_aml_bytes(sink);
                self.right.to_aml_bytes(sink);
            }
        }
    };
}

compare_op!(LogicalAnd, LANDOP, false);
compare_op!(LogicalOr, LOROP, false);
compare_op!(Equal, LEQUALOP, false);
compare_op!(LessThan, LLESSOP, false);
compare_op!(GreaterThan, LGREATEROP, false);
compare_op!(LogicalNand, LANDOP, true);
compare_op!(LogicalNor, LOROP, true);
compare_op!(NotEqual, LEQUALOP, true);
compare_op!(GreaterEqual, LLESSOP, true);
compare_op!(LessEqual, LGREATEROP, true);

/// Argx object.
pub struct Arg(pub u8);

impl Aml for Arg {
    /// Per ACPI spec, there is maximum 7 Argx objects from
    /// Arg0 ~ Arg6. Any other Arg object will not be accepted.
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        assert!(self.0 <= 6);
        sink.byte(ARG0OP + self.0);
    }
}

/// Localx object.
pub struct Local(pub u8);

impl Aml for Local {
    /// Per ACPI spec, there is maximum 8 Localx objects from
    /// Local0 ~ Local7. Any other Local object will not be accepted.
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        assert!(self.0 <= 7);
        sink.byte(LOCAL0OP + self.0);
    }
}

/// Store object with the ACPI object name which can be stored to and
/// the ACPI object value which is to store.
pub struct Store<'a> {
    name: &'a dyn Aml,
    value: &'a dyn Aml,
}

impl<'a> Store<'a> {
    /// Create Store object.
    pub fn new(name: &'a dyn Aml, value: &'a dyn Aml) -> Self {
        Store { name, value }
    }
}

impl Aml for Store<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(STOREOP);
        self.value.to_aml_bytes(sink);
        self.name.to_aml_bytes(sink);
    }
}

/// Conditionally create a reference to an object if it exists.
pub struct CondRefOf<'a> {
    source: &'a dyn Aml,
    target: &'a dyn Aml,
}

impl<'a> CondRefOf<'a> {
    /// Create a conditional reference from `source` into `target`.
    pub fn new(source: &'a dyn Aml, target: &'a dyn Aml) -> Self {
        Self { source, target }
    }
}

impl Aml for CondRefOf<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(CONDREFOFOP);
        self.source.to_aml_bytes(sink);
        self.target.to_aml_bytes(sink);
    }
}

macro_rules! extended_object_op {
    ($name:ident, $opcode:expr) => {
        /// Extended operation on an AML object.
        pub struct $name<'a> {
            object: &'a dyn Aml,
        }

        impl<'a> $name<'a> {
            /// Create the extended object operation.
            pub fn new(object: &'a dyn Aml) -> Self {
                Self { object }
            }
        }

        impl Aml for $name<'_> {
            fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
                sink.byte(EXTOPPREFIX);
                sink.byte($opcode);
                self.object.to_aml_bytes(sink);
            }
        }
    };
}

extended_object_op!(Stall, STALLOP);
extended_object_op!(Sleep, SLEEPOP);

/// Mutex object with a mutex name and a synchronization level.
pub struct Mutex {
    path: Path,
    sync_level: u8,
}

impl Mutex {
    /// Create Mutex object.
    pub fn new(path: Path, sync_level: u8) -> Self {
        Self { path, sync_level }
    }
}

impl Aml for Mutex {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(MUTEXOP);
        self.path.to_aml_bytes(sink);
        sink.byte(self.sync_level);
    }
}

/// Acquire object with a Mutex object and timeout value.
pub struct Acquire {
    mutex: Path,
    timeout: u16,
}

impl Acquire {
    /// Create Acquire object.
    pub fn new(mutex: Path, timeout: u16) -> Self {
        Acquire { mutex, timeout }
    }
}

impl Aml for Acquire {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(ACQUIREOP);
        self.mutex.to_aml_bytes(sink);
        sink.word(self.timeout);
    }
}

/// Release object with a Mutex object to release.
pub struct Release {
    mutex: Path,
}

impl Release {
    /// Create Release object.
    pub fn new(mutex: Path) -> Self {
        Release { mutex }
    }
}

impl Aml for Release {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(RELEASEOP);
        self.mutex.to_aml_bytes(sink);
    }
}

/// Notify object with an object which is to be notified with the value.
pub struct Notify<'a> {
    object: &'a dyn Aml,
    value: &'a dyn Aml,
}

impl<'a> Notify<'a> {
    /// Create Notify object.
    pub fn new(object: &'a dyn Aml, value: &'a dyn Aml) -> Self {
        Notify { object, value }
    }
}

impl Aml for Notify<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(NOTIFYOP);
        self.object.to_aml_bytes(sink);
        self.value.to_aml_bytes(sink);
    }
}

/// While object with the while condition objects(predicate) and
/// the while body objects(while_children).
pub struct While<'a> {
    predicate: &'a dyn Aml,
    while_children: Vec<&'a dyn Aml>,
}

impl<'a> While<'a> {
    /// Create While object.
    pub fn new(predicate: &'a dyn Aml, while_children: Vec<&'a dyn Aml>) -> Self {
        While {
            predicate,
            while_children,
        }
    }
}

impl Aml for While<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.predicate.to_aml_bytes(&mut bytes);
        for child in self.while_children.iter() {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(WHILEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// Terminate the innermost enclosing [`While`] loop.
pub struct Break;

impl Aml for Break {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(BREAKOP);
    }
}

macro_rules! object_op {
    ($name:ident, $opcode:expr) => {
        /// General operation on a object.
        pub struct $name<'a> {
            a: &'a dyn Aml,
        }

        impl<'a> $name<'a> {
            /// Create the object method.
            pub fn new(a: &'a dyn Aml) -> Self {
                $name { a }
            }
        }

        impl<'a> Aml for $name<'a> {
            fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
                sink.byte($opcode);
                self.a.to_aml_bytes(sink);
            }
        }
    };
}

object_op!(RefOf, REFOFOP);
object_op!(Increment, INCREMENTOP);
object_op!(Decrement, DECREMENTOP);
object_op!(ObjectType, OBJECTTYPEOP);
object_op!(SizeOf, SIZEOFOP);
object_op!(Return, RETURNOP);
object_op!(DeRefOf, DEREFOFOP);
object_op!(LogicalNot, LNOTOP);

macro_rules! binary_op {
    ($name:ident, $opcode:expr) => {
        /// General operation object with the operator a/b and a target.
        pub struct $name<'a> {
            a: &'a dyn Aml,
            b: &'a dyn Aml,
            target: &'a dyn Aml,
        }

        impl<'a> $name<'a> {
            /// Create the object.
            pub fn new(target: &'a dyn Aml, a: &'a dyn Aml, b: &'a dyn Aml) -> Self {
                $name { target, a, b }
            }
        }

        impl<'a> Aml for $name<'a> {
            fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
                sink.byte($opcode); /* Op for the binary operator */
                self.a.to_aml_bytes(sink);
                self.b.to_aml_bytes(sink);
                self.target.to_aml_bytes(sink);
            }
        }
    };
}

binary_op!(Add, ADDOP);
binary_op!(Concat, CONCATOP);
binary_op!(Subtract, SUBTRACTOP);
binary_op!(Multiply, MULTIPLYOP);
binary_op!(ShiftLeft, SHIFTLEFTOP);
binary_op!(ShiftRight, SHIFTRIGHTOP);
binary_op!(And, ANDOP);
binary_op!(Nand, NANDOP);
binary_op!(Or, OROP);
binary_op!(Nor, NOROP);
binary_op!(Xor, XOROP);
binary_op!(ConcatRes, CONCATRESOP);
binary_op!(Mod, MODOP);
binary_op!(Index, INDEXOP);
binary_op!(ToString, TOSTRINGOP);
binary_op!(CreateDWordField, CREATEDWFIELDOP);
binary_op!(CreateQWordField, CREATEQWFIELDOP);

/// Divide two AML integers and store both the remainder and quotient.
pub struct Divide<'a> {
    dividend: &'a dyn Aml,
    divisor: &'a dyn Aml,
    remainder: &'a dyn Aml,
    quotient: &'a dyn Aml,
}

impl<'a> Divide<'a> {
    /// Create a divide operation.
    pub fn new(
        dividend: &'a dyn Aml,
        divisor: &'a dyn Aml,
        remainder: &'a dyn Aml,
        quotient: &'a dyn Aml,
    ) -> Self {
        Self {
            dividend,
            divisor,
            remainder,
            quotient,
        }
    }
}

impl Aml for Divide<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(DIVIDEOP);
        self.dividend.to_aml_bytes(sink);
        self.divisor.to_aml_bytes(sink);
        self.remainder.to_aml_bytes(sink);
        self.quotient.to_aml_bytes(sink);
    }
}

macro_rules! convert_op {
    ($name:ident, $opcode:expr) => {
        /// General operation object with the operator a/b and a target.
        pub struct $name<'a> {
            a: &'a dyn Aml,
            target: &'a dyn Aml,
        }

        impl<'a> $name<'a> {
            /// Create the object.
            pub fn new(target: &'a dyn Aml, a: &'a dyn Aml) -> Self {
                $name { target, a }
            }
        }

        impl<'a> Aml for $name<'a> {
            fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
                sink.byte($opcode); /* Op for the binary operator */
                self.a.to_aml_bytes(sink);
                self.target.to_aml_bytes(sink);
            }
        }
    };
}

convert_op!(ToBuffer, TOBUFFEROP);
convert_op!(ToInteger, TOINTEGEROP);

/// Create Field Object.
pub struct CreateField<'a> {
    name_string: &'a dyn Aml,
    source: &'a dyn Aml,
    bit_index: &'a dyn Aml,
    bit_num: &'a dyn Aml,
}

impl<'a> CreateField<'a> {
    pub fn new(
        name_string: &'a dyn Aml,
        source: &'a dyn Aml,
        bit_index: &'a dyn Aml,
        bit_num: &'a dyn Aml,
    ) -> Self {
        CreateField {
            name_string,
            source,
            bit_index,
            bit_num,
        }
    }
}

impl Aml for CreateField<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(EXTOPPREFIX);
        sink.byte(CREATEFIELDOP);
        self.source.to_aml_bytes(sink);
        self.bit_index.to_aml_bytes(sink);
        self.bit_num.to_aml_bytes(sink);
        self.name_string.to_aml_bytes(sink);
    }
}

/// Mid object with the source, index, length, and result objects.
pub struct Mid<'a> {
    source: &'a dyn Aml,
    index: &'a dyn Aml,
    length: &'a dyn Aml,
    result: &'a dyn Aml,
}

impl<'a> Mid<'a> {
    pub fn new(
        source: &'a dyn Aml,
        index: &'a dyn Aml,
        length: &'a dyn Aml,
        result: &'a dyn Aml,
    ) -> Self {
        Mid {
            source,
            index,
            length,
            result,
        }
    }
}

impl Aml for Mid<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        sink.byte(MIDOP);
        self.source.to_aml_bytes(sink);
        self.index.to_aml_bytes(sink);
        self.length.to_aml_bytes(sink);
        self.result.to_aml_bytes(sink);
    }
}

/// MethodCall object with the method name and parameter objects.
pub struct MethodCall<'a> {
    name: Path,
    args: Vec<&'a dyn Aml>,
}

impl<'a> MethodCall<'a> {
    /// Create MethodCall object.
    pub fn new(name: Path, args: Vec<&'a dyn Aml>) -> Self {
        MethodCall { name, args }
    }
}

impl Aml for MethodCall<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.name.to_aml_bytes(sink);
        for arg in self.args.iter() {
            arg.to_aml_bytes(sink);
        }
    }
}

/// Buffer object with the TermArg in it.
pub struct BufferTerm<'a> {
    data: &'a dyn Aml,
}

impl<'a> BufferTerm<'a> {
    /// Create BufferTerm object.
    pub fn new(data: &'a dyn Aml) -> Self {
        BufferTerm { data }
    }
}

impl Aml for BufferTerm<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.data.to_aml_bytes(&mut bytes);

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(BUFFEROP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// Buffer object with the data in it.
pub struct BufferData {
    data: Vec<u8>,
}

impl BufferData {
    /// Create BufferData object.
    pub fn new(data: Vec<u8>) -> Self {
        BufferData { data }
    }
}

impl Aml for BufferData {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.data.len().to_aml_bytes(&mut bytes);
        bytes.extend_from_slice(&self.data);

        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(BUFFEROP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

pub struct Uuid {
    name: BufferData,
}

fn hex2byte(v1: char, v2: char) -> u8 {
    let hi = v1.to_digit(16).unwrap() as u8;
    assert!(hi <= 15);
    let lo = v2.to_digit(16).unwrap() as u8;
    assert!(lo <= 15);

    (hi << 4) | lo
}

impl Uuid {
    // Create Uuid object
    // eg. UUID: aabbccdd-eeff-gghh-iijj-kkllmmnnoopp
    pub fn new(name: &str) -> Self {
        let name_vec: Vec<char> = name.chars().collect();
        let mut data = Vec::new();

        assert_eq!(name_vec.len(), 36);
        assert_eq!(name_vec[8], '-');
        assert_eq!(name_vec[13], '-');
        assert_eq!(name_vec[18], '-');
        assert_eq!(name_vec[23], '-');

        // dd - at offset 00
        data.push(hex2byte(name_vec[6], name_vec[7]));
        // cc - at offset 01
        data.push(hex2byte(name_vec[4], name_vec[5]));
        // bb - at offset 02
        data.push(hex2byte(name_vec[2], name_vec[3]));
        // aa - at offset 03
        data.push(hex2byte(name_vec[0], name_vec[1]));

        // ff - at offset 04
        data.push(hex2byte(name_vec[11], name_vec[12]));
        // ee - at offset 05
        data.push(hex2byte(name_vec[9], name_vec[10]));

        // hh - at offset 06
        data.push(hex2byte(name_vec[16], name_vec[17]));
        // gg - at offset 07
        data.push(hex2byte(name_vec[14], name_vec[15]));

        // ii - at offset 08
        data.push(hex2byte(name_vec[19], name_vec[20]));
        // jj - at offset 09
        data.push(hex2byte(name_vec[21], name_vec[22]));

        // kk - at offset 10
        data.push(hex2byte(name_vec[24], name_vec[25]));
        // ll - at offset 11
        data.push(hex2byte(name_vec[26], name_vec[27]));
        // mm - at offset 12
        data.push(hex2byte(name_vec[28], name_vec[29]));
        // nn - at offset 13
        data.push(hex2byte(name_vec[30], name_vec[31]));
        // oo - at offset 14
        data.push(hex2byte(name_vec[32], name_vec[33]));
        // pp - at offset 15
        data.push(hex2byte(name_vec[34], name_vec[35]));

        Uuid {
            name: BufferData::new(data),
        }
    }
}

impl Aml for Uuid {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        self.name.to_aml_bytes(sink)
    }
}

/// Thermal Zone object containing thermal control methods and values.
pub struct ThermalZone<'a> {
    name: Path,
    children: Vec<&'a dyn Aml>,
}

impl<'a> ThermalZone<'a> {
    /// Create a Thermal Zone object.
    pub fn new(name: Path, children: Vec<&'a dyn Aml>) -> Self {
        Self { name, children }
    }
}

impl Aml for ThermalZone<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();
        self.name.to_aml_bytes(&mut bytes);
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        let pkg_length = create_pkg_length(bytes.len(), true);
        sink.byte(EXTOPPREFIX);
        sink.byte(THERMALZONEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

/// Power Resource object. 'children' represents Power Resource method.
pub struct PowerResource<'a> {
    name: Path,
    level: u8,
    order: u16,
    children: Vec<&'a dyn Aml>,
}

impl<'a> PowerResource<'a> {
    /// Create Power Resouce object
    pub fn new(name: Path, level: u8, order: u16, children: Vec<&'a dyn Aml>) -> Self {
        PowerResource {
            name,
            level,
            order,
            children,
        }
    }
}

impl Aml for PowerResource<'_> {
    fn to_aml_bytes(&self, sink: &mut dyn AmlSink) {
        let mut bytes = Vec::new();

        // Add name string
        self.name.to_aml_bytes(&mut bytes);
        // Add system level
        bytes.push(self.level);
        // Add Resource Order
        let orders = self.order.to_le_bytes();
        bytes.push(orders[0]);
        bytes.push(orders[1]);
        // Add child data
        for child in &self.children {
            child.to_aml_bytes(&mut bytes);
        }

        // PkgLength
        let pkg_length = create_pkg_length(bytes.len(), true);

        sink.byte(EXTOPPREFIX);
        sink.byte(POWERRESOURCEOP);
        sink.vec(&pkg_length);
        sink.vec(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;

    #[test]
    fn test_device() {
        /*
        Device (_SB.COM1)
        {
            Name (_HID, EisaId ("PNP0501") /* 16550A-compatible COM Serial Port */) // _HID: Hardware ID
            Name (_CRS, ResourceTemplate ()  // _CRS: Current Resource Settings
            {
                Interrupt (ResourceConsumer, Edge, ActiveHigh, Exclusive, ,, )
                {
                    0x00000004,
                }
                IO (Decode16,
                    0x03F8,             // Range Minimum
                    0x03F8,             // Range Maximum
                    0x00,               // Alignment
                    0x08,               // Length
                    )
            }
        }
            */
        let com1_device = [
            0x5B, 0x82, 0x30, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x43, 0x4F, 0x4D, 0x31, 0x08, 0x5F,
            0x48, 0x49, 0x44, 0x0C, 0x41, 0xD0, 0x05, 0x01, 0x08, 0x5F, 0x43, 0x52, 0x53, 0x11,
            0x16, 0x0A, 0x13, 0x89, 0x06, 0x00, 0x03, 0x01, 0x04, 0x00, 0x00, 0x00, 0x47, 0x01,
            0xF8, 0x03, 0xF8, 0x03, 0x00, 0x08, 0x79, 0x00,
        ];
        let mut aml = Vec::new();

        Device::new(
            "_SB_.COM1".into(),
            vec![
                &Name::new("_HID".into(), &EISAName::new("PNP0501")),
                &Name::new(
                    "_CRS".into(),
                    &ResourceTemplate::new(vec![
                        &Interrupt::new(true, true, false, false, 4),
                        &IO::new(0x3f8, 0x3f8, 0, 0x8),
                    ]),
                ),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &com1_device[..]);
    }

    #[test]
    fn test_scope() {
        /*
        Scope (_SB.MBRD)
        {
            Name (_CRS, ResourceTemplate ()  // _CRS: Current Resource Settings
            {
                Memory32Fixed (ReadWrite,
                    0xE8000000,         // Address Base
                    0x10000000,         // Address Length
                    )
            })
        }
        */

        let mbrd_scope = [
            0x10, 0x21, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x4D, 0x42, 0x52, 0x44, 0x08, 0x5F, 0x43,
            0x52, 0x53, 0x11, 0x11, 0x0A, 0x0E, 0x86, 0x09, 0x00, 0x01, 0x00, 0x00, 0x00, 0xE8,
            0x00, 0x00, 0x00, 0x10, 0x79, 0x00,
        ];
        let mut aml = Vec::new();

        Scope::new(
            "_SB_.MBRD".into(),
            vec![&Name::new(
                "_CRS".into(),
                &ResourceTemplate::new(vec![&Memory32Fixed::new(true, 0xE800_0000, 0x1000_0000)]),
            )],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &mbrd_scope[..]);
    }

    #[test]
    fn test_scope_raw() {
        let scope = [
            0x10, 0x0E, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x4D, 0x42, 0x52, 0x44, 0xAA, 0xBB, 0xCC,
            0xDD,
        ];
        let bytes = Scope::raw("_SB_.MBRD".into(), vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(bytes, scope);
    }

    #[test]
    fn test_gpio_connection_descriptors() {
        let gpio_int = GpioInt {
            resource_source: "\\_SB.GPI0",
            pins: &[42],
            consumer: true,
            mode: GpioIntMode::Edge,
            polarity: GpioIntPolarity::ActiveLow,
            shared: false,
            pin_config: GpioPinConfig::PullUp,
            debounce: 0,
        };
        let gpio_io = GpioIo {
            resource_source: "\\_SB.GPI0",
            pins: &[10, 11],
            consumer: true,
            io_restriction: GpioIoRestriction::None,
            pin_config: GpioPinConfig::Default,
            drive_strength: 0,
            debounce: 0,
        };

        let mut bytes = Vec::new();
        gpio_int.to_aml_bytes(&mut bytes);
        assert_eq!(
            bytes,
            b"\x8c\x20\x00\x01\x00\x01\x00\x03\x00\x01\x00\x00\x00\x00\x17\x00\x00\x19\x00\x23\x00\x00\x00\x2a\x00\\_SB.GPI0\x00"
        );

        bytes.clear();
        gpio_io.to_aml_bytes(&mut bytes);
        assert_eq!(
            bytes,
            b"\x8c\x22\x00\x01\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x17\x00\x00\x1b\x00\x25\x00\x00\x00\x0a\x00\x0b\x00\\_SB.GPI0\x00"
        );
    }

    #[test]
    #[should_panic]
    fn test_gpio_connection_descriptor_rejects_offset_overflow() {
        let resource_source = "a".repeat(u16::MAX as usize - 1);
        GpioInt {
            resource_source: &resource_source,
            pins: &[],
            consumer: true,
            mode: GpioIntMode::Level,
            polarity: GpioIntPolarity::ActiveHigh,
            shared: false,
            pin_config: GpioPinConfig::Default,
            debounce: 0,
        }
        .to_aml_bytes(&mut Vec::new());
    }

    #[test]
    fn test_serial_bus_connection_descriptors() {
        let i2c = I2cSerialBus {
            resource_source: "\\_SB.I2C0",
            slave_address: 0x50,
            connection_speed: 400_000,
            address_10bit: false,
            consumer: true,
        };
        let spi = SpiSerialBus {
            resource_source: "\\_SB.SPI0",
            connection_speed: 10_000_000,
            data_bit_length: 8,
            clock_phase: SpiClockPhase::First,
            clock_polarity: SpiClockPolarity::Low,
            wire_mode: SpiWireMode::FourWire,
            device_polarity: SpiDevicePolarity::ActiveLow,
            device_selection: 0,
            consumer: true,
        };

        let mut bytes = Vec::new();
        i2c.to_aml_bytes(&mut bytes);
        assert_eq!(
            bytes,
            b"\x8e\x19\x00\x01\x00\x01\x02\x00\x00\x01\x06\x00\x80\x1a\x06\x00\x50\x00\\_SB.I2C0\x00"
        );

        bytes.clear();
        spi.to_aml_bytes(&mut bytes);
        assert_eq!(
            bytes,
            b"\x8e\x1c\x00\x01\x00\x02\x02\x00\x00\x01\x09\x00\x80\x96\x98\x00\x08\x00\x00\x00\x00\\_SB.SPI0\x00"
        );
    }

    #[test]
    fn test_resource_template() {
        /*
        Name (_CRS, ResourceTemplate ()  // _CRS: Current Resource Settings
        {
            Memory32Fixed (ReadWrite,
                0xE8000000,         // Address Base
                0x10000000,         // Address Length
                )
        })
        */
        let crs_memory_32_fixed = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x11, 0x0A, 0x0E, 0x86, 0x09, 0x00, 0x01, 0x00,
            0x00, 0x00, 0xE8, 0x00, 0x00, 0x00, 0x10, 0x79, 0x00,
        ];
        let mut aml = Vec::new();

        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![&Memory32Fixed::new(true, 0xE800_0000, 0x1000_0000)]),
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, crs_memory_32_fixed);

        /*
            Name (_CRS, ResourceTemplate ()  // _CRS: Current Resource Settings
            {
                WordBusNumber (ResourceProducer, MinFixed, MaxFixed, PosDecode,
                    0x0000,             // Granularity
                    0x0000,             // Range Minimum
                    0x00FF,             // Range Maximum
                    0x0000,             // Translation Offset
                    0x0100,             // Length
                    ,, )
                WordIO (ResourceProducer, MinFixed, MaxFixed, PosDecode, EntireRange,
                    0x0000,             // Granularity
                    0x0000,             // Range Minimum
                    0x0CF7,             // Range Maximum
                    0x0000,             // Translation Offset
                    0x0CF8,             // Length
                    ,, , TypeStatic, DenseTranslation)
                WordIO (ResourceProducer, MinFixed, MaxFixed, PosDecode, EntireRange,
                    0x0000,             // Granularity
                    0x0D00,             // Range Minimum
                    0xFFFF,             // Range Maximum
                    0x0000,             // Translation Offset
                    0xF300,             // Length
                    ,, , TypeStatic, DenseTranslation)
                DWordMemory (ResourceProducer, PosDecode, MinFixed, MaxFixed, Cacheable, ReadWrite,
                    0x00000000,         // Granularity
                    0x000A0000,         // Range Minimum
                    0x000BFFFF,         // Range Maximum
                    0x00000000,         // Translation Offset
                    0x00020000,         // Length
                    ,, , AddressRangeMemory, TypeStatic)
                DWordMemory (ResourceProducer, PosDecode, MinFixed, MaxFixed, NonCacheable, ReadWrite,
                    0x00000000,         // Granularity
                    0xC0000000,         // Range Minimum
                    0xFEBFFFFF,         // Range Maximum
                    0x00000000,         // Translation Offset
                    0x3EC00000,         // Length
                    ,, , AddressRangeMemory, TypeStatic)
                QWordMemory (ResourceProducer, PosDecode, MinFixed, MaxFixed, Cacheable, ReadWrite,
                    0x0000000000000000, // Granularity
                    0x0000000800000000, // Range Minimum
                    0x0000000FFFFFFFFF, // Range Maximum
                    0x0000000000000000, // Translation Offset
                    0x0000000800000000, // Length
                    ,, , AddressRangeMemory, TypeStatic)
            })
        */

        // WordBusNumber from above
        let crs_word_bus_number = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x15, 0x0A, 0x12, 0x88, 0x0D, 0x00, 0x02, 0x0C,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x01, 0x79, 0x00,
        ];
        aml.clear();

        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![&AddressSpace::new_bus_number(0x0u16, 0xffu16)]),
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &crs_word_bus_number);

        // WordIO blocks (x 2) from above
        let crs_word_io = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x25, 0x0A, 0x22, 0x88, 0x0D, 0x00, 0x01, 0x0C,
            0x03, 0x00, 0x00, 0x00, 0x00, 0xF7, 0x0C, 0x00, 0x00, 0xF8, 0x0C, 0x88, 0x0D, 0x00,
            0x01, 0x0C, 0x03, 0x00, 0x00, 0x00, 0x0D, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xF3, 0x79,
            0x00,
        ];
        aml.clear();

        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![
                &AddressSpace::new_io(0x0u16, 0xcf7u16, None),
                &AddressSpace::new_io(0xd00u16, 0xffffu16, None),
            ]),
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &crs_word_io[..]);

        // DWordMemory blocks (x 2) from above
        let crs_dword_memory = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x39, 0x0A, 0x36, 0x87, 0x17, 0x00, 0x00, 0x0C,
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0x00, 0xFF, 0xFF, 0x0B, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x87, 0x17, 0x00, 0x00, 0x0C, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0xFF, 0xFF, 0xBF, 0xFE, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xC0, 0x3E, 0x79, 0x00,
        ];
        aml.clear();

        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![
                &AddressSpace::new_memory(
                    AddressSpaceCacheable::Cacheable,
                    true,
                    0xa_0000u32,
                    0xb_ffffu32,
                    None,
                ),
                &AddressSpace::new_memory(
                    AddressSpaceCacheable::NotCacheable,
                    true,
                    0xc000_0000u32,
                    0xfebf_ffffu32,
                    None,
                ),
            ]),
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &crs_dword_memory[..]);

        // QWordMemory from above
        let crs_qword_memory = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x33, 0x0A, 0x30, 0x8A, 0x2B, 0x00, 0x00, 0x0C,
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x79,
            0x00,
        ];
        aml.clear();
        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![&AddressSpace::new_memory(
                AddressSpaceCacheable::Cacheable,
                true,
                0x8_0000_0000u64,
                0xf_ffff_ffffu64,
                None,
            )]),
        )
        .to_aml_bytes(&mut aml);

        assert_eq!(aml, &crs_qword_memory[..]);

        /*
            Name (_CRS, ResourceTemplate ()  // _CRS: Current Resource Settings
            {
                Interrupt (ResourceConsumer, Edge, ActiveHigh, Exclusive, ,, )
                {
                    0x00000004,
                }
                Interrupt (ResourceConsumer, Edge, ActiveHigh, Exclusive, ,, )
                {
                    0x00000005,
                    0x00000006,
                }
                IRQ (Edge, ActiveHigh, Exclusive, )
                    {7}
                IRQNoFlags ()
                    {8}
                IO (Decode16,
                    0x03F8,             // Range Minimum
                    0x03F8,             // Range Maximum
                    0x00,               // Alignment
                    0x08,               // Length
                    )
            })

        */
        let interrupt_io_data = [
            0x08, 0x5F, 0x43, 0x52, 0x53, 0x11, 0x2A, 0x0A, 0x27, 0x89, 0x06, 0x00, 0x03, 0x01,
            0x04, 0x00, 0x00, 0x00, 0x89, 0x0A, 0x00, 0x03, 0x02, 0x05, 0x00, 0x00, 0x00, 0x06,
            0x00, 0x00, 0x00, 0x23, 0x80, 0x00, 0x01, 0x22, 0x00, 0x01, 0x47, 0x01, 0xF8, 0x03,
            0xF8, 0x03, 0x00, 0x08, 0x79, 0x00,
        ];
        aml.clear();
        Name::new(
            "_CRS".into(),
            &ResourceTemplate::new(vec![
                &Interrupt::new(true, true, false, false, 4),
                &Interrupt::new_multiple(true, true, false, false, vec![5, 6]),
                &Irq::new(true, false, false, 7),
                &IrqNoFlags::new(8),
                &IO::new(0x3f8, 0x3f8, 0, 0x8),
            ]),
        )
        .to_aml_bytes(&mut aml);

        assert_eq!(aml, &interrupt_io_data[..]);
    }

    #[test]
    fn test_pkg_length() {
        assert_eq!(create_pkg_length(62, true), vec![63]);
        assert_eq!(
            create_pkg_length(64, true),
            vec![(1 << 6) | (66 & 0xf), 66 >> 4]
        );
        assert_eq!(
            create_pkg_length(4096, true),
            vec![
                (2 << 6) | (4099 & 0xf) as u8,
                (4099 >> 4) as u8,
                (4099 >> 12) as u8
            ]
        );
    }

    #[test]
    fn test_package() {
        /*
        Name (_S5, Package (0x01)  // _S5_: S5 System State
        {
            0x05
        })
        */
        let s5_sleep_data = [0x08, 0x5F, 0x53, 0x35, 0x5F, 0x12, 0x04, 0x01, 0x0A, 0x05];
        let mut aml = Vec::new();

        Name::new("_S5_".into(), &Package::new(vec![&5u8])).to_aml_bytes(&mut aml);

        assert_eq!(s5_sleep_data.to_vec(), aml);
    }

    #[test]
    fn test_eisa_name() {
        let mut aml = Vec::new();
        Name::new("_HID".into(), &EISAName::new("PNP0501")).to_aml_bytes(&mut aml);
        assert_eq!(
            aml,
            [0x08, 0x5F, 0x48, 0x49, 0x44, 0x0C, 0x41, 0xD0, 0x05, 0x01],
        )
    }
    #[test]
    fn test_name_path() {
        let mut aml = Vec::new();
        (&"_SB_".into() as &Path).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x5Fu8, 0x53, 0x42, 0x5F]);
        aml.clear();
        (&"\\_SB_".into() as &Path).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x5C, 0x5F, 0x53, 0x42, 0x5F]);
        aml.clear();
        (&"^_SB_".into() as &Path).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x5E, 0x5F, 0x53, 0x42, 0x5F]);
        aml.clear();
        (&"_SB_.COM1".into() as &Path).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x43, 0x4F, 0x4D, 0x31]);
        aml.clear();
        (&"_SB_.PCI0._HID".into() as &Path).to_aml_bytes(&mut aml);
        assert_eq!(
            aml,
            [0x2F, 0x03, 0x5F, 0x53, 0x42, 0x5F, 0x50, 0x43, 0x49, 0x30, 0x5F, 0x48, 0x49, 0x44]
        );
    }

    #[test]
    fn test_numbers() {
        let mut aml = Vec::new();
        128u8.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x80]);
        aml.clear();
        1024u16.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0b, 0x0, 0x04]);
        aml.clear();
        (16u32 << 20).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0c, 0x00, 0x00, 0x0, 0x01]);
        aml.clear();
        0xdeca_fbad_deca_fbadu64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0e, 0xad, 0xfb, 0xca, 0xde, 0xad, 0xfb, 0xca, 0xde]);
        aml.clear();

        // u8
        0x00_u8.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x00]);
        aml.clear();
        0x01_u8.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x01]);
        aml.clear();
        0x86_u8.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x86]);
        aml.clear();

        // u16
        0x00_u16.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x00]);
        aml.clear();
        0x01_u16.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x01]);
        aml.clear();
        0x86_u16.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x86]);
        aml.clear();
        0xF00D_u16.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0b, 0x0d, 0xf0]);
        aml.clear();

        // u32
        0x00_u32.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x00]);
        aml.clear();
        0x01_u32.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x01]);
        aml.clear();
        0x86_u32.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x86]);
        aml.clear();
        0xF00D_u32.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0b, 0x0d, 0xf0]);
        aml.clear();
        0xDECAF_u32.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0c, 0xaf, 0xec, 0x0d, 0x00]);
        aml.clear();

        // u64
        0x00_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x00]);
        aml.clear();
        0x01_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x01]);
        aml.clear();
        0x86_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x86]);
        aml.clear();
        0xF00D_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0b, 0x0d, 0xf0]);
        aml.clear();
        0xDECAF_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0c, 0xaf, 0xec, 0x0d, 0x00]);
        aml.clear();
        0xDECAFC0FFEE_u64.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0e, 0xee, 0xff, 0xc0, 0xaf, 0xec, 0x0d, 0x00, 0x00]);
        aml.clear();

        // usize
        0x00_usize.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x00]);
        aml.clear();
        0x01_usize.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x01]);
        aml.clear();
        0x86_usize.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0a, 0x86]);
        aml.clear();
        0xF00D_usize.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0b, 0x0d, 0xf0]);
        aml.clear();
        0xDECAF_usize.to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0c, 0xaf, 0xec, 0x0d, 0x00]);
        aml.clear();
        #[cfg(target_pointer_width = "64")]
        {
            0xDECAFC0FFEE_usize.to_aml_bytes(&mut aml);
            assert_eq!(aml, [0x0e, 0xee, 0xff, 0xc0, 0xaf, 0xec, 0x0d, 0x00, 0x00]);
            aml.clear();
        }
    }

    #[test]
    fn test_name() {
        let mut aml = Vec::new();
        Name::new("_SB_.PCI0._UID".into(), &0x1234u16).to_aml_bytes(&mut aml);
        assert_eq!(
            aml,
            [
                0x08, /* NameOp */
                0x2F, /* MultiNamePrefix */
                0x03, /* 3 name parts */
                0x5F, 0x53, 0x42, 0x5F, /* _SB_ */
                0x50, 0x43, 0x49, 0x30, /* PCI0 */
                0x5F, 0x55, 0x49, 0x44, /* _UID  */
                0x0b, /* WordPrefix */
                0x34, 0x12
            ]
        );
    }

    #[test]
    fn test_string() {
        let mut aml = Vec::new();
        (&"ACPI" as &dyn Aml).to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0d, b'A', b'C', b'P', b'I', 0]);
        aml.clear();
        "ACPI".to_owned().to_aml_bytes(&mut aml);
        assert_eq!(aml, [0x0d, b'A', b'C', b'P', b'I', 0]);
    }

    #[test]
    fn test_method() {
        let mut aml = Vec::new();
        Method::new("_STA".into(), 0, false, vec![&Return::new(&0xfu8)]).to_aml_bytes(&mut aml);
        assert_eq!(
            aml,
            [0x14, 0x09, 0x5F, 0x53, 0x54, 0x41, 0x00, 0xA4, 0x0A, 0x0F]
        );
    }

    #[test]
    fn test_field() {
        /*
            Field (PRST, ByteAcc, NoLock, WriteAsZeros)
            {
                Offset (0x04),
                CPEN,   1,
                CINS,   1,
                CRMV,   1,
                CEJ0,   1,
                Offset (0x05),
                CCMD,   8
            }

        */

        let field_data = [
            0x5Bu8, 0x81, 0x23, 0x50, 0x52, 0x53, 0x54, 0x41, 0x00, 0x20, 0x43, 0x50, 0x45, 0x4E,
            0x01, 0x43, 0x49, 0x4E, 0x53, 0x01, 0x43, 0x52, 0x4D, 0x56, 0x01, 0x43, 0x45, 0x4A,
            0x30, 0x01, 0x00, 0x04, 0x43, 0x43, 0x4D, 0x44, 0x08,
        ];
        let mut aml = Vec::new();

        Field::new(
            "PRST".into(),
            FieldAccessType::Byte,
            FieldLockRule::NoLock,
            FieldUpdateRule::WriteAsZeroes,
            vec![
                FieldEntry::Reserved(32),
                FieldEntry::Named(*b"CPEN", 1),
                FieldEntry::Named(*b"CINS", 1),
                FieldEntry::Named(*b"CRMV", 1),
                FieldEntry::Named(*b"CEJ0", 1),
                FieldEntry::Reserved(4),
                FieldEntry::Named(*b"CCMD", 8),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &field_data[..]);

        /*
            Field (PRST, DWordAcc, Lock, Preserve)
            {
                CSEL,   32,
                Offset (0x08),
                CDAT,   32
            }
        */

        let field_data = [
            0x5Bu8, 0x81, 0x12, 0x50, 0x52, 0x53, 0x54, 0x13, 0x43, 0x53, 0x45, 0x4C, 0x20, 0x00,
            0x20, 0x43, 0x44, 0x41, 0x54, 0x20,
        ];
        aml.clear();

        Field::new(
            "PRST".into(),
            FieldAccessType::DWord,
            FieldLockRule::Lock,
            FieldUpdateRule::Preserve,
            vec![
                FieldEntry::Named(*b"CSEL", 32),
                FieldEntry::Reserved(32),
                FieldEntry::Named(*b"CDAT", 32),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &field_data[..]);
    }

    #[test]
    fn test_op_region() {
        /*
            OperationRegion (PRST, SystemIO, 0x0CD8, 0x0C)
        */
        let op_region_data = [
            0x5Bu8, 0x80, 0x50, 0x52, 0x53, 0x54, 0x01, 0x0B, 0xD8, 0x0C, 0x0A, 0x0C,
        ];
        let mut aml = Vec::new();

        OpRegion::new(
            "PRST".into(),
            OpRegionSpace::SystemIO,
            &0xcd8_usize,
            &0xc_usize,
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &op_region_data[..]);
    }

    #[test]
    fn test_arg_if() {
        /*
            Method(TEST, 1, NotSerialized) {
                If (Arg0 == Zero) {
                        Return(One)
                }
                Return(Zero)
            }
        */
        let arg_if_data = [
            0x14, 0x0F, 0x54, 0x45, 0x53, 0x54, 0x01, 0xA0, 0x06, 0x93, 0x68, 0x00, 0xA4, 0x01,
            0xA4, 0x00,
        ];
        let mut aml = Vec::new();

        Method::new(
            "TEST".into(),
            1,
            false,
            vec![
                &If::new(&Equal::new(&Arg(0), &ZERO), vec![&Return::new(&ONE)]),
                &Return::new(&ZERO),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &arg_if_data);
    }

    #[test]
    fn test_local_if() {
        /*
            Method(TEST, 0, NotSerialized) {
                Local0 = One
                If (Local0 == Zero) {
                        Return(One)
                }
                Return(Zero)
            }
        */
        let local_if_data = [
            0x14, 0x12, 0x54, 0x45, 0x53, 0x54, 0x00, 0x70, 0x01, 0x60, 0xA0, 0x06, 0x93, 0x60,
            0x00, 0xA4, 0x01, 0xA4, 0x00,
        ];
        let mut aml = Vec::new();

        Method::new(
            "TEST".into(),
            0,
            false,
            vec![
                &Store::new(&Local(0), &ONE),
                &If::new(&Equal::new(&Local(0), &ZERO), vec![&Return::new(&ONE)]),
                &Return::new(&ZERO),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &local_if_data);
    }

    #[test]
    fn test_compare_op() {
        let expected = [
            0x90, 0x00, 0x01, // 0 && 1
            0x91, 0x00, 0x01, // 0 || 1
            0x93, 0x00, 0x01, // 0 == 1
            0x94, 0x00, 0x01, // 0 > 1
            0x95, 0x00, 0x01, // 0 < 1
            0x92, 0x90, 0x00, 0x01, // !(0 && 1)
            0x92, 0x91, 0x00, 0x01, // !(0 || 1)
            0x92, 0x93, 0x00, 0x01, // 0 != 1
            0x92, 0x95, 0x00, 0x01, // 0 >= 1
            0x92, 0x94, 0x00, 0x01, // 0 <= 1
        ];

        let mut aml = Vec::new();

        LogicalAnd::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        LogicalOr::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        Equal::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        GreaterThan::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        LessThan::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        LogicalNand::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        LogicalNor::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        NotEqual::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        GreaterEqual::new(&ZERO, &ONE).to_aml_bytes(&mut aml);
        LessEqual::new(&ZERO, &ONE).to_aml_bytes(&mut aml);

        assert_eq!(aml, &expected[..]);
    }

    #[test]
    fn test_mutex() {
        /*
        Device (_SB_.MHPC)
        {
                Name (_HID, EisaId("PNP0A06") /* Generic Container Device */)  // _HID: Hardware ID
                Mutex (MLCK, 0x00)
                Method (TEST, 0, NotSerialized)
                {
                    Acquire (MLCK, 0xFFFF)
                    Local0 = One
                    Release (MLCK)
                }
        }
        */

        let mutex_data = [
            0x5B, 0x82, 0x33, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x4D, 0x48, 0x50, 0x43, 0x08, 0x5F,
            0x48, 0x49, 0x44, 0x0C, 0x41, 0xD0, 0x0A, 0x06, 0x5B, 0x01, 0x4D, 0x4C, 0x43, 0x4B,
            0x00, 0x14, 0x17, 0x54, 0x45, 0x53, 0x54, 0x00, 0x5B, 0x23, 0x4D, 0x4C, 0x43, 0x4B,
            0xFF, 0xFF, 0x70, 0x01, 0x60, 0x5B, 0x27, 0x4D, 0x4C, 0x43, 0x4B,
        ];
        let mut aml = Vec::new();

        let mutex = Mutex::new("MLCK".into(), 0);
        Device::new(
            "_SB_.MHPC".into(),
            vec![
                &Name::new("_HID".into(), &EISAName::new("PNP0A06")),
                &mutex,
                &Method::new(
                    "TEST".into(),
                    0,
                    false,
                    vec![
                        &Acquire::new("MLCK".into(), 0xffff),
                        &Store::new(&Local(0), &ONE),
                        &Release::new("MLCK".into()),
                    ],
                ),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &mutex_data[..]);
    }

    #[test]
    fn test_notify() {
        /*
        Device (_SB.MHPC)
        {
            Name (_HID, EisaId ("PNP0A06") /* Generic Container Device */)  // _HID: Hardware ID
            Method (TEST, 0, NotSerialized)
            {
                Notify (MHPC, One) // Device Check
            }
        }
        */
        let notify_data = [
            0x5B, 0x82, 0x21, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x4D, 0x48, 0x50, 0x43, 0x08, 0x5F,
            0x48, 0x49, 0x44, 0x0C, 0x41, 0xD0, 0x0A, 0x06, 0x14, 0x0C, 0x54, 0x45, 0x53, 0x54,
            0x00, 0x86, 0x4D, 0x48, 0x50, 0x43, 0x01,
        ];
        let mut aml = Vec::new();

        Device::new(
            "_SB_.MHPC".into(),
            vec![
                &Name::new("_HID".into(), &EISAName::new("PNP0A06")),
                &Method::new(
                    "TEST".into(),
                    0,
                    false,
                    vec![&Notify::new(&Path::new("MHPC"), &ONE)],
                ),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &notify_data[..]);
    }

    #[test]
    fn test_while() {
        /*
        Device (_SB.MHPC)
        {
            Name (_HID, EisaId ("PNP0A06") /* Generic Container Device */)  // _HID: Hardware ID
            Method (TEST, 0, NotSerialized)
            {
                Local0 = Zero
                While ((Local0 < 0x04))
                {
                    Local0 += One
                }
            }
        }
        */

        let while_data = [
            0x5B, 0x82, 0x28, 0x2E, 0x5F, 0x53, 0x42, 0x5F, 0x4D, 0x48, 0x50, 0x43, 0x08, 0x5F,
            0x48, 0x49, 0x44, 0x0C, 0x41, 0xD0, 0x0A, 0x06, 0x14, 0x13, 0x54, 0x45, 0x53, 0x54,
            0x00, 0x70, 0x00, 0x60, 0xA2, 0x09, 0x95, 0x60, 0x0A, 0x04, 0x72, 0x60, 0x01, 0x60,
        ];
        let mut aml = Vec::new();

        Device::new(
            "_SB_.MHPC".into(),
            vec![
                &Name::new("_HID".into(), &EISAName::new("PNP0A06")),
                &Method::new(
                    "TEST".into(),
                    0,
                    false,
                    vec![
                        &Store::new(&Local(0), &ZERO),
                        &While::new(
                            &LessThan::new(&Local(0), &4usize),
                            vec![&Add::new(&Local(0), &Local(0), &ONE)],
                        ),
                    ],
                ),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &while_data[..])
    }

    #[test]
    fn test_object_op() {
        let expected = [
            0x8e, 0x00, // ObjectType
            0x87, 0x00, // SizeOf
            0xa4, 0x00, // Return
            0x83, 0x00, // DeRefOf
            0x92, 0x00, // LogicalNot
        ];

        let mut aml = Vec::new();

        ObjectType::new(&ZERO).to_aml_bytes(&mut aml);
        SizeOf::new(&ZERO).to_aml_bytes(&mut aml);
        Return::new(&ZERO).to_aml_bytes(&mut aml);
        DeRefOf::new(&ZERO).to_aml_bytes(&mut aml);
        LogicalNot::new(&ZERO).to_aml_bytes(&mut aml);

        assert_eq!(aml, &expected[..]);
    }

    #[test]
    fn test_method_call() {
        /*
            Method (TST1, 1, NotSerialized)
            {
                TST2 (One, One)
            }

            Method (TST2, 2, NotSerialized)
            {
                TST1 (One)
            }
        */
        let test_data = [
            0x14, 0x0C, 0x54, 0x53, 0x54, 0x31, 0x01, 0x54, 0x53, 0x54, 0x32, 0x01, 0x01, 0x14,
            0x0B, 0x54, 0x53, 0x54, 0x32, 0x02, 0x54, 0x53, 0x54, 0x31, 0x01,
        ];

        let mut methods = Vec::new();
        Method::new(
            "TST1".into(),
            1,
            false,
            vec![&MethodCall::new("TST2".into(), vec![&ONE, &ONE])],
        )
        .to_aml_bytes(&mut methods);
        Method::new(
            "TST2".into(),
            2,
            false,
            vec![&MethodCall::new("TST1".into(), vec![&ONE])],
        )
        .to_aml_bytes(&mut methods);
        assert_eq!(&methods[..], &test_data[..])
    }

    #[test]
    fn test_buffer() {
        /*
        Name (_MAT, Buffer (0x08)  // _MAT: Multiple APIC Table Entry
        {
            0x00, 0x08, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00   /* ........ */
        })
        */
        let buffer_data = [
            0x08, 0x5F, 0x4D, 0x41, 0x54, 0x11, 0x0B, 0x0A, 0x08, 0x00, 0x08, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00,
        ];
        let mut aml = Vec::new();

        Name::new(
            "_MAT".into(),
            &BufferData::new(vec![0x00, 0x08, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]),
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, &buffer_data[..])
    }

    #[test]
    fn test_create_field() {
        /*
        Method (MCRS, 0, Serialized)
        {
            Name (MR64, ResourceTemplate ()
            {
                QWordMemory (ResourceProducer, PosDecode, MinFixed, MaxFixed, Cacheable, ReadWrite,
                    0x0000000000000000, // Granularity
                    0x0000000000000000, // Range Minimum
                    0xFFFFFFFFFFFFFFFE, // Range Maximum
                    0x0000000000000000, // Translation Offset
                    0xFFFFFFFFFFFFFFFF, // Length
                    ,, _Y00, AddressRangeMemory, TypeStatic)
            })
            CreateField (MR64, 14, 64, MIN_)  // _MIN: Minimum Base Address
            CreateField (MR64, 22, 64, MAX_)  // _MAX: Maximum Base Address
            CreateField (MR64, 38, 64, LEN_)  // _LEN: Length
        }
         */
        let data = [
            0x14, 0x4a, 0x06, 0x4d, 0x43, 0x52, 0x53, 0x08, 0x08, 0x4d, 0x52, 0x36, 0x34, 0x11,
            0x33, 0x0a, 0x30, 0x8a, 0x2b, 0x00, 0x00, 0x0c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x79, 0x00, 0x5b, 0x13, 0x4d, 0x52, 0x36,
            0x34, 0x0a, 0x0e, 0x0a, 0x40, 0x4d, 0x49, 0x4e, 0x5f, 0x5b, 0x13, 0x4d, 0x52, 0x36,
            0x34, 0x0a, 0x16, 0x0a, 0x40, 0x4d, 0x41, 0x58, 0x5f, 0x5b, 0x13, 0x4d, 0x52, 0x36,
            0x34, 0x0a, 0x26, 0x0a, 0x40, 0x4c, 0x45, 0x4e, 0x5f,
        ];

        let mut aml = Vec::new();
        Method::new(
            "MCRS".into(),
            0,
            true,
            vec![
                &Name::new(
                    "MR64".into(),
                    &ResourceTemplate::new(vec![&AddressSpace::new_memory(
                        AddressSpaceCacheable::Cacheable,
                        true,
                        0x0000_0000_0000_0000u64,
                        0xFFFF_FFFF_FFFF_FFFEu64,
                        None,
                    )]),
                ),
                &CreateField::new(&Path::new("MIN_"), &Path::new("MR64"), &14u64, &64usize),
                &CreateField::new(&Path::new("MAX_"), &Path::new("MR64"), &22u64, &64usize),
                &CreateField::new(&Path::new("LEN_"), &Path::new("MR64"), &38u64, &64usize),
            ],
        )
        .to_aml_bytes(&mut aml);
        assert_eq!(aml, data);
    }

    #[test]
    fn test_packagebuilder() {
        let expected = vec![0x12, 0x04, 0x01, 0x0A, 0x05];
        {
            let mut aml = Vec::new();

            // Ensure Package and PackageBuilder produce the same output for a single element
            Package::new(vec![&5u8]).to_aml_bytes(&mut aml);
            assert_eq!(expected, aml);
        }
        {
            let mut aml = Vec::new();
            let mut builder = PackageBuilder::new();
            builder.add_element(&5u8);
            builder.to_aml_bytes(&mut aml);
            assert_eq!(expected, aml);
        }
    }

    #[test]
    fn test_additional_aml_operations() {
        let name = Path::new("OBJ0");
        let local0 = Local(0);
        let local1 = Local(1);

        let cases: &[(&dyn Aml, &[u8])] = &[
            (&Break, b"\xa5"),
            (&RefOf::new(&name), b"\x71OBJ0"),
            (&Increment::new(&local0), b"\x75\x60"),
            (&Decrement::new(&local1), b"\x76\x61"),
            (&Stall::new(&10u8), b"\x5b\x21\x0a\x0a"),
            (&Sleep::new(&100u8), b"\x5b\x22\x0a\x64"),
            (&CondRefOf::new(&name, &local0), b"\x5b\x12OBJ0\x60"),
            (
                &Divide::new(&10u8, &3u8, &local0, &local1),
                b"\x78\x0a\x0a\x0a\x03\x60\x61",
            ),
            (
                &ThermalZone::new(Path::new("TZ00"), vec![]),
                b"\x5b\x85\x05TZ00",
            ),
        ];

        for (aml, expected) in cases {
            let mut bytes = Vec::new();
            aml.to_aml_bytes(&mut bytes);
            assert_eq!(&bytes, expected);
        }
    }

    #[test]
    fn test_power_resource() {
        let power_resource = PowerResource::new(Path::new("PWR0"), 0, 0, vec![]);
        let mut aml = Vec::new();

        power_resource.to_aml_bytes(&mut aml);

        assert_eq!(aml, b"\x5b\x84\x08PWR0\x00\x00\x00");
    }

    #[test]
    fn test_packagebuilder_multiple() {
        let expected = vec![
            0x12, 0x47, 0x9, 0x8, 0x12, 0x10, 0x4, 0xb, 0xff, 0xff, 0x0, 0x2e, 0x5f, 0x53, 0x42,
            0x5f, 0x47, 0x53, 0x49, 0x30, 0x0, 0x12, 0x10, 0x4, 0xb, 0xff, 0xff, 0x1, 0x2e, 0x5f,
            0x53, 0x42, 0x5f, 0x47, 0x53, 0x49, 0x31, 0x0, 0x12, 0x11, 0x4, 0xb, 0xff, 0xff, 0xa,
            0x2, 0x2e, 0x5f, 0x53, 0x42, 0x5f, 0x47, 0x53, 0x49, 0x32, 0x0, 0x12, 0x11, 0x4, 0xb,
            0xff, 0xff, 0xa, 0x3, 0x2e, 0x5f, 0x53, 0x42, 0x5f, 0x47, 0x53, 0x49, 0x33, 0x0, 0x12,
            0x12, 0x4, 0xc, 0xff, 0xff, 0x1, 0x0, 0x0, 0x2e, 0x5f, 0x53, 0x42, 0x5f, 0x47, 0x53,
            0x49, 0x31, 0x0, 0x12, 0x12, 0x4, 0xc, 0xff, 0xff, 0x1, 0x0, 0x1, 0x2e, 0x5f, 0x53,
            0x42, 0x5f, 0x47, 0x53, 0x49, 0x32, 0x0, 0x12, 0x13, 0x4, 0xc, 0xff, 0xff, 0x1, 0x0,
            0xa, 0x2, 0x2e, 0x5f, 0x53, 0x42, 0x5f, 0x47, 0x53, 0x49, 0x33, 0x0, 0x12, 0x13, 0x4,
            0xc, 0xff, 0xff, 0x1, 0x0, 0xa, 0x3, 0x2e, 0x5f, 0x53, 0x42, 0x5f, 0x47, 0x53, 0x49,
            0x30, 0x0,
        ];
        {
            let mut aml = Vec::new();
            Package::new(vec![
                &Package::new(vec![&0xffffu32, &0u32, &Path::new("_SB_.GSI0"), &0u32]),
                &Package::new(vec![&0xffffu32, &1u32, &Path::new("_SB_.GSI1"), &0u32]),
                &Package::new(vec![&0xffffu32, &2u32, &Path::new("_SB_.GSI2"), &0u32]),
                &Package::new(vec![&0xffffu32, &3u32, &Path::new("_SB_.GSI3"), &0u32]),
                &Package::new(vec![&0x1ffffu32, &0u32, &Path::new("_SB_.GSI1"), &0u32]),
                &Package::new(vec![&0x1ffffu32, &1u32, &Path::new("_SB_.GSI2"), &0u32]),
                &Package::new(vec![&0x1ffffu32, &2u32, &Path::new("_SB_.GSI3"), &0u32]),
                &Package::new(vec![&0x1ffffu32, &3u32, &Path::new("_SB_.GSI0"), &0u32]),
            ])
            .to_aml_bytes(&mut aml);
            assert_eq!(expected, aml);
        }

        {
            let mut aml = Vec::new();
            let mut builder = PackageBuilder::new();
            builder.add_element(&Package::new(vec![
                &0xffffu32,
                &0u32,
                &Path::new("_SB_.GSI0"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0xffffu32,
                &1u32,
                &Path::new("_SB_.GSI1"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0xffffu32,
                &2u32,
                &Path::new("_SB_.GSI2"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0xffffu32,
                &3u32,
                &Path::new("_SB_.GSI3"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0x1ffffu32,
                &0u32,
                &Path::new("_SB_.GSI1"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0x1ffffu32,
                &1u32,
                &Path::new("_SB_.GSI2"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0x1ffffu32,
                &2u32,
                &Path::new("_SB_.GSI3"),
                &0u32,
            ]));
            builder.add_element(&Package::new(vec![
                &0x1ffffu32,
                &3u32,
                &Path::new("_SB_.GSI0"),
                &0u32,
            ]));
            builder.to_aml_bytes(&mut aml);
            assert_eq!(expected, aml);
        }
    }
}
