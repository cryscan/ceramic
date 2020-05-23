use amethyst::{
    core::{math::{Matrix4, Point3}, Transform},
    ecs::prelude::*,
};

pub trait Helper {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn global_position(&self, entity: Entity) -> Point3<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl Helper for WriteStorage<'_, Transform> {
    #[inline]
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected `Transform` component")
            .global_matrix()
    }

    fn global_position(&self, entity: Entity) -> Point3<f32> {
        self
            .global_transform(entity)
            .transform_point(&Point3::origin())
    }

    #[inline]
    fn local_transform(&self, entity: Entity) -> Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected `Transform` component")
            .global_view_matrix()
    }
}