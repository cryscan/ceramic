use std::ops::Deref;

use amethyst::{
    core::{math::{Matrix4, Point3}, Transform},
    ecs::{prelude::*, storage::MaskedStorage},
};

pub trait TransformStorageTrait {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn global_position(&self, entity: Entity) -> Point3<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl<D> TransformStorageTrait for Storage<'_, Transform, D>
    where D: Deref<Target=MaskedStorage<Transform>> {
    #[inline]
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected `Transform` component")
            .global_matrix()
    }

    fn global_position(&self, entity: Entity) -> Point3<f32> {
        let ref origin = Point3::origin();
        self
            .global_transform(entity)
            .transform_point(origin)
    }

    #[inline]
    fn local_transform(&self, entity: Entity) -> Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected `Transform` component")
            .global_view_matrix()
    }
}

pub trait StorageTrait<T: Clone> {
    fn get_multiple_cloned(&self, entities: &[Entity]) -> Option<Vec<T>>;
}

impl<T, D> StorageTrait<T> for Storage<'_, T, D>
    where T: Component + Clone,
          D: Deref<Target=MaskedStorage<T>> {
    fn get_multiple_cloned(&self, entities: &[Entity]) -> Option<Vec<T>> {
        entities
            .iter()
            .map(|entity| self.get(*entity).cloned())
            .collect()
    }
}

pub trait TransformTrait {
    fn global_position(&self) -> Point3<f32>;
}

impl TransformTrait for Transform {
    fn global_position(&self) -> Point3<f32> {
        let ref origin = Point3::origin();
        self.global_matrix().transform_point(origin)
    }
}