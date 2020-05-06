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
        let mut end: Point3<f32> = Point3::origin();
        let mut target: Point3<f32> = {
            let target = transforms.get(chain.target).unwrap()
                .global_matrix()
                .transform_point(&Point3::origin());
            transforms.get(entity).unwrap()
                .global_view_matrix()
                .transform_point(&target)
        };

        for (&first, &second) in entities.iter().tuple_windows() {
            {
                let transform = transforms.get(first).unwrap();
                end = transform.matrix().transform_point(&end);
                target = transform.matrix().transform_point(&target);
            }
            if let Some((axis, angle)) =
            UnitQuaternion::rotation_between(&end.coords, &target.coords)
                .and_then(|rotation| rotation.axis_angle()) {
                let transform = transforms.get_mut(second).unwrap();
                transform.append_rotation(axis, angle);
                target = UnitQuaternion::from_axis_angle(&axis, -angle).transform_point(&target);
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

            for (&start, &end) in entities.iter().tuple_windows() {
                let start = transforms.get(start).unwrap()
                    .global_matrix()
                    .transform_point(&Point3::origin());
                let end = transforms.get(end).unwrap()
                    .global_matrix()
                    .transform_point(&Point3::origin());
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            self.update(entity, chain, &entities.iter().map(Clone::clone).collect_vec(), &mut transforms);
        }
    }
}