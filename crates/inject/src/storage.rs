use std::any::{Any, TypeId};
use std::boxed::ThinBox;
use std::collections::HashMap;
use std::marker::Unsize;
use std::ops::{Deref, DerefMut};

use util::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A registry is a container for type-erased structs. It can store
/// any struct, or any `dyn Trait` object, which can then be queried again by calling
/// `get::<T>()` for a regular struct or `get_dyn::<dyn Trait>()` for trait objects.
#[derive(Debug, Default)]
pub struct ErasedStorage {
    dyn_items: HashMap<TypeId, ThinBox<dyn Any>>,
    items: HashMap<TypeId, Box<dyn Any>>,
}

impl ErasedStorage {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            dyn_items: HashMap::default(),
            items: HashMap::default(),
        }
    }

    fn put_dyn_boxed<T: ?Sized + 'static>(&mut self, item: ThinBox<T>) {
        // SAFETY: ThinBox always has the same size regardless of the type inside,
        // so we can transmute this to a different pointer until we cast it back to
        // T in get()
        let any = unsafe { std::mem::transmute::<_, ThinBox<dyn Any>>(item) };
        self.dyn_items.insert(TypeId::of::<T>(), any);
    }

    /// Put a static type `T` into the registry. This can then be retrieved back
    /// by calling [`Self::get::<T>()`]
    pub fn put<T: 'static>(&mut self, item: T) {
        self.items.insert(TypeId::of::<T>(), Box::new(item));
    }

    /// Put a static type T into the registry, with an additional lock around it.
    pub fn put_sync<T: 'static>(&mut self, item: T) {
        self.put(RwLock::with_name(item, std::any::type_name::<T>()));
    }

    /// Put a trait object into the registry. If called with `dyn MyTrait`, this takes in
    /// any `Foo: MyTrait`, which is then moved into the registry and can be queried back with
    /// [`Self::get_dyn::<dyn MyTrait>()`]
    pub fn put_dyn<T: ?Sized + 'static>(&mut self, item: impl Unsize<T>) {
        self.put_dyn_boxed(ThinBox::<T>::new_unsize(item));
    }

    /// Get the registered object for `T`, or `None` if it didn't exist.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let any = self.items.get(&TypeId::of::<T>());
        any.map(|value| value.downcast_ref::<T>().unwrap())
    }

    /// Acquire a reader lock to a synchronized object stored in the registry
    pub fn read_sync<T: 'static>(&self) -> Option<RwLockReadGuard<T>> {
        self.get::<RwLock<T>>().map(|lock| lock.read().unwrap())
    }

    /// Acquire a writer lock to a synchronized object stored in the registry
    pub fn write_sync<T: 'static>(&self) -> Option<RwLockWriteGuard<T>> {
        self.get::<RwLock<T>>().map(|lock| lock.write().unwrap())
    }

    /// Get a mutable reference to the registered object for `T`, or `None` if it didn't exist.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let any = self.items.get_mut(&TypeId::of::<T>());
        any.map(|value| value.downcast_mut::<T>().unwrap())
    }

    /// Get the registered implementation for `dyn MyTrait`, or `None` if it didn't exist.
    pub fn get_dyn<T: ?Sized + 'static>(&self) -> Option<&T> {
        let any = self.dyn_items.get(&TypeId::of::<T>());
        any.map(|any| unsafe { std::mem::transmute::<_, &ThinBox<T>>(any) }.deref())
    }

    /// Get a mutable reference to the registered implementation for `dyn MyTrait`, or `None` if it didn't exist.
    pub fn get_dyn_mut<T: ?Sized + 'static>(&mut self) -> Option<&mut T> {
        let any = self.dyn_items.get_mut(&TypeId::of::<T>());
        any.map(|any| unsafe { std::mem::transmute::<_, &mut ThinBox<T>>(any) }.deref_mut())
    }
}

#[cfg(test)]
mod tests {
    use crate::ErasedStorage;

    struct Foo;

    trait MyTrait {
        fn call(&self);
    }

    impl MyTrait for Foo {
        fn call(&self) {
            assert!(true);
        }
    }

    #[test]
    fn put_static() {
        let mut registry = ErasedStorage::new();
        registry.put(Foo);
        assert!(registry.get::<Foo>().is_some());
    }

    #[test]
    fn put_dyn() {
        let mut registry = ErasedStorage::new();
        registry.put_dyn::<dyn MyTrait>(Foo);
        assert!(registry.get_dyn::<dyn MyTrait>().is_some());
    }
}
