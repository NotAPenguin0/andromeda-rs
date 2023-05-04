use glam::Mat4;

pub trait ByteSize {
    fn byte_size(&self) -> usize;
}

impl<T, const N: usize> ByteSize for [T; N]
where
    T: Sized,
{
    fn byte_size(&self) -> usize {
        N * std::mem::size_of::<T>()
    }
}

impl<T> ByteSize for &[T] {
    fn byte_size(&self) -> usize {
        std::mem::size_of_val(*self)
    }
}

impl ByteSize for Mat4 {
    fn byte_size(&self) -> usize {
        16 * std::mem::size_of::<f32>()
    }
}
