use std::f32::{consts::FRAC_PI_2, EPSILON};

use amethyst::{
    assets::PrefabData,
    core::{math::{Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    Error,
};
use serde::{Deserialize, Serialize};

use crate::{
    system::binder::Binder,
    utils::transform::Getter,
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
            let origin = &Point3::<f32>::origin();
            let target = transforms.global_transform(tracker.target).transform_point(origin);
            let joint = transforms.global_transform(entity).transform_point(origin);
            let target = target - joint;

            let transform = transforms.local_transform(entity);
            let target = transform.transform_vector(&target);
            let up = transform.transform_vector(&Vector3::y());
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
