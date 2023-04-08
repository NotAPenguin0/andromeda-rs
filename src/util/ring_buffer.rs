use std::fmt::{Debug, Formatter};

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
}

impl<T: Debug, const SIZE: usize> Debug for RingBuffer<T, SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RingBuffer (current = {:?}, items = {:?})", self.current, self.buffer)
    }
}
