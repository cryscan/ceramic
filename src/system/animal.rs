use std::{
    convert::TryInto,
    f32::{consts::{FRAC_PI_2, PI, TAU}, EPSILON},
};

use amethyst::{
    assets::PrefabData,
    core::{math::{Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    Error,
    renderer::{debug_drawing::DebugLines, palette::Srgba},
};
use getset::CopyGetters;
use itertools::Itertools;
use num_traits::identities::Zero;
use num_traits::Signed;
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
    pub step_length_limit: [f32; 2],
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
        let min_radius = self.step_length_limit[0] / self.max_duty_factor;
        self.radius = min_radius;

        let angular_velocity = speed / min_radius;
        self.angular_velocity = angular_velocity.min(self.max_angular_velocity);

        if angular_velocity > self.angular_velocity {
            self.radius = speed / self.angular_velocity;
        }

        let step_length = (TAU * self.radius * self.max_duty_factor)
            .min(self.step_length_limit[1]);
        self.duty_factor = step_length / (TAU * self.radius);

        self
    }
}

impl Component for Motor {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone)]
pub struct Limb {
    pub foot: Entity,
    pub anchor: Entity,
    pub angle: f32,
    pub stance_position: Point3<f32>,
    pub home: Option<Point3<f32>>,
    pub length: Option<f32>,
}

#[derive(Debug, Copy, Clone)]
pub struct Quadruped {
    pub limbs: [Limb; 4],
}

impl Component for Quadruped {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct QuadrupedPrefab {
    pub feet: [usize; 4],
    pub anchors: [usize; 4],
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
                angle: PI,
                stance_position: Point3::origin(),
                home: None,
                length: None,
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
        WriteStorage<'a, Motor>,
        WriteStorage<'a, Quadruped>,
        ReadStorage<'a, Player>,
        Read<'a, Time>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut transforms,
            mut motors,
            mut quadrupeds,
            players,
            time,
            mut debug_lines,
        ) = data;

        for (entity, motor, quadruped, player) in (&*entities, &mut motors, &mut quadrupeds, &players).join() {
            let velocity = player.movement() * player.speed();
            motor.with_speed(velocity.norm());

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
                    {
                        let color = Srgba::new(0.0, 0.0, 1.0, 1.0);
                        debug_lines.draw_rotated_circle(
                            limb.stance_position.clone(),
                            0.4,
                            10,
                            UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0),
                            color,
                        );
                    }

                    if motor.is_stance(limb.angle) {
                        limb.stance_position = transforms.global_position(limb.foot);
                    } else {
                        let step = motor.step_length();
                        let transform = transforms.global_transform(entity);
                        let home = transform.transform_point(&home);
                        let movement = transform.transform_vector(&player.movement());
                        let next = &home + step * movement;
                        let delta = next - &limb.stance_position;

                        {
                            let start_color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                            let end_color = Srgba::new(0.0, 0.0, 1.0, 1.0);
                            debug_lines.draw_gradient_line(limb.stance_position.clone(), next.clone(), start_color, end_color);
                        }

                        let angle = {
                            let angle = limb.angle.cos().acos();
                            if limb.angle.sin().is_positive() { angle } else { -angle }
                        };
                        let factor = {
                            let bound = motor.duty_factor.acos();
                            (angle + bound) / (2.0 * bound)
                        };

                        let translation = &limb.stance_position + Vector3::zero().lerp(&delta, factor);
                        transforms
                            .get_mut(limb.foot)
                            .unwrap()
                            .set_translation(translation.coords);
                    }

                    limb.angle += motor.angular_velocity * time.delta_seconds();
                }
            }
        }
    }
}