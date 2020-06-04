use std::f32::{consts::FRAC_PI_2, EPSILON};

use amethyst::{
    core::{math::{UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
};

use crate::utils::transform::TransformTrait;

use super::Tracker;

#[derive(Default, SystemDesc)]
pub struct TrackSystem;

impl TrackSystem {
    fn process_tracker(
        entity: Entity,
        tracker: &Tracker,
        delta_seconds: f32,
        transforms: &mut WriteStorage<'_, Transform>,
    ) -> Option<()> {
        let target = transforms.get(tracker.target)?.global_position();
        let joint = transforms.get(entity)?.global_position();
        let ref target = target - joint;

        let transform = transforms.get(entity)?.global_view_matrix();
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

        let current = transforms.get(entity)?.rotation();
        let interpolation = 1.0 - (-tracker.speed * delta_seconds).exp();
        if let Some(rotation) = current.try_slerp(&target, interpolation, EPSILON) {
            transforms.get_mut(entity)?.set_rotation(rotation);
        }

        Some(())
    }
}

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
            Self::process_tracker(entity, tracker, time.delta_seconds(), &mut transforms);
        }
    }
}