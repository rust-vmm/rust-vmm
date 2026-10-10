// Copyright © 2024, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
//

#[allow(
    clippy::too_many_arguments,
    clippy::missing_safety_doc,
    clippy::useless_transmute,
    clippy::unnecessary_cast,
    clippy::non_canonical_clone_impl,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::ptr_offset_with_cast
)]
pub mod bindings;
#[allow(ambiguous_glob_reexports, unused_imports)]
pub use bindings::*;
pub mod regs;
pub use regs::*;
mod unmarshal;
