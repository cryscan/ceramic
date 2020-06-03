use std::ops::Deref;

use amethyst::{
    core::{math::{Matrix4, Point3}, Transform},
    ecs::{prelude::*, storage::MaskedStorage},
};

pub trait TransformStorageExt {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn global_position(&self, entity: Entity) -> Point3<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl<D> TransformStorageExt for Storage<'_, Transform, D>
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