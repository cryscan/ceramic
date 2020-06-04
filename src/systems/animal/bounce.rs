use amethyst::{
    core::{math::Vector3, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use easer::functions::{Easing, Linear};
use num_traits::Zero;

use crate::{
    systems::player::Player,
    utils::{match_shape, transform::TransformStorageTrait},
};

use super::{limb_velocity, Quadruped, State};

#[derive(Default, SystemDesc)]
pub struct BounceSystem;

impl<'a> System<'a> for BounceSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, Quadruped>,
        ReadStorage<'a, Player>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut transforms,
            mut quadrupeds,
            players,
            mut debug_lines
        ) = data;
        for (entity, quadruped, player) in (&*entities, &mut quadrupeds, &players).join() {
            let mut anchors = Vec::new();
            let mut origins = Vec::new();

            for limb in quadruped.limbs.iter_mut() {
                let origin = transforms.global_position(limb.origin);
                let mut anchor = origin.clone();

                let length = anchor.y - limb.config.stance_height;
                let max_step_radius = limb.config.step_limit[1] / 2.0;
                let baseline = (length * length - max_step_radius * max_step_radius).sqrt();

                let velocity = limb_velocity(&transforms, entity, limb, player);
                let speed = velocity.norm();
                let [_, max_speed] = player.speed_limit();
                let height = Linear::ease_in_out(speed, length, baseline - length, max_speed);
                anchor.y = limb.config.stance_height + height;

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

            let (translation, rotation) = match_shape(origins, anchors, 0.01, 10);
            transforms
                .get_mut(quadruped.root)
                .unwrap()
                .set_translation(translation)
                .set_rotation(rotation);
        }
    }
}