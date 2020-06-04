use std::f32::{consts::{FRAC_PI_2, FRAC_PI_4, PI}, EPSILON};

use amethyst::{
    core::{math::{Complex, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use amethyst_physics::PhysicsTime;
use easer::functions::{Cubic, Easing, Sine};
use interpolation::Lerp;
use itertools::Itertools;
use num_traits::Zero;

use crate::{
    systems::player::Player,
    utils::transform::TransformStorageTrait,
};

use super::{limb_velocity, Quadruped, State};

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
            for limb in quadruped.limbs.iter_mut() {
                let ref home = transforms.global_position(limb.home);
                let ref foot = transforms.global_position(limb.foot);
                let ref root = transforms.global_position(limb.root);
                let delta = foot - home;

                let velocity = limb_velocity(&transforms, entity, limb, player);
                let speed = velocity.norm();
                limb.match_speed(speed);

                let step_radius = limb.step_radius();
                let flight_time = limb.flight_time();

                {
                    let mut home = home.clone();
                    home.coords.y = limb.config.stance_height;

                    let color = Srgba::new(0.0, 1.0, 0.0, limb.duty_factor);
                    debug_lines.draw_rotated_circle(
                        home.clone(),
                        step_radius,
                        10,
                        UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0),
                        color,
                    );

                    let color = Srgba::new(1.0, 1.0, 0.0, 1.0);
                    debug_lines.draw_sphere(foot.clone(), 0.2, 4, 4, color);

                    let signal = limb.signal;
                    let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                    let ref direction = Vector3::new(0.0, signal.im, -signal.re).scale(step_radius);
                    let direction = transforms.global_transform(limb.foot).transform_vector(direction);
                    debug_lines.draw_direction(home, direction, color);
                }

                limb.state = match &limb.state {
                    State::Stance => {
                        let condition = {
                            if limb.angular_velocity > limb.threshold {
                                let transition = limb.transition;
                                limb.transition = false;
                                transition
                            } else {
                                delta.norm() > step_radius
                            }
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
                        let mut next = home.clone();
                        if limb.angular_velocity > limb.threshold {
                            next += velocity * (flight_time - time) + direction * step_radius;
                        }
                        next.coords.y = limb.config.stance_height;

                        {
                            let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                            debug_lines.draw_sphere(next.clone(), 0.1, 4, 4, color);
                        }

                        if time < flight_time {
                            let ref stance = stance.coords;
                            let ref next = next.coords;

                            let direction = {
                                let ref delta = root - foot;
                                let direction = delta - direction.scale(direction.dot(delta));
                                direction.try_normalize(EPSILON).unwrap_or(Vector3::zero())
                            };
                            let step_length = step_radius * 2.0;
                            let height = limb.config.flight_factor * step_length;

                            let factor = Sine::ease_in(time, 0.0, 1.0, flight_time);

                            let translation = {
                                let ref center = next.lerp(stance, 0.2) + direction * height;
                                let ref first = stance.lerp(center, factor);
                                let ref second = center.lerp(next, factor);
                                first.lerp(second, factor)
                            };

                            let rotation = transforms
                                .get(entity)
                                .unwrap()
                                .rotation()
                                .clone();

                            let ref factor = Cubic::ease_in_out(time, 0.0, 1.0, flight_time);
                            let angle = {
                                let max_step_length = limb.config.step_limit[1];
                                let ref center = FRAC_PI_2 * step_length / max_step_length;
                                let ref first = 0.0.lerp(center, factor);
                                let ref second = center.lerp(&0.0, factor);
                                first.lerp(second, factor)
                            };

                            transforms
                                .get_mut(limb.foot)
                                .unwrap()
                                .set_translation(translation)
                                .set_rotation(rotation)
                                .append_rotation_x_axis(angle);

                            State::Flight { stance: stance.xyz().into(), time: delta_seconds + time }
                        } else {
                            transforms
                                .get_mut(limb.foot)
                                .unwrap()
                                .set_translation(next.coords);
                            State::Stance
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default, SystemDesc)]
pub struct OscillatorSystem;

impl<'a> System<'a> for OscillatorSystem {
    type SystemData = (
        WriteStorage<'a, Quadruped>,
        Read<'a, PhysicsTime>,
    );

    fn run(&mut self, (mut quadrupeds, time): Self::SystemData) {
        for quadruped in (&mut quadrupeds).join() {
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
                [0.0, FRAC_PI_2, 0.0, -3.0 * FRAC_PI_4],
                [-FRAC_PI_2, 0.0, 3.0 * FRAC_PI_4, 0.0],
                [0.0, -3.0 * FRAC_PI_4, 0.0, 0.0],
                [3.0 * FRAC_PI_4, 0.0, 0.0, 0.0],
            ];

            let previous = quadruped.limbs.iter()
                .map(|limb| limb.signal)
                .collect_vec();
            for (i, limb) in quadruped.limbs.iter_mut().enumerate() {
                let ref mut signal = limb.signal;

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

                for (j, signal) in previous.iter().enumerate() {
                    let weight = WEIGHTS[i][j];
                    let ref phi = match duty_factor {
                        factor if factor > 0.5 => {
                            let trot = TROT_PHASES[i][j];
                            let ref diagonal = DIAGONAL_PHASES[i][j];
                            let ref factor = (duty_factor - 0.5) / 0.5;
                            trot.lerp(diagonal, factor)
                        }
                        factor if factor > 0.3 => {
                            let gallop = GALLOP_PHASES[i][j];
                            let ref trot = TROT_PHASES[i][j];
                            let ref factor = duty_factor / 0.5;
                            gallop.lerp(trot, factor)
                        }
                        _ => GALLOP_PHASES[i][j],
                    };

                    let delta = weight * signal * Complex::from_polar(&1.0, phi);
                    derivative += delta;
                }

                let previous = *signal;
                *signal += derivative.scale(time.delta_seconds());
                if signal.im > 0.0 && previous.im < 0.0 { limb.transition = true; }
            }
        }
    }
}