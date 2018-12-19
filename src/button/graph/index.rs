// Copyright (c) 2018 Jason White
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
use std::fmt;
use std::hash::Hash;
use std::iter::FromIterator;
use std::marker::PhantomData;

use bit_set::{self, BitSet};

use holyhashmap::EntryIndex;

pub trait Index:
    Copy + Clone + Eq + PartialEq + Hash + From<usize> + Into<usize>
{
}

impl Index for usize {}

/// A type-safe node index.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeIndex(EntryIndex);

impl From<EntryIndex> for NodeIndex {
    fn from(index: EntryIndex) -> Self {
        NodeIndex(index)
    }
}

impl From<usize> for NodeIndex {
    fn from(index: usize) -> Self {
        NodeIndex(index.into())
    }
}

impl Into<EntryIndex> for NodeIndex {
    fn into(self) -> EntryIndex {
        self.0
    }
}

impl Into<usize> for NodeIndex {
    fn into(self) -> usize {
        self.0.into()
    }
}

impl fmt::Display for NodeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Index for NodeIndex {}

/// A type-safe edge index.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct EdgeIndex(EntryIndex);

impl From<EntryIndex> for EdgeIndex {
    fn from(index: EntryIndex) -> Self {
        EdgeIndex(index)
    }
}

impl From<usize> for EdgeIndex {
    fn from(index: usize) -> Self {
        EdgeIndex(index.into())
    }
}

impl Into<EntryIndex> for EdgeIndex {
    fn into(self) -> EntryIndex {
        self.0
    }
}

impl Into<usize> for EdgeIndex {
    fn into(self) -> usize {
        self.0.into()
    }
}

impl fmt::Display for EdgeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Index for EdgeIndex {}

/// A set of indices stored as a bitset.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct IndexSet<T> {
    set: BitSet,
    phantom: PhantomData<T>,
}

impl<T> Default for IndexSet<T> {
    #[inline]
    fn default() -> Self {
        IndexSet {
            set: BitSet::new(),
            phantom: PhantomData,
        }
    }
}

impl<T> IndexSet<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(nbits: usize) -> Self {
        IndexSet {
            set: BitSet::with_capacity(nbits),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.set.capacity()
    }

    #[inline]
    pub fn reserve_len(&mut self, len: usize) {
        self.set.reserve_len(len)
    }

    #[inline]
    pub fn reserve_len_exact(&mut self, len: usize) {
        self.set.reserve_len_exact(len)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.set.shrink_to_fit()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.set.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.set.clear()
    }

    #[inline]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.set.is_disjoint(&other.set)
    }

    #[inline]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.set.is_subset(&other.set)
    }

    #[inline]
    pub fn is_superset(&self, other: &Self) -> bool {
        self.set.is_superset(&other.set)
    }
}

impl<T> IndexSet<T>
where
    T: Index,
{
    #[inline]
    pub fn iter(&self) -> IndexSetIter<'_, T> {
        IndexSetIter {
            iter: self.set.iter(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn contains(&self, value: &T) -> bool {
        self.set.contains((*value).into())
    }

    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.set.insert(value.into())
    }

    #[inline]
    pub fn remove(&mut self, value: &T) -> bool {
        self.set.remove((*value).into())
    }
}

impl<T> FromIterator<T> for IndexSet<T>
where
    T: Index,
{
    #[inline]
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut ret = Self::default();
        ret.extend(iter);
        ret
    }
}

impl<T> Extend<T> for IndexSet<T>
where
    T: Index,
{
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.set.extend(iter.into_iter().map(T::into))
    }
}

impl<'a, T> IntoIterator for &'a IndexSet<T>
where
    T: Index,
{
    type Item = T;
    type IntoIter = IndexSetIter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct IndexSetIter<'a, T> {
    iter: bit_set::Iter<'a, u32>,
    phantom: PhantomData<T>,
}

impl<'a, T> Iterator for IndexSetIter<'a, T>
where
    T: Index,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(T::from)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
