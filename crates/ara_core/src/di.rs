use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
    sync::Arc,
};
use parking_lot::RwLock;

pub trait Construct: Sized {
    fn construct(inj: &Injector) -> Self;
}

#[derive(Clone, Default)]
pub struct Injector {
    instances: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Injector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create<T: Construct + Send + Sync + 'static>(&self) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        if let Some(existing) = self.instances.read().get(&type_id)
            && let Ok(downcasted) = Arc::clone(existing).downcast::<T>()
        {
            return downcasted;
        }

        let created = Arc::new(T::construct(self));
        let mut guard = self.instances.write();
        let entry = guard
            .entry(type_id)
            .or_insert_with(|| Arc::clone(&created) as Arc<dyn Any + Send + Sync>);
        Arc::clone(entry)
            .downcast::<T>()
            .expect("TypeId mismatch in Injector")
    }
}

pub struct Inject<T: 'static> {
    inner: Arc<T>,
}

impl<T: 'static> Inject<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }

    pub fn into_arc(self) -> Arc<T> {
        self.inner
    }
}

impl<T: 'static> Clone for Inject<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: 'static> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Construct + Send + Sync + 'static> Construct for Inject<T> {
    fn construct(inj: &Injector) -> Self {
        Self {
            inner: inj.get_or_create::<T>(),
        }
    }
}
