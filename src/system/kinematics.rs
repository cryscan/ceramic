use std::borrow::Cow;

use amethyst::{
    assets::PrefabData,
    core::{
        math::{Point3, UnitQuaternion, Vector3},
        Named, Parent, Transform,
    },
    derive::SystemDesc,
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

#[derive(Debug, Clone)]
pub struct ChainBinder {
    pub length: usize,
    pub target: Entity,
    pub name: Cow<'static, str>,
}

impl Component for ChainBinder {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainPrefab {
    length: usize,
    target: usize,
    name: Option<Cow<'static, str>>,
}

impl<'a> PrefabData<'a> for ChainPrefab {
    type SystemData = (
        WriteStorage<'a, Chain>,
        WriteStorage<'a, ChainBinder>
    );
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _: &[Entity],
    ) -> Result<Self::Result, Error> {
        match &self.name {
            None => {
                let chain = Chain {
                    length: self.length,
                    target: entities[self.target],
                };
                data.0.insert(entity, chain).map(|_| ()).map_err(Into::into)
            }
            Some(name) => {
                let chain_binder = ChainBinder {
                    length: self.length,
                    target: entities[self.target],
                    name: name.clone(),
                };
                data.1.insert(entity, chain_binder).map(|_| ()).map_err(Into::into)
            }
        }
    }
}

pub fn bind_chains(data: (Entities, ReadStorage<Named>, ReadStorage<ChainBinder>, WriteStorage<Chain>)) {
    let (entities, names, binders, mut chains) = data;
    for binder in (&binders).join() {
        for (entity, name) in (&*entities, &names).join() {
            if name.name == binder.name {
                let chain = Chain {
                    length: binder.length,
                    target: binder.target,
                };
                chains.insert(entity, chain).map(|_| ()).unwrap_or(());
            }
        }
    }
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl KinematicsSystem {
    fn global_position(
        entity: Entity,
        transforms: &WriteStorage<Transform>,
        local: &Point3<f32>,
    ) -> Point3<f32> {
        transforms
            .get(entity)
            .unwrap()
            .global_matrix()
            .transform_point(local)
    }

    fn global_position_origin(
        entity: Entity,
        transforms: &WriteStorage<Transform>,
    ) -> Point3<f32> {
        Self::global_position(entity, transforms, &Point3::origin())
    }

    fn local_position(
        entity: Entity,
        transforms: &WriteStorage<Transform>,
        global: &Point3<f32>,
    ) -> Point3<f32> {
        transforms
            .get(entity)
            .unwrap()
            .global_view_matrix()
            .transform_point(global)
    }

    fn update(
        &self,
        entity: Entity,
        chain: &Chain,
        entities: &Vec<Entity>,
        transforms: &mut WriteStorage<Transform>,
    ) -> f32 {
        let mut end = Point3::<f32>::origin();
        let mut target = {
            let global = Self::global_position_origin(chain.target, transforms);
            Self::local_position(entity, transforms, &global)
        };

        for (first, second) in entities.iter().tuple_windows() {
            let matrix = transforms
                .get(*first)
                .unwrap()
                .matrix();
            end = matrix.transform_point(&end);
            target = matrix.transform_point(&target);

            if let Some((axis, angle)) =
            UnitQuaternion::rotation_between(&end.coords, &target.coords)
                .and_then(|rotation| rotation.axis_angle()) {
                transforms
                    .get_mut(*second)
                    .unwrap()
                    .append_rotation(axis, angle);
                target = UnitQuaternion::from_axis_angle(&axis, -angle)
                    .transform_point(&target);
            }
        }

        Vector3::from(end - target).norm_squared()
    }
}

impl<'a> System<'a> for KinematicsSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Chain>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Parent>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, (entities, chains, mut transforms, parents, mut debug_lines): Self::SystemData) {
        for (entity, chain) in (&*entities, &chains).join() {
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

            for (start, end) in entities.iter().tuple_windows() {
                let start = Self::global_position_origin(*start, &transforms);
                let end = Self::global_position_origin(*end, &transforms);
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            self.update(entity, chain, &entities, &mut transforms);
        }
    }
}