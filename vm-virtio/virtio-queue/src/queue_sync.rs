// Copyright (C) 2021 Alibaba Cloud. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0 AND BSD-3-Clause

use std::num::Wrapping;
use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};

use vm_memory::GuestMemory;

use crate::{DescriptorChain, Error, Queue, QueueGuard, QueueT};

/// Struct to maintain information and manipulate state of a virtio queue for multi-threaded
/// context.
///
/// # Example
///
/// ```rust
/// use virtio_queue::{Queue, QueueSync, QueueT};
/// use vm_memory::{Bytes, GuestAddress, GuestAddressSpace, GuestMemoryMmap};
///
/// let m = &GuestMemoryMmap::<()>::from_ranges(&[(GuestAddress(0), 0x10000)]).unwrap();
/// let mut queue = QueueSync::new(1024).unwrap();
///
/// // First, the driver sets up the queue; this set up is done via writes on the bus (PCI, MMIO).
/// queue.set_size(8);
/// queue.set_desc_table_address(Some(0x1000), None);
/// queue.set_avail_ring_address(Some(0x2000), None);
/// queue.set_used_ring_address(Some(0x3000), None);
/// queue.set_ready(true);
/// // The user should check if the queue is valid before starting to use it.
/// assert!(queue.is_valid(m.memory()));
///
/// // The memory object is not embedded in the `QueueSync`, so we have to pass it as a
/// // parameter to the methods that access the guest memory. Examples would be:
/// queue.add_used(m.memory(), 1, 0x100).unwrap();
/// queue.needs_notification(m.memory()).unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct QueueSync {
    state: Arc<Mutex<Queue>>,
}

impl QueueSync {
    fn lock_state(&self) -> MutexGuard<'_, Queue> {
        // Do not expect poisoned lock.
        self.state.lock().unwrap()
    }
}

impl<'a> QueueGuard<'a> for QueueSync {
    type G = MutexGuard<'a, Queue>;
}

impl QueueT for QueueSync {
    fn new(max_size: u16) -> Result<Self, Error> {
        Ok(QueueSync {
            state: Arc::new(Mutex::new(Queue::new(max_size)?)),
        })
    }

    fn is_valid<M: GuestMemory>(&self, mem: &M) -> bool {
        self.lock_state().is_valid(mem)
    }

    fn reset(&mut self) {
        self.lock_state().reset();
    }

    fn lock(&mut self) -> <Self as QueueGuard<'_>>::G {
        self.lock_state()
    }

    fn max_size(&self) -> u16 {
        self.lock_state().max_size()
    }

    fn size(&self) -> u16 {
        self.lock_state().size()
    }

    fn set_size(&mut self, size: u16) {
        self.lock_state().set_size(size);
    }

    fn ready(&self) -> bool {
        self.lock_state().ready()
    }

    fn set_ready(&mut self, ready: bool) {
        self.lock_state().set_ready(ready)
    }

    fn set_desc_table_address(&mut self, low: Option<u32>, high: Option<u32>) {
        self.lock_state().set_desc_table_address(low, high);
    }

    fn set_avail_ring_address(&mut self, low: Option<u32>, high: Option<u32>) {
        self.lock_state().set_avail_ring_address(low, high);
    }

    fn set_used_ring_address(&mut self, low: Option<u32>, high: Option<u32>) {
        self.lock_state().set_used_ring_address(low, high);
    }

    fn set_event_idx(&mut self, enabled: bool) {
        self.lock_state().set_event_idx(enabled);
    }

    fn avail_idx<M>(&self, mem: &M, order: Ordering) -> Result<Wrapping<u16>, Error>
    where
        M: GuestMemory + ?Sized,
    {
        self.lock_state().avail_idx(mem, order)
    }

    fn used_idx<M: GuestMemory>(&self, mem: &M, order: Ordering) -> Result<Wrapping<u16>, Error> {
        self.lock_state().used_idx(mem, order)
    }

    fn add_used<M: GuestMemory>(
        &mut self,
        mem: &M,
        head_index: u16,
        len: u32,
    ) -> Result<(), Error> {
        self.lock_state().add_used(mem, head_index, len)
    }

    fn enable_notification<M: GuestMemory>(&mut self, mem: &M) -> Result<bool, Error> {
        self.lock_state().enable_notification(mem)
    }

    fn disable_notification<M: GuestMemory>(&mut self, mem: &M) -> Result<(), Error> {
        self.lock_state().disable_notification(mem)
    }

    fn needs_notification<M: GuestMemory>(&mut self, mem: &M) -> Result<bool, Error> {
        self.lock_state().needs_notification(mem)
    }

    fn next_avail(&self) -> u16 {
        self.lock_state().next_avail()
    }

    fn set_next_avail(&mut self, next_avail: u16) {
        self.lock_state().set_next_avail(next_avail);
    }

    fn next_used(&self) -> u16 {
        self.lock_state().next_used()
    }

    fn set_next_used(&mut self, next_used: u16) {
        self.lock_state().set_next_used(next_used);
    }

    fn desc_table(&self) -> u64 {
        self.lock_state().desc_table()
    }

    fn avail_ring(&self) -> u64 {
        self.lock_state().avail_ring()
    }

    fn used_ring(&self) -> u64 {
        self.lock_state().used_ring()
    }

    fn event_idx_enabled(&self) -> bool {
        self.lock_state().event_idx_enabled()
    }

    fn pop_descriptor_chain<M>(&mut self, mem: M) -> Option<DescriptorChain<M>>
    where
        M: Clone + Deref,
        M::Target: GuestMemory,
    {
        self.lock_state().pop_descriptor_chain(mem)
    }
}

impl From<Queue> for QueueSync {
    fn from(queue: Queue) -> Self {
        QueueSync {
            state: Arc::new(Mutex::new(queue)),
        }
    }
}

impl From<Arc<Mutex<Queue>>> for QueueSync {
    fn from(state: Arc<Mutex<Queue>>) -> Self {
        QueueSync { state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn test_queue_state_sync() {
        let mut q = QueueSync::new(0x1000).unwrap();
        let mut q2 = q.clone();
        let q3 = q.clone();
        let barrier = Arc::new(Barrier::new(3));
        let b2 = barrier.clone();
        let b3 = barrier.clone();

        let t1 = std::thread::spawn(move || {
            {
                let guard = q2.lock();
                assert!(!guard.ready());
            }
            b2.wait();
            b2.wait();
            {
                let guard = q2.lock();
                assert!(guard.ready());
            }
        });

        let t2 = std::thread::spawn(move || {
            assert!(!q3.ready());
            b3.wait();
            b3.wait();
            assert!(q3.ready());
        });

        barrier.wait();
        q.set_ready(true);
        barrier.wait();

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn test_queue_sync_from() {
        let q = Queue::new(0x100).unwrap();
        let q_sync = QueueSync::from(q);
        assert_eq!(q_sync.max_size(), 0x100);

        let q2 = Queue::new(0x200).unwrap();
        let state = Arc::new(Mutex::new(q2));
        let q_sync2 = QueueSync::from(state);
        assert_eq!(q_sync2.max_size(), 0x200);
    }
}
