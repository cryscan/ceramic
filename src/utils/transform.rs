use amethyst::{
    core::{math::{Matrix4, Point3}, Transform},
    ecs::prelude::*,
};

pub trait Adaptor {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn global_position(&self, entity: Entity) -> Point3<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl Adaptor for WriteStorage<'_, Transform> {
    #[inline]
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected to have `Transform`")
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
            .expect("Expected to have `Transform`")
            .global_view_matrix()
    }
}