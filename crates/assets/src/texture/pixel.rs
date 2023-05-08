use std::ops::{Deref, DerefMut};

use bytemuck::{Pod, Zeroable};

pub trait Primitive: Pod {}

impl<T: Pod> Primitive for T {}

#[derive(Copy, Clone, Zeroable)]
#[repr(C)]
pub struct LumaPixel<T: Primitive>(T);

#[derive(Copy, Clone, Zeroable)]
#[repr(C)]
pub struct RgbPixel<T: Primitive>([T; 3]);

#[derive(Copy, Clone, Zeroable)]
#[repr(C)]
pub struct RgbaPixel<T: Primitive>([T; 4]);

// SAFETY: These structs are simple wrappers over arrays of T, where T is also Pod.

unsafe impl<T: Primitive> Pod for LumaPixel<T> {}

unsafe impl<T: Primitive> Pod for RgbPixel<T> {}

unsafe impl<T: Primitive> Pod for RgbaPixel<T> {}

pub trait Pixel: Pod {
    type SubPixel: Primitive;
}

impl<T: Primitive> Pixel for LumaPixel<T> {
    type SubPixel = T;
}

impl<T: Primitive> Pixel for RgbPixel<T> {
    type SubPixel = T;
}

impl<T: Primitive> Pixel for RgbaPixel<T> {
    type SubPixel = T;
}

impl<T: Primitive> Deref for LumaPixel<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Primitive> DerefMut for LumaPixel<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
