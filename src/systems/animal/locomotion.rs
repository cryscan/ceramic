use std::f32::{consts::{FRAC_PI_2, PI}, EPSILON};

use amethyst::{
    core::{math::{Complex, Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use itertools::multizip;
use num_traits::Zero;

use crate::{
    systems::{
        animal::{Quadruped, State},
        player::Player,
    },
    utils::transform::Helper,
};

#[derive(Default, SystemDesc)]
pub struct LocomotionSystem;

impl<'a> System<'a> for LocomotionSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, Quadruped>,
        ReadStorage<'a, Player>,
        Read<'a, Time>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut transforms,
            mut quadrupeds,
            players,
            time,
            mut debug_lines,
        ) = data;
        let delta_seconds = time.delta_seconds();

        for (entity, quadruped, player) in (&*entities, &mut quadrupeds, &players).join() {
            for (limb, signal, previous) in
            multizip((&mut quadruped.limbs, &quadruped.signals, &quadruped.previous)) {
                if limb.home.is_none() {
                    let ref foot = transforms.global_position(limb.foot);
                    let home = transforms.local_transform(limb.anchor).transform_point(foot);
                    limb.home.replace(home);
                }

                /*
                if limb.length.is_none() {
                    let foot = transforms.global_position(limb.foot);
                    let anchor = transforms.global_position(limb.anchor);
                    let length = (foot - anchor).norm();
                    limb.length.replace(length);
                }
                 */

                if let Some(ref home) = limb.home {
                    let home = transforms.global_transform(limb.anchor).transform_point(home);

                    let ref foot = transforms.global_position(limb.foot);
                    let ref anchor = transforms.global_position(limb.anchor);
                    let delta = foot - home;

                    let velocity = {
                        let root = transforms.global_position(entity);
                        let ref radial = home - root;
                        let ref angular = player.rotation().scaled_axis();
                        let ref linear = player.velocity();
                        let transform = transforms.global_transform(entity);
                        let angular = transform.transform_vector(angular);
                        let linear = transform.transform_vector(linear);
                        linear + angular.cross(radial)
                    };
                    let speed = velocity.norm();
                    limb.match_speed(speed);

                    let step_radius = limb.step_radius();
                    let flight_time = limb.flight_time();

                    {
                        let color = Srgba::new(0.0, 1.0, 0.0, limb.duty_factor);
                        debug_lines.draw_rotated_circle(
                            home.clone(),
                            step_radius,
                            10,
                            UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0),
                            color,
                        );

                        let color = Srgba::new(1.0, 1.0, 0.0, 1.0);
                        debug_lines.draw_direction(home.clone(), delta.clone(), color);

                        let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                        let ref direction = Vector3::new(0.0, signal.im, -signal.re).scale(step_radius);
                        let direction = transforms.global_transform(limb.foot).transform_vector(direction);
                        debug_lines.draw_direction(home.clone(), direction, color);
                    }

                    limb.state = match &limb.state {
                        State::Stance => {
                            let condition = {
                                let step = delta.norm() > step_radius;
                                let signal = signal.re > 0.0 && signal.im > 0.0 && previous.im < 0.0;
                                if limb.angular_velocity > limb.threshold { step || signal } else { step }
                            };
                            if condition {
                                let stance = foot.clone();
                                State::Flight { stance, time: 0.0 }
                            } else {
                                State::Stance
                            }
                        }
                        State::Flight { stance, time } => {
                            let time = *time;

                            let direction = velocity.try_normalize(EPSILON).unwrap_or(Vector3::zero());
                            let mut next = home;
                            if limb.angular_velocity > limb.threshold {
                                next += velocity * (flight_time - time) + direction * step_radius;
                            }
                            // Todo: Change this after ray casting works.
                            next.coords.y = limb.config.stance_height;

                            if time < flight_time {
                                let ref stance = stance.coords;
                                let direction = {
                                    let ref delta = anchor - foot;
                                    delta - direction.scale(direction.dot(delta))
                                }
                                    .try_normalize(EPSILON)
                                    .unwrap_or(Vector3::zero());
                                let step_length = step_radius * 2.0;
                                let height = limb.config.flight_height * step_length;
                                let center = Point3::from(next.coords.lerp(stance, 0.5)) + direction * height;
                                let current = {
                                    let ref center = center.coords;
                                    let ref next = next.coords;

                                    let factor = time / flight_time;
                                    let first = stance.lerp(center, factor);
                                    let ref second = center.lerp(next, factor);
                                    first.lerp(second, factor)
                                };

                                let rotation = transforms
                                    .get(entity)
                                    .unwrap()
                                    .rotation()
                                    .clone();
                                transforms
                                    .get_mut(limb.foot)
                                    .unwrap()
                                    .set_translation(current)
                                    .set_rotation(rotation);

                                State::Flight { stance: stance.xyz().into(), time: delta_seconds + time }
                            } else {
                                State::Stance
                            }
                        }
                    }
                }
            }

            quadruped.previous = quadruped.signals;

            const WEIGHTS: [[f32; 4]; 4] = [
                [0.0, 1.0, 0.0, 1.0],
                [1.0, 0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0, 1.0],
                [1.0, 0.0, 1.0, 0.0],
            ];
            const DIAGONAL_PHASES: [[f32; 4]; 4] = [
                [0.0, PI, 0.0, FRAC_PI_2],
                [-PI, 0.0, FRAC_PI_2, 0.0],
                [0.0, -FRAC_PI_2, 0.0, PI],
                [-FRAC_PI_2, 0.0, -PI, 0.0],
            ];
            const TROT_PHASES: [[f32; 4]; 4] = [
                [0.0, PI, 0.0, PI],
                [-PI, 0.0, PI, 0.0],
                [0.0, -PI, 0.0, PI],
                [-PI, 0.0, -PI, 0.0],
            ];
            const GALLOP_PHASES: [[f32; 4]; 4] = [
                [0.0, FRAC_PI_2, 0.0, 3.0 * FRAC_PI_2],
                [-FRAC_PI_2, 0.0, 3.0 * FRAC_PI_2, 0.0],
                [0.0, -3.0 * FRAC_PI_2, 0.0, FRAC_PI_2],
                [-3.0 * FRAC_PI_2, 0.0, -FRAC_PI_2, 0.0],
            ];

            for (i, (limb, signal)) in quadruped.limbs.iter_mut()
                .zip(quadruped.signals.iter_mut())
                .enumerate() {
                let angular_velocity = limb.angular_velocity;
                let duty_factor = limb.duty_factor;
                let omega = if signal.im < 0.0 {
                    angular_velocity / duty_factor / 2.0
                } else {
                    angular_velocity / (1.0 - duty_factor) / 2.0
                };

                let mut derivative = signal.scale(1.0 - signal.norm_sqr()) * PI;
                derivative.re -= omega * signal.im;
                derivative.im += omega * signal.re;

                for (j, signal) in quadruped.previous.iter()
                    .enumerate() {
                    let weight = WEIGHTS[i][j];
                    let ref phi = if duty_factor > 0.5 {
                        let trot = TROT_PHASES[i][j];
                        let diagonal = DIAGONAL_PHASES[i][j];
                        let factor = (duty_factor - 0.5) / 0.5;
                        trot * factor + diagonal * (1.0 - factor)
                    } else {
                        let gallop = GALLOP_PHASES[i][j];
                        let trot = TROT_PHASES[i][j];
                        let factor = duty_factor / 0.5;
                        gallop * factor + trot * (1.0 - factor)
                    };

                    let delta = weight * signal * Complex::from_polar(&1.0, phi);
                    derivative += delta;
                }

                *signal += derivative.scale(delta_seconds);
            }
        }
    }
}