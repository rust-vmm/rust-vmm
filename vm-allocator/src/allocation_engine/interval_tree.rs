// Copyright (C) 2022 Alibaba Cloud. All rights reserved.
// Copyright 2022 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

use std::cmp::{max, Ordering};

use alloc::boxed::Box;

use crate::{AllocPolicy, Constraint, Error, RangeInclusive, Result};

/// Returns the first multiple of `alignment` that is lower or equal to the
/// specified address. This method works only for alignment values that are a
/// power of two.
pub fn align_down(address: u64, alignment: u64) -> Result<u64> {
    if !alignment.is_power_of_two() {
        return Err(Error::InvalidAlignment);
    }
    // It is safe to subtract 1 as alignment is already checked to be greater
    // than 0.
    Ok(address & !(alignment - 1))
}

/// Returns the first multiple of `alignment` that is greater or equal to the
/// specified address. This method works only for alignment values that are a
/// power of two.
pub fn align_up(address: u64, alignment: u64) -> Result<u64> {
    if alignment == 0 {
        return Err(Error::InvalidAlignment);
    }
    // It is safe to subtract 1 as alignment is already checked to be greater
    // than 0.
    if let Some(intermediary_address) = address.checked_add(alignment - 1) {
        return align_down(intermediary_address, alignment);
    }
    Err(Error::Overflow)
}

/// Node state for interval tree nodes.
///
/// Valid state transition:
/// - None -> Free: IntervalTree::insert()
/// - Free -> Allocated: IntervalTree::allocate()
/// - Allocated -> Free: IntervalTree::free()
/// - * -> None: IntervalTree::delete()
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeState {
    /// Node is free.
    Free,
    /// Node is allocated.
    Allocated,
}

impl NodeState {
    fn is_free(&self) -> bool {
        *self == NodeState::Free
    }
}

/// Internal tree node to implement interval tree.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct InnerNode {
    /// Interval handled by this node.
    key: RangeInclusive,
    /// NodeState, can be Free or Allocated.
    node_state: NodeState,
    /// Optional left child of current node.
    left: Option<Box<InnerNode>>,
    /// Optional right child of current node.
    right: Option<Box<InnerNode>>,
    /// Cached height of the node.
    height: u64,
}

impl InnerNode {
    /// Creates a new InnerNode object.
    fn new(key: RangeInclusive, node_state: NodeState) -> Self {
        InnerNode {
            key,
            node_state,
            left: None,
            right: None,
            height: 1,
        }
    }

    /// Returns a readonly reference to the node associated with the `key` or
    /// None if the searched key does not exist in the tree.
    fn search(&self, key: &RangeInclusive) -> Option<&InnerNode> {
        match self.key.cmp(key) {
            Ordering::Equal => Some(self),
            Ordering::Less => self.right.as_ref().and_then(|node| node.search(key)),
            Ordering::Greater => self.left.as_ref().and_then(|node| node.search(key)),
        }
    }

    /// Returns a readonly reference to the node associated with the `key` or
    /// None if there is no Node representing an interval that covers the
    /// searched key. For a key [a, b], this method will return a node with
    /// a key [c, d] such that c <= a and b <= d.
    fn search_superset(&self, key: &RangeInclusive) -> Option<&InnerNode> {
        if self.key.contains(key) {
            Some(self)
        } else if key.end < self.key.start {
            self.left
                .as_ref()
                .and_then(|node| node.search_superset(key))
        } else {
            self.right
                .as_ref()
                .and_then(|node| node.search_superset(key))
        }
    }

    /// Rotates the tree such that height difference between subtrees
    /// is not greater than abs(1).
    fn rotate(self: Box<Self>) -> Box<Self> {
        let l = height(&self.left);
        let r = height(&self.right);

        match (l as i64) - (r as i64) {
            -1..=1 => self,
            // Safe to unwrap as rotate_left_successor always returns Some when
            // the current node has a left child and we just checked that it
            // has at least one child otherwise this difference would not be two.
            2 => self.rotate_left_successor().unwrap(),
            // Safe to unwrap as rotate_right_successor always returns Some when
            // the current node has a right child and we just checked that it
            // has at least one child otherwise this difference would not be
            // minus two.
            -2 => self.rotate_right_successor().unwrap(),
            _ => unreachable!(),
        }
    }

    /// Performs a single left rotation on this node.
    fn rotate_left(mut self: Box<Self>) -> Option<Box<Self>> {
        if let Some(mut new_root) = self.right.take() {
            self.right = new_root.left.take();
            self.update_cached_height();
            new_root.left = Some(self);
            new_root.update_cached_height();
            return Some(new_root);
        }
        None
    }

    /// Performs a single right rotation on this node.
    fn rotate_right(mut self: Box<Self>) -> Option<Box<Self>> {
        if let Some(mut new_root) = self.left.take() {
            self.left = new_root.right.take();
            self.update_cached_height();
            new_root.right = Some(self);
            new_root.update_cached_height();
            return Some(new_root);
        }
        None
    }

    /// Performs a rotation when the left successor is too high.
    fn rotate_left_successor(mut self: Box<Self>) -> Option<Box<Self>> {
        if let Some(left) = self.left.take() {
            if height(&left.left) < height(&left.right) {
                self.left = left.rotate_left();
                self.update_cached_height();
            } else {
                self.left = Some(left);
            }
            return self.rotate_right();
        }
        None
    }

    /// Performs a rotation when the right successor is too high.
    fn rotate_right_successor(mut self: Box<Self>) -> Option<Box<Self>> {
        if let Some(right) = self.right.take() {
            if height(&right.left) > height(&right.right) {
                self.right = right.rotate_right();
                self.update_cached_height();
            } else {
                self.right = Some(right);
            }
            return self.rotate_left();
        }
        None
    }

    /// Deletes the entry point of this tree structure.
    fn delete_root(mut self) -> Option<Box<Self>> {
        match (self.left.take(), self.right.take()) {
            (None, None) => None,
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (Some(l), Some(r)) => Some(Self::combine_subtrees(l, r)),
        }
    }

    /// Finds the minimal key below the tree and returns a new optional tree
    /// where the minimal value has been removed and the (optional) minimal node
    /// as tuple (min_node, remaining).
    fn get_new_root(mut self: Box<Self>) -> (Box<Self>, Option<Box<Self>>) {
        match self.left.take() {
            None => {
                let remaining = self.right.take();
                (self, remaining)
            }
            Some(left) => {
                let (min_node, left) = left.get_new_root();
                self.left = left;
                self.update_cached_height();
                (min_node, Some(self.rotate()))
            }
        }
    }

    /// Creates a single tree from the subtrees resulted from deleting the root
    /// node.
    fn combine_subtrees(l: Box<Self>, r: Box<Self>) -> Box<Self> {
        let (mut new_root, remaining) = r.get_new_root();
        new_root.left = Some(l);
        new_root.right = remaining;
        new_root.update_cached_height();
        new_root.rotate()
    }

    /// Updates cached information of the node.
    fn update_cached_height(&mut self) {
        // It is safe adding 1 to the height as it can not be greater than 50
        // hence no chance of overflowing.
        self.height = max(height(&self.left), height(&self.right)) + 1;
    }

    /// Insert a new node in the subtree. After the node is inserted the
    /// tree will be balanced. The node_state parameter is needed because in
    /// the AddressAllocator allocation logic we will need to insert both free
    /// and allocated nodes.
    fn insert(
        mut self: Box<Self>,
        key: RangeInclusive,
        node_state: NodeState,
    ) -> Result<Box<Self>> {
        // The InnerNode structure has 48 a length of 48 bytes. With other nested
        // calls that are made during the insertion process the size occupied
        // on the stack by just one insert call is around 122 bytes. Considering
        // that the default stack size on Linux is 8K we could make around 73
        // calls to insert method before confronting with an stack overflow. To
        // be cautious we will use 50 as the maximum height of the tree. A
        // maximum height of 50 will result in the possibility to allocate
        // (2^50 - 1) memory slots. Considering the imposed maximum height the
        // recursion is safe to use.
        // It is safe adding 1 to the height as it can not be greater than 50
        // hence no chance of overflowing.
        if (self.height + 1) > 50 {
            return Err(Error::Overflow);
        }
        if self.key.overlaps(&key) {
            return Err(Error::Overlap(key, self.key));
        }
        match self.key.cmp(&key) {
            // It is not possible for a RangeInclusive to be equal with an existing node
            // as the overlaps method will also catch this case and return the
            // corresponding error code.
            Ordering::Equal => unreachable!(),
            Ordering::Less => match self.right {
                None => self.right = Some(Box::new(InnerNode::new(key, node_state))),
                Some(right) => {
                    self.right = Some(right.insert(key, node_state)?);
                }
            },
            Ordering::Greater => match self.left {
                None => self.left = Some(Box::new(InnerNode::new(key, node_state))),
                Some(left) => {
                    self.left = Some(left.insert(key, node_state)?);
                }
            },
        }
        self.update_cached_height();
        Ok(self.rotate())
    }

    /// Update the state of an old node. This method should be used when we
    /// find an existing node with the state `NodeState::Free` that satisfies
    /// all constraints of an allocation request. The recursion is safe as we
    /// have in place a maximum height for the tree.
    fn mark_as_allocated(&mut self, key: &RangeInclusive) -> Result<()> {
        match self.key.cmp(key) {
            Ordering::Equal => {
                if self.node_state != NodeState::Free {
                    return Err(Error::InvalidStateTransition(self.key, self.node_state));
                }
                self.node_state = NodeState::Allocated;
                Ok(())
            }
            Ordering::Less => match self.right.as_mut() {
                None => Err(Error::ResourceNotAvailable),
                Some(node) => node.mark_as_allocated(key),
            },
            Ordering::Greater => match self.left.as_mut() {
                None => Err(Error::ResourceNotAvailable),
                Some(node) => node.mark_as_allocated(key),
            },
        }
    }

    /// Delete `key` from the subtree.
    ///
    /// Note: it doesn't return whether the key exists in the subtree, so caller
    /// need to ensure the logic.
    fn delete(mut self: Box<Self>, key: &RangeInclusive) -> Option<Box<Self>> {
        match self.key.cmp(key) {
            Ordering::Equal => {
                return self.delete_root();
            }
            Ordering::Less => {
                if let Some(node) = self.right.take() {
                    let right = node.delete(key);
                    self.right = right;
                    self.update_cached_height();
                    return Some(self.rotate());
                }
            }
            Ordering::Greater => {
                if let Some(node) = self.left.take() {
                    let left = node.delete(key);
                    self.left = left;
                    self.update_cached_height();
                    return Some(self.rotate());
                }
            }
        }
        Some(self)
    }

    /// Returns the best node from the tree to place the desired memory slot
    /// and a RangeInclusive object with the start address aligned to the value specified
    /// in the constraint.The RangeInclusive returned by this method may be larger than
    /// what was requested. It's up for the caller to split the node if it wants
    /// to allocate the exact size from this node.
    fn find_candidate(&self, constraint: &Constraint) -> Result<(&Self, RangeInclusive)> {
        match constraint.policy {
            // Returns the first node from the managed address space that is
            // satisfying the specified constraints or `ResourceNotAvailable`
            // if the request can not be satisfied.
            AllocPolicy::FirstMatch => self.first_match(constraint),
            // Returns the last node from the managed address space that is
            // satisfying the specified constraints or `ResourceNotAvailable`
            // if the request can not be satisfied.
            AllocPolicy::LastMatch => self.last_match(constraint),
            // Returns the node containing the address specified or the
            // `ResourceNotAvailable` error if any of the sanity checks is not
            // passing.
            AllocPolicy::ExactMatch(start_address) => {
                // Search the node in the interval tree that contains the
                // desired starting address.
                let node = self
                    .search_superset(&RangeInclusive::new(start_address, start_address)?)
                    .ok_or(Error::ResourceNotAvailable)?;
                let end_address = start_address
                    .checked_add(constraint.size().checked_sub(1).ok_or(Error::Underflow)?)
                    .ok_or(Error::Overflow)?;
                // We should check that starting from the desired address the
                // whole memory slot will fit in the selected free node.
                let range = RangeInclusive::new(start_address, end_address)?;
                if !node.node_state.is_free() || !node.key.contains(&range) {
                    return Err(Error::ResourceNotAvailable);
                }
                Ok((node, range))
            }
        }
    }

    /// Returns the first node from the managed address space that is satisfying
    /// the specified constraints and the aligned address of the desired memory
    /// slot. Or if the request can not be satisfied `ResourceNotAvailable`.
    fn first_match(&self, constraint: &Constraint) -> Result<(&Self, RangeInclusive)> {
        // Searches the first free node from the tree.
        let mut res = self
            .left
            .as_ref()
            .map_or(Err(Error::ResourceNotAvailable), |node| {
                node.first_match(constraint)
            });

        // If the result is Error::ResourceNotAvailable this means that we got
        // to the first free node from the tree. We check if this node is
        // satisfying all the constraints, if yes save the values and return
        // them at the end of the method.
        if res == Err(Error::ResourceNotAvailable) {
            res = self
                .check_constraint(constraint)
                .map_or(res, |node| Ok((self, node)))
        }

        // If res is still Error::ResourceNotAvailable we continue our search
        // on the right part of the tree, as the method is recursive the same
        // logic from above will apply.
        if res == Err(Error::ResourceNotAvailable) {
            res = self
                .right
                .as_ref()
                .map_or(Err(Error::ResourceNotAvailable), |node| {
                    node.first_match(constraint)
                });
        }
        res
    }

    /// Returns the last node from the managed address space that is satisfying
    /// the specified constraints and the aligned address of the desired memory
    /// slot. Or if the request can not be satisfied `ResourceNotAvailable`.
    fn last_match(&self, constraint: &Constraint) -> Result<(&Self, RangeInclusive)> {
        // Searches the last free node from the tree.
        let mut res = self
            .right
            .as_ref()
            .map_or(Err(Error::ResourceNotAvailable), |node| {
                node.last_match(constraint)
            });

        // If the result is Error::ResourceNotAvailable this means that we got
        // to the last free node from the tree. We check if this node is
        // satisfying all the constraints, if yes save the values and return
        // them at the end of the method
        if res == Err(Error::ResourceNotAvailable) {
            res = self
                .check_constraint(constraint)
                .map_or(res, |node| Ok((self, node)))
        }

        // If res is still Error::ResourceNotAvailable we continue our search
        // on the left part of the tree, as the method is recursive the same
        // logic from above will apply.
        if res == Err(Error::ResourceNotAvailable) {
            res = self
                .left
                .as_ref()
                .map_or(Err(Error::ResourceNotAvailable), |node| {
                    node.last_match(constraint)
                });
        }
        res
    }

    /// Check that the candidate node is satisfying all the constraints for
    /// the requested memory slot.
    fn check_constraint(&self, constraint: &Constraint) -> Result<RangeInclusive> {
        // Exit if node is already allocated.
        if !self.node_state.is_free() || self.key.len() < constraint.size {
            return Err(Error::ResourceNotAvailable);
        }
        let node_key = self.key;
        // Get the starting address for the memory slot.
        let range_start = match constraint.policy {
            AllocPolicy::FirstMatch => align_up(node_key.start(), constraint.align)?,
            AllocPolicy::LastMatch => {
                // This operation can not underflow as we check at the beginning
                // of this method that the requested node fits in the selected
                // node. The subsequent addition can not overflow as well since
                // we already subtract the desired length (e.g. Give a range
                // [x, u64::MAX] and we want to allocate a node with size Y and
                // AllocPolicy::LastMatch computing the candidate address will
                // not overflow as we subtract from u64::MAX Y in the step above).
                let candidate_address = node_key
                    .end()
                    .checked_sub(constraint.size())
                    .ok_or(Error::Underflow)
                    .and_then(|addr| addr.checked_add(1).ok_or(Error::Overflow))?;
                let aligned_address = align_down(candidate_address, constraint.align)?;
                if aligned_address < self.key.start() {
                    return Err(Error::UnalignedAddress);
                }
                aligned_address
            }
            AllocPolicy::ExactMatch(_) => unreachable!(),
        };
        // Create the result range.
        let key = RangeInclusive::new(range_start, self.key.end())?;
        // Check if the desired memory slot does fit in the candidate node.
        if key.len() >= constraint.size() {
            return Ok(key);
        }
        Err(Error::ResourceNotAvailable)
    }
}

/// Compute height of the optional sub-tree.
fn height(node: &Option<Box<InnerNode>>) -> u64 {
    node.as_ref().map_or(0, |n| n.height)
}

/// An interval tree implementation specialized for VMM memory slots management.
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntervalTree {
    root: Option<Box<InnerNode>>,
}

impl IntervalTree {
    /// Creates a new IntervalTree object that is going to be used by the
    /// AddressAllocator.
    pub fn new(key: RangeInclusive) -> Self {
        IntervalTree {
            root: Some(Box::new(InnerNode::new(key, NodeState::Free))),
        }
    }

    fn search_superset(&self, key: &RangeInclusive) -> Option<&InnerNode> {
        match self.root {
            None => None,
            Some(ref node) => node.search_superset(key),
        }
    }

    fn insert(&mut self, key: RangeInclusive, node_state: NodeState) -> Result<()> {
        match self.root.take() {
            None => self.root = Some(Box::new(InnerNode::new(key, node_state))),
            Some(node) => self.root = Some(node.insert(key, node_state)?),
        };
        Ok(())
    }

    fn mark_as_allocated(&mut self, key: &RangeInclusive) -> Result<()> {
        match self.root.as_mut() {
            None => (),
            Some(node) => node.mark_as_allocated(key)?,
        };
        Ok(())
    }

    fn delete(&mut self, key: &RangeInclusive) -> Result<()> {
        if let Some(node) = self.root.take() {
            if node.search(key).is_none() {
                self.root = Some(node);
                return Err(Error::ResourceNotAvailable);
            }
            self.root = node.delete(key);
        }
        Ok(())
    }

    /// This method implements the allocation logic for the address allocator.
    /// Given a set of constraints it will find the most suitable free node to
    /// fit the desired memory slot. This will modify the backing interval tree
    /// such that the RangeInclusive representing the desired memory slot will appear as
    /// an node with the state `NodeState::Allocated` while the leftovers of
    /// the previous node will be present in the tree as free nodes.
    pub fn allocate(&mut self, constraint: Constraint) -> Result<RangeInclusive> {
        // Return ResourceNotAvailable if we can not get a reference to the
        // root node.
        let root = self.root.as_ref().ok_or(Error::ResourceNotAvailable)?;
        let (node, range) = root.find_candidate(&constraint)?;
        let node_key = node.key;
        // Create a new RangeInclusive starting at an address that is aligned to the
        // value specified by constraint.
        let result = RangeInclusive::new(
            range.start(),
            range
                .start()
                .checked_add(constraint.size())
                .ok_or(Error::Overflow)
                .and_then(|addr| addr.checked_sub(1).ok_or(Error::Underflow))?,
        )?;

        // Allocate a resource from the node, no need to split the candidate node.
        if node_key.start() == result.start() && node_key.len() == constraint.size {
            self.mark_as_allocated(&node_key)?;
            return Ok(node_key);
        }

        // If we do not find a node that is a perfect match we delete the old
        // node and insert three new nodes. The first node will represent the
        // RangeInclusive [old_node.start, aligned_addr - 1] and will be marked as free.
        // The second node will have the state NodeState::Allocated and is
        // actually the requested memory slot. The last node will have the
        // state NodeState::Free and is what is left from the old node.
        self.delete(&node_key)?;
        if result.start > node_key.start() {
            self.insert(
                RangeInclusive::new(
                    node_key.start(),
                    result.start().checked_sub(1).ok_or(Error::Overflow)?,
                )?,
                NodeState::Free,
            )?;
        }

        self.insert(result, NodeState::Allocated)?;
        if result.end() < node_key.end() {
            self.insert(
                RangeInclusive::new(
                    result.end().checked_add(1).ok_or(Error::Overflow)?,
                    node_key.end(),
                )?,
                NodeState::Free,
            )?;
        }
        Ok(result)
    }

    /// Free an allocated range.
    pub fn free(&mut self, key: &RangeInclusive) -> Result<()> {
        self.delete(key)?;
        let mut range = *key;

        // If the deleted RangeInclusive did not start at 0 we try to find range that
        // are placed to its left so we can merge them together.
        if range.start() > 0 {
            if let Some(node) = self.search_superset(&RangeInclusive::new(
                range.start().checked_sub(2).ok_or(Error::Underflow)?,
                range.start().checked_sub(1).ok_or(Error::Underflow)?,
            )?) {
                if node.node_state == NodeState::Free {
                    range = RangeInclusive::new(node.key.start(), range.end())?;
                }
            }
        }
        // If the deleted range did not end at u64::MAX we try to find ranges
        // that are placed to its left so we can merge them together.
        if range.end() < u64::MAX {
            if let Some(node) = self.search_superset(&RangeInclusive::new(
                range.end().checked_add(1).ok_or(Error::Overflow)?,
                range.end().checked_add(2).ok_or(Error::Overflow)?,
            )?) {
                if node.node_state == NodeState::Free {
                    range = RangeInclusive::new(range.start(), node.key.end())?;
                }
            }
        }

        // If we merged the freed node to the one on its left we should delete
        // the left node as it now belongs to a bigger RangeInclusive that will be
        // inserted in the tree.
        if range.start() < key.start() {
            self.delete(&RangeInclusive::new(
                range.start(),
                key.start().checked_sub(1).ok_or(Error::Underflow)?,
            )?)?;
        }

        // If we merged the freed node to the one on its right we should delete
        // the right node as it now belongs to a bigger RangeInclusive that will be
        // inserted in the tree.
        if range.end() > key.end() {
            self.delete(&RangeInclusive::new(
                key.end().checked_add(1).ok_or(Error::Overflow)?,
                range.end(),
            )?)?;
        }
        // Insert in the tree the new created range.
        self.insert(range, NodeState::Free)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_align_up() {
        assert_eq!(align_up(2, 0).unwrap_err(), Error::InvalidAlignment);
        assert_eq!(align_up(2, 1).unwrap(), 2);
        assert_eq!(align_up(2, 2).unwrap(), 2);
        assert_eq!(align_up(2, 4).unwrap(), 4);
        assert_eq!(align_up(2, 3).unwrap_err(), Error::InvalidAlignment);

        assert_eq!(
            align_up(0xFFFF_FFFF_FFFF_FFFDu64, 2).unwrap(),
            0xFFFF_FFFF_FFFF_FFFEu64
        );
        assert_eq!(
            align_up(0xFFFF_FFFF_FFFF_FFFDu64, 4).unwrap_err(),
            Error::Overflow
        );
    }

    #[test]
    fn test_is_free() {
        let mut ns = NodeState::Allocated;
        assert!(!ns.is_free());
        ns = NodeState::Free;
        assert!(ns.is_free());
    }

    #[test]
    fn test_search() {
        let mut tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x100, 0x110).unwrap(),
            NodeState::Allocated,
        ));
        let left_child = InnerNode::new(RangeInclusive::new(0x90, 0x99).unwrap(), NodeState::Free);

        tree = tree.insert(left_child.key, left_child.node_state).unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x200, 0x2FF).unwrap(), NodeState::Free)
            .unwrap();

        assert_eq!(
            tree.search(&RangeInclusive::new(0x90, 0x99).unwrap()),
            Some(&left_child)
        );
        assert_eq!(
            tree.search(&RangeInclusive::new(0x200, 0x250).unwrap()),
            None
        );
        assert_eq!(
            tree.search(&RangeInclusive::new(0x111, 0x1fe).unwrap()),
            None
        );
    }

    #[test]
    fn test_search_superset() {
        let mut tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x100, 0x110).unwrap(),
            NodeState::Allocated,
        ));
        let right_child =
            InnerNode::new(RangeInclusive::new(0x200, 0x2FF).unwrap(), NodeState::Free);
        let left_child = InnerNode::new(RangeInclusive::new(0x90, 0x9F).unwrap(), NodeState::Free);

        tree = tree.insert(left_child.key, left_child.node_state).unwrap();
        tree = tree
            .insert(right_child.key, right_child.node_state)
            .unwrap();

        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x100, 0x101).unwrap()),
            Some(&(*tree))
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x90, 0x95).unwrap()),
            Some(&left_child)
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x200, 0x201).unwrap()),
            Some(&right_child)
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x200, 0x2FF).unwrap()),
            Some(&right_child)
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x209, 0x210).unwrap()),
            Some(&right_child)
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x2EF, 0x2FF).unwrap()),
            Some(&right_child)
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x2FF, 0x300).unwrap()),
            None
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x300, 0x301).unwrap()),
            None
        );
        assert_eq!(
            tree.search_superset(&RangeInclusive::new(0x1FF, 0x300).unwrap()),
            None
        );
    }

    fn is_balanced(tree: Option<Box<InnerNode>>) -> bool {
        if tree.is_none() {
            return true;
        }
        let left_height = height(&tree.as_ref().unwrap().left.clone());
        let right_height = height(&tree.as_ref().unwrap().right.clone());
        if (left_height as i64 - right_height as i64).abs() <= 1
            && is_balanced(tree.as_ref().unwrap().left.clone())
            && is_balanced(tree.as_ref().unwrap().right.clone())
        {
            return true;
        }
        false
    }

    #[test]
    fn test_tree_insert_balanced() {
        let mut tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x300, 0x310).unwrap(),
            NodeState::Allocated,
        ));
        tree = tree
            .insert(RangeInclusive::new(0x100, 0x110).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x350, 0x360).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x340, 0x34F).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x311, 0x33F).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree.delete_root().unwrap();
        assert!(is_balanced(Some(tree)));
        tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x300, 0x310).unwrap(),
            NodeState::Allocated,
        ));
        tree = tree
            .insert(RangeInclusive::new(0x100, 0x110).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x90, 0x9F).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x311, 0x313).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x314, 0x316).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x317, 0x319).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x321, 0x323).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));

        tree = tree
            .delete(&RangeInclusive::new(0x321, 0x323).unwrap())
            .unwrap();
        tree = tree
            .delete(&RangeInclusive::new(0x314, 0x316).unwrap())
            .unwrap();
        tree = tree
            .delete(&RangeInclusive::new(0x317, 0x319).unwrap())
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x80, 0x8F).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree
            .insert(RangeInclusive::new(0x70, 0x7F).unwrap(), NodeState::Free)
            .unwrap();
        let _ = tree
            .insert(RangeInclusive::new(0x60, 0x6F).unwrap(), NodeState::Free)
            .unwrap();
    }

    #[test]
    fn test_tree_insert_intersect_negative() {
        let mut tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x100, 0x200).unwrap(),
            NodeState::Allocated,
        ));
        tree = tree
            .insert(RangeInclusive::new(0x201, 0x2FF).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        let res = tree
            .clone()
            .insert(RangeInclusive::new(0x201, 0x2FE).unwrap(), NodeState::Free);
        assert_eq!(
            res.unwrap_err(),
            Error::Overlap(
                RangeInclusive::new(0x201, 0x2FE).unwrap(),
                RangeInclusive::new(0x201, 0x2FF).unwrap()
            )
        );
        tree = tree
            .insert(RangeInclusive::new(0x90, 0x9F).unwrap(), NodeState::Free)
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        let res = tree.insert(RangeInclusive::new(0x90, 0x9E).unwrap(), NodeState::Free);
        assert_eq!(
            res.unwrap_err(),
            Error::Overlap(
                RangeInclusive::new(0x90, 0x9E).unwrap(),
                RangeInclusive::new(0x90, 0x9F).unwrap()
            )
        );
    }

    #[test]
    fn test_tree_insert_duplicate_negative() {
        let range = RangeInclusive::new(0x100, 0x200).unwrap();
        let tree = Box::new(InnerNode::new(range, NodeState::Allocated));
        let res = tree.insert(range, NodeState::Free);
        assert_eq!(res.unwrap_err(), Error::Overlap(range, range));
    }

    #[test]
    fn test_tree_stack_overflow_negative() {
        let mut inner_node = InnerNode::new(
            RangeInclusive::new(0x100, 0x200).unwrap(),
            NodeState::Allocated,
        );
        inner_node.height = 50;
        let tree = Box::new(inner_node);
        let res = tree.insert(RangeInclusive::new(0x100, 0x200).unwrap(), NodeState::Free);
        assert_eq!(res.unwrap_err(), Error::Overflow);
    }

    #[test]
    fn test_tree_mark_as_allocated_invalid_transition() {
        let range = RangeInclusive::new(0x100, 0x110).unwrap();
        let mut tree = Box::new(InnerNode::new(range, NodeState::Allocated));
        assert_eq!(
            tree.mark_as_allocated(&range).unwrap_err(),
            Error::InvalidStateTransition(range, NodeState::Allocated)
        );
    }

    #[test]
    fn test_tree_mark_as_allocated_resource_not_available() {
        let range = RangeInclusive::new(0x100, 0x110).unwrap();
        let mut tree = Box::new(InnerNode::new(range, NodeState::Allocated));
        assert_eq!(
            tree.mark_as_allocated(&RangeInclusive::new(0x111, 0x112).unwrap())
                .unwrap_err(),
            Error::ResourceNotAvailable
        );
        assert_eq!(
            tree.mark_as_allocated(&RangeInclusive::new(0x90, 0x92).unwrap())
                .unwrap_err(),
            Error::ResourceNotAvailable
        );
    }

    #[test]
    fn test_tree_mark_as_allocated() {
        let range = RangeInclusive::new(0x100, 0x110).unwrap();
        let range2 = RangeInclusive::new(0x200, 0x2FF).unwrap();
        let mut tree = Box::new(InnerNode::new(range, NodeState::Allocated));
        tree = tree.insert(range2, NodeState::Free).unwrap();
        assert!(tree.mark_as_allocated(&range2).is_ok());
        assert_eq!(
            *tree.search(&range2).unwrap(),
            InnerNode::new(range2, NodeState::Allocated)
        );
    }

    #[test]
    fn test_tree_delete() {
        let left_child =
            InnerNode::new(RangeInclusive::new(0x100, 0x110).unwrap(), NodeState::Free);
        let right_child =
            InnerNode::new(RangeInclusive::new(0x300, 0x3FF).unwrap(), NodeState::Free);
        let mut tree = Box::new(InnerNode::new(
            RangeInclusive::new(0x200, 0x290).unwrap(),
            NodeState::Free,
        ));
        tree = tree
            .insert(right_child.key, right_child.node_state)
            .unwrap();
        tree = tree
            .delete(&RangeInclusive::new(0x200, 0x290).unwrap())
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        tree = tree
            .insert(RangeInclusive::new(0x200, 0x290).unwrap(), NodeState::Free)
            .unwrap();
        tree = tree.insert(left_child.key, left_child.node_state).unwrap();
        assert!(is_balanced(Some(tree.clone())));

        assert_eq!(
            *tree
                .search(&RangeInclusive::new(0x100, 0x110).unwrap())
                .unwrap(),
            left_child
        );
        assert_eq!(*tree.search(&right_child.key).unwrap(), right_child);

        tree = tree
            .delete(&RangeInclusive::new(0x200, 0x290).unwrap())
            .unwrap();
        tree = tree
            .delete(&RangeInclusive::new(0x300, 0x3FF).unwrap())
            .unwrap();
        assert!(is_balanced(Some(tree.clone())));
        assert_eq!(
            *tree
                .search(&RangeInclusive::new(0x100, 0x110).unwrap())
                .unwrap(),
            left_child
        );
    }

    #[test]
    fn test_integer_wrapping() {
        let mut tree = IntervalTree::new(RangeInclusive::new(0x1, 0xFFFFFFFFFFFFFFFF).unwrap());

        // We have to create a valid constraint (that has an alignment that is a power of 2).
        // In case the size + the start address would overflow, we want to make sure the appropriate error is returned.
        let constraint = Constraint::new(
            0x8000000000000000,
            0x8000000000000000,
            AllocPolicy::ExactMatch(0x8000000000000000),
        )
        .unwrap();
        let res = tree.allocate(constraint);
        assert_eq!(res.unwrap_err(), Error::Overflow);
    }

    #[test]
    fn test_exact_match_inside_allocated() {
        let mut tree = IntervalTree::new(RangeInclusive::new(0x1000, 0x1FFF).unwrap());
        tree.allocate(Constraint::new(0x800, 0x100, AllocPolicy::FirstMatch).unwrap())
            .unwrap();
        let constraint = Constraint::new(0x100, 0x100, AllocPolicy::ExactMatch(0x1200)).unwrap();
        assert_eq!(
            tree.allocate(constraint).unwrap_err(),
            Error::ResourceNotAvailable
        );
    }

    #[test]
    fn test_exact_match_last_address() {
        let mut tree = IntervalTree::new(RangeInclusive::new(0x1000, 0x1FFF).unwrap());
        let constraint = Constraint::new(0x1, 0x1, AllocPolicy::ExactMatch(0x1FFF)).unwrap();
        assert_eq!(
            tree.allocate(constraint).unwrap(),
            RangeInclusive::new(0x1FFF, 0x1FFF).unwrap()
        );
    }
}
