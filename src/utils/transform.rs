use amethyst::core::{math::Point3, Transform};

pub trait TransformTrait {
    fn global_position(&self) -> Point3<f32>;
}

impl TransformTrait for Transform {
    fn global_position(&self) -> Point3<f32> {
        let ref origin = Point3::origin();
        self.global_matrix().transform_point(origin)
    }
}