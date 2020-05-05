use amethyst::{
    core::{
        math::{Matrix4, Point3, UnitQuaternion, Vector3},
        Parent, Transform,
    },
    derive::SystemDesc,
    ecs::prelude::*,
};
use itertools::{iterate, Itertools};

#[derive(Copy, Clone)]
pub struct Chain {
    pub length: usize,
    pub target: Entity,
}

impl Component for Chain {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl<'a> System<'a> for KinematicsSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Chain>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, chains, mut transforms, parents): Self::SystemData) {
        for (entity, chain) in (&*entities, &chains).join() {
            let effector = Point3::<f32>::origin();
            let target = {
                let global = transforms
                    .get(chain.target)
                    .unwrap()
                    .global_matrix()
                    .transform_point(&Point3::<f32>::origin());
                transforms
                    .get(entity)
                    .unwrap()
                    .global_view_matrix()
                    .transform_point(&global)
            };

            let entities = iterate(
                entity,
                |entity| {
                    parents
                        .get(*entity)
                        .expect("IK chain too long")
                        .entity
                })
                .take(chain.length)
                .skip(1)
                .collect_vec();

            entities
                .iter()
                .fold(
                    (
                        Matrix4::<f32>::identity(),
                        Matrix4::<f32>::identity()
                    ),
                    |(effector_matrix, target_matrix), entity| {
                        let transform = transforms.get_mut(*entity).unwrap();

                        let target_matrix = target_matrix * transform.matrix();
                        let target = target_matrix.transform_point(&target);

                        let effector = (effector_matrix * transform.matrix())
                            .transform_point(&effector);

                        if let Some((axis, angle)) =
                        UnitQuaternion::rotation_between(
                            &Vector3::from(effector - Point3::origin()),
                            &Vector3::from(target - Point3::origin()),
                        )
                            .and_then(|rotation| rotation.axis_angle()) {
                            transform.append_rotation(axis, angle);
                        }

                        let effector_matrix = effector_matrix * transform.matrix();
                        (effector_matrix, target_matrix)
                    },
                );
        }
    }
}