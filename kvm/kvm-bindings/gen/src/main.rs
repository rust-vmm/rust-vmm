// SPDX-License-Identifier: Apache-2.0 OR MIT

mod derives;

use anyhow::bail;
use bindgen::callbacks::{AttributeInfo, ParseCallbacks};
use bindgen::{RustEdition, RustTarget, builder};
use derives::{attributes, types};
use std::collections::HashMap;
use std::convert::Into;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

const KERNEL_GIT_URL: &str = "https://github.com/torvalds/linux.git";
const KERNEL_TAG: &str = "v6.15";
const ARCHITECTURES: &[Arch] = &[Arch::X86_64, Arch::Arm64, Arch::Riscv64];
const KVM_HEADER_PATH: &str = "include/linux/kvm.h";

/// Bindings post-processing replacements.
const REPLACEMENTS: &[(&str, &str)] = &[
    (
        "#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>",
        "#[repr(transparent)]
#[derive(Default)]
#[cfg_attr(
    feature = \"serde\",
    derive(zerocopy::IntoBytes, zerocopy::Immutable, zerocopy::FromBytes)
)]
pub struct __IncompleteArrayField<T>",
    ),
    (
        "#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct __BindgenBitfieldUnit<Storage>",
        "#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = \"serde\",
    derive(zerocopy::IntoBytes, zerocopy::Immutable, zerocopy::FromBytes)
)]
pub struct __BindgenBitfieldUnit<Storage>",
    ),
    (
        "pub struct kvm_regs {
    pub regs: user_pt_regs,
    pub sp_el1: __u64,
    pub elr_el1: __u64,
    pub spsr: [__u64; 5usize],
    pub fp_regs: user_fpsimd_state,
}",
        "pub struct kvm_regs {
    pub regs: user_pt_regs,
    pub sp_el1: __u64,
    pub elr_el1: __u64,
    pub spsr: [__u64; 5usize],
    pub __bindgen_padding_0: u64,
    pub fp_regs: user_fpsimd_state,
}",
    ),
];

fn main() -> Result<(), anyhow::Error> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_dir = tempdir()?;
    let kernel_dir = clone_kernel(KERNEL_TAG, temp_dir.path().to_owned())?;

    for arch in ARCHITECTURES {
        println!("Generating bindings for {}", arch.output_name());
        let header_dir = make_headers(arch, &kernel_dir)?;
        let output_file = out_dir.join(format!("../src/{}/bindings.rs", arch.output_name()));

        generate_bindings(
            *arch,
            &header_dir.join(KVM_HEADER_PATH),
            &header_dir.join("include"),
            &output_file,
        )?;
    }

    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum Arch {
    X86_64,
    Arm64,
    Riscv64,
}

impl Arch {
    /// Returns the name of the Clang target for the architecture.
    fn clang_target(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64-unknown-linux-gnu",
            Arch::Arm64 => "aarch64-unknown-linux-gnu",
            Arch::Riscv64 => "riscv64-unknown-linux-gnu",
        }
    }

    /// Returns the name of the headers directory in the linux kernel.
    fn linux_header(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "arm64",
            Arch::Riscv64 => "riscv",
        }
    }

    /// Returns the directory name in `kvm-bindings/src` for the architecture.
    fn output_name(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "arm64",
            Arch::Riscv64 => "riscv64",
        }
    }
}

#[derive(Debug)]
struct ZeroCopyDerive {
    attributes: HashMap<&'static str, &'static str>,
}

impl ZeroCopyDerive {
    fn new(arch: Arch) -> Self {
        let mut attributes = HashMap::new();

        let mut extend_attributes = |attribute: &'static str, types: &[&'static str]| {
            attributes.extend(types.iter().map(|type_name| (*type_name, attribute)))
        };

        extend_attributes(attributes::ZEROCOPY, types::ZEROCOPY);
        extend_attributes(
            attributes::ZEROCOPY_IMMUTABLE_FROM_BYTES,
            types::ZEROCOPY_IMMUTABLE_FROM_BYTES,
        );

        match arch {
            Arch::X86_64 => {
                extend_attributes(attributes::ZEROCOPY, types::x86_64::ZEROCOPY);
                extend_attributes(
                    attributes::ZEROCOPY_IMMUTABLE_FROM_BYTES,
                    types::x86_64::ZEROCOPY_IMMUTABLE_FROM_BYTES,
                );
                extend_attributes(
                    attributes::ZEROCOPY_TRY_FROM_BYTES,
                    types::x86_64::ZEROCOPY_TRY_FROM_BYTES,
                );
            }
            Arch::Arm64 => {
                extend_attributes(attributes::ZEROCOPY, types::arm64::ZEROCOPY);
            }
            Arch::Riscv64 => {
                extend_attributes(attributes::ZEROCOPY, types::riscv64::ZEROCOPY);
            }
        };
        Self { attributes }
    }
}

impl ParseCallbacks for ZeroCopyDerive {
    fn add_attributes(&self, info: &AttributeInfo<'_>) -> Vec<String> {
        self.attributes
            .get(info.name)
            .map(|attribute| vec![attribute.to_string()])
            .unwrap_or_default()
    }
}

fn generate_bindings(
    arch: Arch,
    header_path: &Path,
    include_path: &Path,
    output_file: &Path,
) -> Result<(), anyhow::Error> {
    let header_path = header_path.canonicalize()?;
    let include_path = include_path.canonicalize()?;

    println!("arch: {arch:?}");
    println!("header_path: {header_path:?}");
    println!("include_path: {include_path:?}");
    println!("output_file: {output_file:?}");

    let mut bindings = builder()
        .header(header_path.to_str().unwrap())
        .derive_default(true)
        .derive_partialeq(true)
        .impl_partialeq(true)
        .impl_debug(true)
        .rust_edition(RustEdition::Edition2024)
        .rust_target(RustTarget::stable(85, 0).unwrap())
        .parse_callbacks(Box::new(ZeroCopyDerive::new(arch)))
        .clang_arg(format!("--target={}", arch.clang_target()))
        .clang_arg(format!("-I{}", include_path.to_str().unwrap()))
        .generate()?
        .to_string();

    for (to_replace, replacement) in REPLACEMENTS {
        bindings = bindings.replace(*to_replace, replacement);
    }

    fs::write(output_file, bindings).map_err(Into::into)
}

fn make_headers(arch: &Arch, kernel_dir: &Path) -> Result<PathBuf, anyhow::Error> {
    let headers_dir = kernel_dir.join(format!("headers/{}", arch.linux_header()));

    fs::create_dir_all(&headers_dir)?;

    if !Command::new("make")
        .arg("headers_install")
        .arg(format!("ARCH={}", arch.linux_header()))
        .arg(format!(
            "INSTALL_HDR_PATH={}",
            headers_dir.canonicalize()?.to_str().unwrap()
        ))
        .current_dir(kernel_dir)
        .status()?
        .success()
    {
        bail!("failed to make headers for {}", arch.linux_header());
    }

    Ok(headers_dir)
}

fn clone_kernel(revision: &str, mut working_dir: PathBuf) -> Result<PathBuf, anyhow::Error> {
    if !Command::new("git")
        .arg("clone")
        .arg(KERNEL_GIT_URL)
        // Don't include any trees or blobs.
        .arg("--filter=tree:0")
        // Don't include anything beyond the latest commit.
        .arg("--depth=1")
        // Don't checkout yet, we need to prepare the sparse checkout first.
        .arg("--no-checkout")
        .arg("--branch")
        .arg(revision)
        .current_dir(&working_dir)
        .status()?
        .success()
    {
        bail!("failed to clone kernel repo");
    }

    working_dir.push("linux");

    // Use sparse-checkout to only check out what we actually need.
    if !Command::new("git")
        .arg("sparse-checkout")
        .arg("set")
        .arg("--no-cone")
        .arg("/*")
        .arg("!/*/")
        .arg("/include/")
        .arg("/arch/")
        .arg("/scripts/")
        .arg("/tools/")
        .current_dir(&working_dir)
        .status()?
        .success()
    {
        bail!("failed to initialize sparse checkout");
    }

    if !Command::new("git")
        .arg("checkout")
        .arg(revision)
        .arg("-f")
        .current_dir(&working_dir)
        .status()?
        .success()
    {
        bail!("failed to checkout kernel");
    }

    Ok(working_dir)
}
