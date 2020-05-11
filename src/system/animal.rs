use std::f32::{consts::FRAC_PI_2, EPSILON};

use amethyst::{
    assets::PrefabData,
    core::{math::{Point3, UnitQuaternion, Vector3}, Time, Transform},
    derive::SystemDesc,
    ecs::prelude::*,
    Error,
    renderer::{
        debug_drawing::DebugLines,
        palette::Srgba,
    },
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    system::binder::Binder,
    utils::transform::Getter,
};

#[derive(Debug, Copy, Clone)]
pub struct Tracker {
    target: Entity,
    limit: Option<[f32; 3]>,
    speed: f32,
    rotation: Option<UnitQuaternion<f32>>,
}

impl Component for Tracker {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TrackerPrefab {
    target: usize,
    limit: Option<[f32; 3]>,
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
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            binders,
            mut transforms,
            mut trackers,
            time,
            mut debug_lines
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
            {
                let color = Srgba::new(1.0, 1.0, 1.0, 1.0);
                debug_lines.draw_line(target.clone(), joint.clone(), color);
            }
            let target = target - joint;

            let transform = transforms.local_transform(entity);
            let target = transform.transform_vector(&target);
            let up = transform.transform_vector(&Vector3::y());

            let target = {
                let correction = UnitQuaternion::from_euler_angles(FRAC_PI_2, 0.0, 0.0);
                let face = UnitQuaternion::face_towards(&target, &up);
                let rotation = tracker.rotation.unwrap_or_else(UnitQuaternion::identity);
                correction * face * rotation
            };
            let transform = transforms.get_mut(entity).unwrap();
            let current = transform.rotation();

            let interpolation = 1.0 - (-tracker.speed * time.delta_seconds()).exp();
            if let Some(rotation) = current.try_slerp(&target, interpolation, EPSILON) {
                if let Some(limit) = tracker.limit {
                    let (x, y, z) = rotation.euler_angles();
                    let (x, y, z) = [x, y, z].iter()
                        .zip(limit.iter())
                        .map(|(&angle, &limit)| angle.min(limit).max(-limit))
                        .collect_tuple()
                        .unwrap();
                    transform.set_rotation_euler(x, y, z);
                } else {
                    transform.set_rotation(rotation);
                }
            }
        }
    }
}
