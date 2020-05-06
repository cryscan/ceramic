use amethyst::{
    assets::PrefabData,
    core::{
        math::{Point3, UnitQuaternion, Vector3},
        Parent, Transform,
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

#[derive(Copy, Clone, Debug)]
pub struct Chain {
    pub length: usize,
    pub target: Entity,
}

impl Component for Chain {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl KinematicsSystem {
    fn update(
        &self,
        entity: Entity,
        chain: &Chain,
        entities: &Vec<Entity>,
        transforms: &mut WriteStorage<'_, Transform>,
    ) -> f32 {
        let end = Point3::<f32>::origin();
        let target = {
            let global = transforms.get(chain.target).unwrap()
                .global_matrix()
                .transform_point(&Point3::<f32>::origin());
            transforms.get(entity).unwrap()
                .global_view_matrix()
                .transform_point(&global)
        };

        let (end, target) = entities.iter()
            .fold(
                (end, target),
                |(end, target), entity| {
                    let transform = transforms.get_mut(*entity).unwrap();
                    let target = transform.matrix().transform_point(&target);
                    {
                        let end = transform.matrix().transform_point(&end);
                        if let Some((axis, angle)) =
                        UnitQuaternion::rotation_between(&end.coords, &target.coords)
                            .and_then(|rotation| rotation.axis_angle()) {
                            transform.prepend_rotation(axis, angle);
                        }
                    }
                    let end = transform.matrix().transform_point(&end);
                    (end, target)
                },
            );

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

            entities.iter().skip(1)
                .fold(entity, |prev, &entity| {
                    let start = transforms.get(entity).unwrap()
                        .global_matrix()
                        .transform_point(&Point3::origin());
                    let end = transforms.get(prev).unwrap()
                        .global_matrix()
                        .transform_point(&Point3::origin());
                    let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                    debug_lines.draw_line(start, end, color);
                    entity
                });

            self.update(entity, chain, &entities, &mut transforms);
        }
    }
}