use amethyst::{
    core::{math::Matrix4, Transform},
    ecs::prelude::*,
};

pub trait Getter {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl Getter for WriteStorage<'_, Transform> {
    #[inline]
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected to have `Transform`")
            .global_matrix()
    }

    #[inline]
    fn local_transform(&self, entity: Entity) -> Matrix4<f32> {
        self
            .get(entity)
            .expect("Expected to have `Transform`")
            .global_view_matrix()
    }
}