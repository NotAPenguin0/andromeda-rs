use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use slotmap::{Key, KeyData};

pub struct Handle<A> {
    data: KeyData,
    _marker: PhantomData<A>,
}

impl<A> Debug for Handle<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

impl<A> From<KeyData> for Handle<A> {
    fn from(value: KeyData) -> Self {
        Self {
            data: value,
            _marker: PhantomData,
        }
    }
}

impl<A> Copy for Handle<A> {}

impl<A> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A> Default for Handle<A> {
    fn default() -> Self {
        Self {
            data: KeyData::default(),
            _marker: PhantomData,
        }
    }
}

impl<A> Eq for Handle<A> {}

impl<A> PartialEq<Self> for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

impl<A> Ord for Handle<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

impl<A> PartialOrd<Self> for Handle<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<A> Hash for Handle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

// SAFETY: This implementation is identical to the provided implementation, but we need
// a manual one because of the PhantomData marker
unsafe impl<A> Key for Handle<A> {
    fn data(&self) -> KeyData {
        self.data
    }
}
