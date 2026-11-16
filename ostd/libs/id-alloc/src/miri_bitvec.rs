// SPDX-License-Identifier: MPL-2.0

//! A word-based bitmap used in place of `bitvec::BitVec` when running under
//! (K)Miri.
//!
//! `bitvec` 1.1.1 rebuilds `&mut [u8]` views of its storage from pointers that
//! went through a zero-size retag (`BitSlice::fill` -> `domain_mut` ->
//! `Domain::spanning` -> `slice::from_raw_parts_mut`). Stacked Borrows rejects
//! that, so any OSTD code path that allocates an ID (e.g. `IrqLine::alloc`)
//! stops with undefined behavior inside `bitvec` before reaching OSTD itself.
//! The problem is in the third-party crate, not in OSTD, so only Miri builds
//! use this replacement; all other builds keep using `bitvec`.
//!
//! Only the subset of the `BitVec` API that `IdAlloc` needs is provided.

extern crate alloc;

// The crate is `no_std` except in unit tests, where `Vec` is in the prelude.
#[cfg(not(test))]
use alloc::vec::Vec;
use core::{
    marker::PhantomData,
    ops::{Index, Range},
};

const BITS_PER_WORD: usize = u64::BITS as usize;

#[derive(Clone)]
pub(crate) struct BitVec<T> {
    words: Vec<u64>,
    len: usize,
    _store: PhantomData<T>,
}

impl<T> BitVec<T> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            words: Vec::with_capacity(capacity.div_ceil(BITS_PER_WORD)),
            len: 0,
            _store: PhantomData,
        }
    }

    pub(crate) fn resize(&mut self, new_len: usize, value: bool) {
        let old_len = self.len;
        self.words.resize(new_len.div_ceil(BITS_PER_WORD), 0);
        self.len = new_len;
        // Whole words are filled at once. Setting bits one by one takes
        // minutes under Miri for a large ID space such as the 2^20 block
        // device IDs.
        let whole_start = old_len.div_ceil(BITS_PER_WORD);
        let whole_end = new_len / BITS_PER_WORD;
        if whole_start < whole_end {
            for id in old_len..whole_start * BITS_PER_WORD {
                self.set(id, value);
            }
            self.words[whole_start..whole_end].fill(if value { u64::MAX } else { 0 });
            for id in whole_end * BITS_PER_WORD..new_len {
                self.set(id, value);
            }
        } else {
            for id in old_len..new_len {
                self.set(id, value);
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn set(&mut self, id: usize, value: bool) {
        assert!(id < self.len, "bit index out of bounds");
        let word = &mut self.words[id / BITS_PER_WORD];
        let mask = 1u64 << (id % BITS_PER_WORD);
        if value {
            *word |= mask;
        } else {
            *word &= !mask;
        }
    }

    fn bit(&self, id: usize) -> bool {
        assert!(id < self.len, "bit index out of bounds");
        self.words[id / BITS_PER_WORD] & (1u64 << (id % BITS_PER_WORD)) != 0
    }

    pub(crate) fn get(&self, range: Range<usize>) -> Option<BitRange<'_, T>> {
        (range.start <= range.end && range.end <= self.len)
            .then_some(BitRange { bits: self, range })
    }
}

impl<T> Index<usize> for BitVec<T> {
    type Output = bool;

    fn index(&self, id: usize) -> &bool {
        if self.bit(id) { &true } else { &false }
    }
}

/// A read-only view of a range of bits, like `&BitSlice`.
pub(crate) struct BitRange<'a, T> {
    bits: &'a BitVec<T>,
    range: Range<usize>,
}

impl<T> BitRange<'_, T> {
    /// Returns the offset (relative to the start of the range) of the first
    /// zero bit.
    pub(crate) fn first_zero(&self) -> Option<usize> {
        self.range.clone().position(|id| !self.bits.bit(id))
    }
}
