// Copyright 2026 The rust-vmm authors
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::hint::black_box;
use std::num::NonZeroUsize;

use criterion::{BenchmarkId, Criterion};
use vm_memory::bitmap::AtomicBitmap;

const PAGE_SIZE: usize = 4096;
const BITMAP_SIZE: usize = 16 * 1024 * 1024;

pub fn benchmark_for_bitmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("AtomicBitmap::set_addr_range");

    for page_count in [1, 16, 64, 256] {
        let bitmap = AtomicBitmap::new(
            BITMAP_SIZE,
            NonZeroUsize::new(PAGE_SIZE).expect("page size is non-zero"),
        );
        let len = page_count * PAGE_SIZE;

        group.bench_with_input(BenchmarkId::from_parameter(page_count), &len, |b, &len| {
            b.iter(|| bitmap.set_addr_range(black_box(PAGE_SIZE), black_box(len)))
        });
    }

    group.finish();
}
