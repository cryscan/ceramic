use std::f32::{consts::FRAC_PI_2, EPSILON};

use amethyst::{
    assets::PrefabData,
    core::{math::{UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::{Component, prelude::*},
    error::Error,
};
use serde::{Deserialize, Serialize};

use ceramic_derive::Redirect;
use redirect::Redirect;

use crate::{
    scene::RedirectField,
    utils::transform::TransformTrait,
};

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Tracker {
    target: Entity,
    limit: Option<f32>,
    speed: f32,
    rotation: Option<UnitQuaternion<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct TrackerPrefab {
    pub target: RedirectField,
    #[redirect(skip)]
    pub limit: Option<f32>,
    #[redirect(skip)]
    pub speed: f32,
}

impl<'a> PrefabData<'a> for TrackerPrefab {
    type SystemData = WriteStorage<'a, Tracker>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Tracker {
            target: self.target.clone().into_entity(entities),
            limit: self.limit.clone(),
            speed: self.speed,
            rotation: None,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

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