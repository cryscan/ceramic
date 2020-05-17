use std::{
    convert::TryInto,
    f32::{consts::{FRAC_PI_2, PI, TAU}, EPSILON},
};

use amethyst::{
    assets::PrefabData,
    core::{math::{Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    Error,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use itertools::Itertools;
use num_traits::identities::Zero;
use serde::{Deserialize, Serialize};

use crate::{
    system::{
        binder::Binder,
        player::Player,
    },
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
    target: usize,
    limit: Option<f32>,
    speed: f32,
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
        ReadStorage<'a, Binder>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, Tracker>,
        Read<'a, Time>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            binders,
            mut transforms,
            mut trackers,
            time,
        ) = data;

        for (tracker, transform, _) in (&mut trackers, &transforms, !&binders).join() {
            if tracker.rotation.is_none() {
                let rotation = transform.rotation();
                tracker.rotation.replace(rotation.clone());
            }
        }

        for (entity, tracker, _) in (&*entities, &trackers, !&binders).join() {
            let target = transforms.global_position(tracker.target);
            let joint = transforms.global_position(entity);
            let target = target - joint;

            let transform = transforms.local_transform(entity);
            let target = transform.transform_vector(&target);
            let up = transform.transform_vector(&Vector3::y());

            // The hack here is that the direction of joints is y axis, not z axis by default.
            let mut target = UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0)
                * UnitQuaternion::face_towards(&target, &up);

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
struct Config {
    max_angular_velocity: f32,
    max_duty_factor: f32,
    default_flight_time: f32,
    step_limit: [f32; 2],
    flight_factor: f32,
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
    duty_factor: f32,

    config: Config,
}

impl Limb {
    fn match_speed(&mut self, speed: f32) {
        let config = &self.config;

        let [min_step, max_step] = config.step_limit;

        let min_radius = min_step / config.max_duty_factor / TAU;
        self.angular_velocity = (speed / min_radius).min(config.max_angular_velocity);
        self.radius = if self.angular_velocity > 0.0 { speed / self.angular_velocity } else { min_radius };

        let step_length = (TAU * self.radius * config.max_duty_factor).min(max_step);
        self.duty_factor = step_length / (TAU * self.radius);
    }

    fn step_radius(&self) -> f32 {
        PI * self.radius * self.duty_factor
    }

    fn flight_time(&self) -> f32 {
        if self.angular_velocity > 0.0 {
            TAU * (1.0 - self.duty_factor) / self.angular_velocity
        } else {
            self.config.default_flight_time
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Quadruped {
    limbs: [Limb; 4],
}

impl Component for Quadruped {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct QuadrupedPrefab {
    feet: [usize; 4],
    anchors: [usize; 4],

    #[serde(flatten)]
    config: Config,
}

impl<'a> PrefabData<'a> for QuadrupedPrefab {
    type SystemData = WriteStorage<'a, Quadruped>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity]) -> Result<Self::Result, Error> {
        let vec = self.feet.iter()
            .zip(self.anchors.iter())
            .map(|(&foot, &anchor)| Limb {
                foot: entities[foot],
                anchor: entities[anchor],
                state: State::Stance,
                home: None,
                length: None,

                radius: 0.0,
                angular_velocity: 0.0,
                duty_factor: 0.0,

                config: self.config.clone(),
            })
            .collect_vec();
        let limbs = vec[..].try_into().unwrap();
        let component = Quadruped { limbs };

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
            for limb in quadruped.limbs.iter_mut() {
                if limb.home.is_none() {
                    let foot = transforms.global_position(limb.foot);
                    let home = transforms.local_transform(entity).transform_point(&foot);
                    limb.home.replace(home);
                }

                if limb.length.is_none() {
                    let foot = transforms.global_position(limb.foot);
                    let anchor = transforms.global_position(limb.anchor);
                    let length = (foot - anchor).norm();
                    limb.length.replace(length);
                }

                if let Some((home, _length)) = limb.home.zip(limb.length) {
                    let home = transforms.global_transform(entity).transform_point(&home);
                    let foot = transforms.global_position(limb.foot);
                    let anchor = transforms.global_position(limb.anchor);
                    let delta = &foot - &home;

                    let velocity = {
                        let root = transforms.global_position(entity);
                        let radial = &home - root;
                        let transform = transforms.global_transform(entity);
                        let angular = transform.transform_vector(&player.rotation().scaled_axis());
                        let linear = transform.transform_vector(&player.velocity());
                        &linear + angular.cross(&radial)
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
                    }

                    limb.state = match &limb.state {
                        State::Stance => {
                            if delta.norm() > step_radius {
                                let stance = foot;
                                State::Flight { stance, time: 0.0 }
                            } else {
                                State::Stance
                            }
                        }
                        State::Flight { stance, time } => {
                            let stance = stance.clone();
                            let time = *time;

                            let direction = velocity.try_normalize(EPSILON).unwrap_or(Vector3::zero());
                            let target = &home + velocity * (flight_time - time);
                            let next = target + direction * step_radius;
                            {
                                let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                                debug_lines.draw_line(home.clone(), next.clone(), color);
                            }

                            if time < flight_time {
                                let translation = next - &stance;
                                let current = &stance + Vector3::zero().lerp(&translation, time / flight_time);
                                let target = {
                                    let step_length = step_radius * 2.0;
                                    let coefficient = 4.0 * step_length * limb.config.flight_factor / flight_time;
                                    let lift = coefficient * time * (flight_time - time);
                                    let direction = (anchor - foot)
                                        .try_normalize(EPSILON)
                                        .unwrap_or(Vector3::zero());
                                    current + direction * lift
                                };
                                {
                                    let color = Srgba::new(1.0, 0.0, 1.0, 1.0);
                                    debug_lines.draw_line(current.clone(), target.clone(), color);
                                }

                                let rotation = transforms
                                    .get(entity)
                                    .unwrap()
                                    .rotation()
                                    .clone();
                                transforms
                                    .get_mut(limb.foot)
                                    .unwrap()
                                    .set_translation(target.coords)
                                    .set_rotation(rotation);

                                State::Flight { stance, time: delta_seconds + time }
                            } else {
                                State::Stance
                            }
                        }
                    }
                }
            }
        }
    }
}