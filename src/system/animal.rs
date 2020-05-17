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

/*

#[derive(CopyGetters, Debug, Default, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
#[get_copy = "pub"]
pub struct Motor {
    #[serde(skip)]
    radius: f32,
    #[serde(skip)]
    angular_velocity: f32,
    #[serde(skip)]
    duty_factor: f32,

    pub max_angular_velocity: f32,
    pub max_duty_factor: f32,
    pub step_limit: [f32; 2],
}

impl Motor {
    fn is_stance(&self, angle: f32) -> bool {
        angle.cos() < (PI * self.duty_factor).cos()
    }

    fn step_length(&self) -> f32 {
        TAU * self.radius * self.duty_factor
    }

    /// Determine the parameters given a speed.
    /// Increase angular velocity first, then radius; duty factor decreases as speed goes up.
    fn with_speed(&mut self, speed: f32) -> &Self {
        let min_radius = self.step_limit[0] / self.max_duty_factor;
        self.radius = min_radius;

        let angular_velocity = speed / min_radius;
        self.angular_velocity = angular_velocity.min(self.max_angular_velocity);

        if angular_velocity > self.angular_velocity {
            self.radius = speed / self.angular_velocity;
        }

        let step_length = (TAU * self.radius * self.max_duty_factor)
            .min(self.step_limit[1]);
        self.duty_factor = step_length / (TAU * self.radius);

        self
    }
}

impl Component for Motor {
    type Storage = DenseVecStorage<Self>;
}

 */

#[derive(Debug, Copy, Clone)]
enum State {
    Stance,
    Flight { stance: Point3<f32>, time: f32 },
}

#[derive(Debug, Copy, Clone)]
pub struct Limb {
    foot: Entity,
    anchor: Entity,
    state: State,

    home: Option<Point3<f32>>,
    length: Option<f32>,
}

#[derive(Debug, Copy, Clone)]
pub struct Quadruped {
    limbs: [Limb; 4],

    radius: f32,
    angular_velocity: f32,
    duty_factor: f32,

    max_angular_velocity: f32,
    max_duty_factor: f32,
    default_flight_time: f32,
    step_limit: [f32; 2],
}

impl Quadruped {
    fn match_speed(&mut self, speed: f32) {
        let [min_step, max_step] = self.step_limit;

        let min_radius = min_step / self.max_duty_factor / TAU;
        self.angular_velocity = (speed / min_radius).min(self.max_angular_velocity);
        self.radius = if self.angular_velocity > 0.0 { speed / self.angular_velocity } else { min_radius };

        let step_length = (TAU * self.radius * self.max_duty_factor).min(max_step);
        self.duty_factor = step_length / (TAU * self.radius);
    }

    fn step_radius(&self) -> f32 {
        PI * self.radius * self.duty_factor
    }

    fn flight_time(&self) -> f32 {
        if self.angular_velocity > 0.0 {
            TAU * (1.0 - self.duty_factor) / self.angular_velocity
        } else {
            self.default_flight_time
        }
    }
}

impl Component for Quadruped {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct QuadrupedPrefab {
    feet: [usize; 4],
    anchors: [usize; 4],

    max_angular_velocity: f32,
    max_duty_factor: f32,
    default_flight_time: f32,
    step_limit: [f32; 2],
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
            })
            .collect_vec();
        let limbs = vec[..].try_into().unwrap();
        let component = Quadruped {
            limbs,

            radius: 0.0,
            angular_velocity: 0.0,
            duty_factor: 0.0,

            max_angular_velocity: self.max_angular_velocity,
            max_duty_factor: self.max_duty_factor,
            default_flight_time: self.default_flight_time,
            step_limit: self.step_limit,
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

        for (entity, quadruped, player) in (&*entities, &mut quadrupeds, &players).join() {
            let velocity = player.movement().scale(player.speed());
            let speed = velocity.norm();
            quadruped.match_speed(speed);

            let step_radius = quadruped.step_radius();
            let flight_time = quadruped.flight_time();
            let delta_seconds = time.delta_seconds();

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
                    let radial = &foot - &home;
                    {
                        let color = Srgba::new(0.0, 1.0, 0.0, quadruped.duty_factor);
                        debug_lines.draw_rotated_circle(
                            home.clone(),
                            step_radius,
                            10,
                            UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0),
                            color,
                        );

                        let color = Srgba::new(1.0, 1.0, 0.0, 1.0);
                        debug_lines.draw_direction(home.clone(), radial.clone(), color);
                    }

                    limb.state = match &limb.state {
                        State::Stance => {
                            if radial.norm() > step_radius {
                                let stance = foot;
                                State::Flight { stance, time: 0.0 }
                            } else {
                                State::Stance
                            }
                        }
                        State::Flight { stance, time } => {
                            let stance = stance.clone();
                            let time = *time;

                            let direction = transforms
                                .global_transform(entity)
                                .transform_vector(player.movement());
                            let next = home + direction * step_radius;
                            {
                                let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                                debug_lines.draw_line(home.clone(), next.clone(), color);
                            }

                            if time < flight_time {
                                let translation = next - &stance;
                                let current = Vector3::zero().lerp(&translation, time / flight_time);
                                let current = &stance + current;

                                transforms
                                    .get_mut(limb.foot)
                                    .unwrap()
                                    .set_translation(current.coords);

                                State::Flight { stance, time: delta_seconds + time }
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
}