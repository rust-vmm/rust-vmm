# rust-vmm

rust-vmm is an open-source project that empowers the community to build
custom Virtual Machine Monitors (VMMs) and hypervisors. It provides a set of
virtualization components that any project can use to quickly develop
virtualization solutions while focusing on the key differentiators of their
product rather than re-implementing common components like KVM wrappers, virtio
devices and other VMM libraries.

For more details about the rust-vmm scope, governance, and organization, please
check the [rust-vmm/community](https://github.com/rust-vmm/community)
repository.

## Scope

This repository contains the rust-vmm crates.
Each crate has its code owners as
defined in the [CODEOWNERS](CODEOWNERS) file.
For more details about each crate, you can check their corresponding readme.

## Current Status

This repository is still under construction as we're working towards merging
all rust-vmm crates here. Once a crate/worspace is moved, its original
repository becomes a public archive that can still be accessed. All future
development moves to this repository.

We're tracking the progress of this move as part of
[community#193](https://github.com/rust-vmm/community/issues/193).

## The rust-vmm crates

*NOTE: As we're working on merging all crates under this mono-repo, we're
listing below only the crates that currently live here. You can find all other
rust-vmm crates in the root organization [rust-vmm](https://github.com/rust-vmm).*

- [vmm-sys-util](vmm-sys-util): Helpers and utilities used by multiple rust-vmm
  components.
