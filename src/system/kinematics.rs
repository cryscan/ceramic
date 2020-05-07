use std::borrow::Cow;

use amethyst::{
    assets::PrefabData,
    core::{
        math::{Point3, UnitQuaternion},
        Named, Parent, Transform,
    },
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    Error,
    renderer::{
        debug_drawing::DebugLines,
        palette::Srgba,
    },
};
use itertools::{iterate, Itertools};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub struct Chain {
    pub length: usize,
    pub target: Entity,
}

impl Component for Chain {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainPrefab {
    length: usize,
    target: usize,
}

impl<'a> PrefabData<'a> for ChainPrefab {
    type SystemData = WriteStorage<'a, Chain>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let chain = Chain {
            length: self.length,
            target: entities[self.target],
        };
        data.insert(entity, chain).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Binder {
    pub name: Cow<'static, str>,
}

impl Component for Binder {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl<'a> System<'a> for KinematicsSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Parent>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Binder>,
        ReadStorage<'a, Chain>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, (entities, parents, mut transforms, binders, chains, mut debug_lines): Self::SystemData) {
        for (entity, chain, _) in (&*entities, &chains, !&binders).join() {
            let entities = iterate(
                entity,
                |entity| {
                    parents
                        .get(*entity)
                        .expect("IK chain too long")
                        .entity
                })
                .take(chain.length)
                .collect_vec();

            let global_position = |entity| transforms
                .get(entity)
                .unwrap()
                .global_matrix()
                .transform_point(&Point3::<f32>::origin());

            let local_position = |entity, global| transforms
                .get(entity)
                .unwrap()
                .global_view_matrix()
                .transform_point(global);

            for (start, end) in entities.iter().tuple_windows() {
                let start = global_position(*start);
                let end = global_position(*end);
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            let mut end = Point3::<f32>::origin();
            let mut target = local_position(entity, &global_position(chain.target));

            for (entity, parent) in entities.iter().tuple_windows() {
                let transform = |point| transforms
                    .get(*entity)
                    .unwrap()
                    .matrix()
                    .transform_point(&point);
                end = transform(end);
                target = transform(target);

                if let Some((axis, angle)) = UnitQuaternion::rotation_between(&end.coords, &target.coords)
                    .and_then(|rotation| rotation.axis_angle()) {
                    transforms
                        .get_mut(*parent)
                        .unwrap()
                        .append_rotation(axis, angle);
                    target = UnitQuaternion::from_axis_angle(&axis, -angle)
                        .transform_point(&target);
                }
            }
        }
    }
}

#[derive(Default, SystemDesc)]
pub struct BinderSystem;

impl<'a> System<'a> for BinderSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Binder>,
        ReadStorage<'a, Named>,
        WriteStorage<'a, Chain>,
    );

    fn run(&mut self, (entities, binders, names, mut chains): Self::SystemData) {
        for (entity, binder) in (&*entities, &binders).join() {
            let chain = chains.get(entity).cloned();
            for (entity, name) in (&*entities, &names).join() {
                if binder.name == name.name {
                    if let Some(chain) = chain { chains.insert(entity, chain).unwrap(); }
                }
            }
            entities.delete(entity).unwrap();
        }
    }
}