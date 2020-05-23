use std::{
    convert::TryInto,
    f32::{consts::{FRAC_PI_2, PI, TAU}, EPSILON},
};

use amethyst::{
    assets::PrefabData,
    core::{math::{Complex, Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    Error,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use itertools::{Itertools, multizip};
use num_traits::identities::Zero;
use serde::{Deserialize, Serialize};

use crate::{
    systems::player::Player,
    utils::transform::Adaptor,
};

#[derive(Debug, Copy, Clone)]
pub struct Tracker {
    target: Entity,
    limit: Option<f32>,
    speed: f32,
    rotation: Option<UnitQuaternion<f32>>,
}

impl Component for Tracker {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TrackerPrefab {
    pub target: usize,
    pub limit: Option<f32>,
    pub speed: f32,
}

impl<'a> PrefabData<'a> for TrackerPrefab {
    type SystemData = WriteStorage<'a, Tracker>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity]) -> Result<Self::Result, Error> {
        let component = Tracker {
            target: entities[self.target],
            limit: self.limit.clone(),
            speed: self.speed,
            rotation: None,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Default, SystemDesc)]
pub struct TrackSystem;

impl<'a> System<'a> for TrackSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, Tracker>,
        Read<'a, Time>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut transforms,
            mut trackers,
            time,
        ) = data;

        for (tracker, transform) in (&mut trackers, &transforms).join() {
            if tracker.rotation.is_none() {
                let rotation = transform.rotation();
                tracker.rotation.replace(rotation.clone());
            }
        }

        for (entity, tracker) in (&*entities, &trackers).join() {
            let target = transforms.global_position(tracker.target);
            let joint = transforms.global_position(entity);
            let ref target = target - joint;

            let transform = transforms.local_transform(entity);
            let ref target = transform.transform_vector(target);
            let ref up = transform.transform_vector(&Vector3::y());

            // The hack here is that the direction of joints is y axis, not z axis by default.
            let mut target = UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0)
                * UnitQuaternion::face_towards(target, up);

            let rotation = tracker.rotation.unwrap_or_else(UnitQuaternion::identity);
            if let Some((axis, angle)) = (rotation.inverse() * target).axis_angle() {
                if let Some(limit) = tracker.limit {
                    let angle = angle.min(limit);
                    let delta = UnitQuaternion::from_axis_angle(&axis, angle);
                    target = delta * rotation * rotation;
                }
            }

            let current = transforms.get(entity).unwrap().rotation();
            let interpolation = 1.0 - (-tracker.speed * time.delta_seconds()).exp();
            if let Some(rotation) = current.try_slerp(&target, interpolation, EPSILON) {
                transforms.get_mut(entity).unwrap().set_rotation(rotation);
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum State {
    Stance,
    Flight { stance: Point3<f32>, time: f32 },
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_angular_velocity: f32,
    pub max_duty_factor: f32,
    pub step_limit: [f32; 2],
    pub flight_time: f32,
    pub flight_height: f32,
    pub stance_height: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Limb {
    foot: Entity,
    anchor: Entity,
    state: State,

    home: Option<Point3<f32>>,
    length: Option<f32>,

    radius: f32,
    angular_velocity: f32,

    /// The minimum angular velocity whose flight time is greater than `flight_time`.
    threshold: f32,
    duty_factor: f32,

    config: Config,
}

impl Limb {
    fn match_speed(&mut self, speed: f32) {
        let ref config = self.config;
        let [min_step, max_step] = config.step_limit;

        // Increase angular speed to be maximum, and then increase radius.
        let min_radius = min_step / config.max_duty_factor / TAU;
        self.angular_velocity = (speed / min_radius).min(config.max_angular_velocity);
        self.radius = if self.angular_velocity > 0.0 { speed / self.angular_velocity } else { min_radius };

        // The step length at this situation to ensure the maximum duty factor and the maximum step length.
        let step_length = (TAU * self.radius * config.max_duty_factor).min(max_step);
        self.duty_factor = step_length / (TAU * self.radius);
        self.threshold = TAU * (1.0 - config.max_duty_factor) / config.flight_time;
    }

    fn step_radius(&self) -> f32 {
        PI * self.radius * self.duty_factor
    }

    fn flight_time(&self) -> f32 {
        if self.angular_velocity > self.threshold {
            TAU * (1.0 - self.duty_factor) / self.angular_velocity
        } else {
            self.config.flight_time
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Quadruped {
    limbs: [Limb; 4],
    previous: [Complex<f32>; 4],
    signals: [Complex<f32>; 4],
}

impl Component for Quadruped {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct QuadrupedPrefab {
    pub feet: [usize; 4],
    pub anchors: [usize; 4],

    #[serde(flatten)]
    pub config: Config,
}

impl<'a> PrefabData<'a> for QuadrupedPrefab {
    type SystemData = WriteStorage<'a, Quadruped>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let limbs = self.feet.iter()
            .zip(self.anchors.iter())
            .map(|(&foot, &anchor)| Limb {
                foot: entities[foot],
                anchor: entities[anchor],
                state: State::Stance,
                home: None,
                length: None,

                radius: 0.0,
                angular_velocity: 0.0,
                threshold: 0.0,
                duty_factor: 0.0,

                config: self.config.clone(),
            })
            .collect_vec();
        let limbs = limbs[..].try_into().unwrap();
        let signals = (0..4)
            .map(|i| Complex::from_polar(&1.0, &(FRAC_PI_2 * i as f32)))
            .collect_vec();
        let signals = signals[..].try_into().unwrap();
        let component = Quadruped {
            limbs,
            signals,
            previous: signals,
        };

        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

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

                if limb.length.is_none() {
                    let foot = transforms.global_position(limb.foot);
                    let anchor = transforms.global_position(limb.anchor);
                    let length = (foot - anchor).norm();
                    limb.length.replace(length);
                }

                if let Some((ref home, _length)) = limb.home.zip(limb.length) {
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
                                if limb.angular_velocity > limb.threshold {
                                    delta.norm() > step_radius ||
                                        signal.im > 0.0 && previous.im < 0.0 && signal.re > 0.0
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