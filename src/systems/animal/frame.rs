use std::f32::EPSILON;

use amethyst::{
    core::{math::{Matrix1x4, Matrix3x4, UnitQuaternion, Vector3}, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use num_traits::Zero;

use crate::utils::transform::Helper;

use super::{Quadruped, State};

#[derive(Default, SystemDesc)]
pub struct FrameSystem;

impl<'a> System<'a> for FrameSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, Quadruped>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut transforms,
            mut quadrupeds,
            mut debug_lines
        ) = data;
        for (entity, quadruped) in (&*entities, &mut quadrupeds).join() {
            let mut anchors = Vec::new();
            let mut origins = Vec::new();

            for limb in quadruped.limbs.iter_mut() {
                if limb.origin.is_none() {
                    let ref anchor = transforms.global_position(limb.anchor);
                    limb.origin.replace(transforms.local_transform(entity).transform_point(anchor));
                }

                if let Some(ref origin) = limb.origin {
                    let origin = transforms.global_transform(entity).transform_point(origin);
                    let mut anchor = origin.clone();

                    let length = anchor.y - limb.config.stance_height;
                    let step_radius = limb.step_radius();
                    let baseline = (length * length - step_radius * step_radius).sqrt();
                    anchor.y = limb.config.stance_height + baseline;

                    let speed = limb.angular_velocity * limb.radius;
                    match limb.state {
                        State::Stance => {}
                        State::Flight { time, .. } => {
                            let flight_time = limb.flight_time();
                            let height = limb.config.bounce_factor * flight_time * speed;
                            let current = {
                                let factor = time / flight_time;
                                let ref center = Vector3::y() * height;
                                let ref origin = Vector3::zero();
                                let ref first = origin.lerp(center, factor);
                                let ref second = center.lerp(origin, factor);
                                first.lerp(second, factor)
                            };
                            anchor += current;
                        }
                    }
                    {
                        let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                        debug_lines.draw_sphere(anchor.clone(), 0.4, 4, 4, color);
                    }

                    anchors.append(&mut vec![anchor.x, anchor.y, anchor.z]);
                    origins.append(&mut vec![origin.x, origin.y, origin.z]);
                }
            }

            let anchors = Matrix3x4::from_vec(anchors);
            let origins = Matrix3x4::from_vec(origins);
            let anchors_mean = anchors.column_mean();
            let origins_mean = origins.column_mean();
            let translation = anchors_mean - origins_mean;

            let anchors = anchors - anchors_mean * Matrix1x4::repeat(1.0);
            let origins = origins - origins_mean * Matrix1x4::repeat(1.0);
            let ref covariance = origins * anchors.transpose();
            let rotation = UnitQuaternion::from_matrix_eps(covariance, EPSILON, 10, UnitQuaternion::identity());

            transforms
                .get_mut(quadruped.root)
                .unwrap()
                .set_translation(translation)
                .set_rotation(rotation);
        }
    }
}