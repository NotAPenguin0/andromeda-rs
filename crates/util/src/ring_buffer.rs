use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub struct Iter<'a, T> {
    ptr: *const T,
    index: usize,
    last_index: usize,
    size: usize,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.last_index {
            None
        } else {
            // SAFETY: We just checked self.index is not the last index, and because of the wrapping behaviour we
            // are always in range.
            let item = unsafe { self.ptr.add(self.index).as_ref().unwrap() };
            self.index = (self.index + 1) % self.size;
            Some(item)
        }
    }
}

pub struct RingBuffer<T, const SIZE: usize> {
    buffer: [T; SIZE],
    current: usize,
}

impl<T: Default + Copy, const SIZE: usize> Default for RingBuffer<T, SIZE> {
    fn default() -> Self {
        Self {
            buffer: [T::default(); SIZE],
            current: 0,
        }
    }
}

impl<T, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn new(values: [T; SIZE]) -> Self {
        Self {
            buffer: values,
            current: 0,
        }
    }

    pub fn current(&self) -> &T {
        self.buffer.get(self.current).unwrap()
    }

    pub fn current_mut(&mut self) -> &mut T {
        self.buffer.get_mut(self.current).unwrap()
    }

    pub fn next(&mut self) {
        self.current = (self.current + 1) % SIZE;
    }

    pub fn iter(&self) -> Iter<'_, T> {
        let last = if self.current == 0 {
            SIZE - 1
        } else {
            self.current - 1
        };
        Iter {
            ptr: self.buffer.as_ptr(),
            index: self.current,
            last_index: last,
            size: SIZE,
            _marker: PhantomData,
        }
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Iterate over the values, starting at the value that has not been returned from current()
    /// in the longest time. (So starting at old values)
    pub fn iter_fifo(&self) -> Iter<'_, T> {
        let start = (self.current + 1) % SIZE;
        let last = if start == 0 {
            SIZE - 1
        } else {
            start - 1
        };

        Iter {
            ptr: self.buffer.as_ptr(),
            index: start,
            last_index: last,
            size: SIZE,
            _marker: PhantomData,
        }
    }
}

impl<T: Debug, const SIZE: usize> Debug for RingBuffer<T, SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RingBuffer (current = {:?}, items = {:?})", self.current, self.buffer)
    }
}
