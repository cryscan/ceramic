use amethyst::{
    core::{
        math::{Point3, try_convert, UnitQuaternion, Vector3}, Parent,
        Transform,
    },
    derive::SystemDesc,
    ecs::prelude::*,
};
use itertools::{iterate, Itertools};
use serde::{Deserialize, Serialize};

pub struct IKChain {
    pub length: usize,
    pub target: Entity,
}

impl Component for IKChain {
    type Storage = DenseVecStorage<Self>;
}

#[derive(SystemDesc)]
pub struct IKSolver;

impl<'a> System<'a> for IKSolver {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, IKChain>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, chains, mut transforms, parents): Self::SystemData) {
        for (entity, chain) in (&*entities, &chains).join() {
            iterate(
                entity,
                |entity| {
                    parents
                        .get(*entity)
                        .expect("IK chain too long")
                        .entity
                })
                .take(chain.length)
                .skip(1)
                .fold(
                    (
                        Point3::<f32>::origin(),
                        {
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
                        }
                    ),
                    |(end, target), entity| {
                        let target = transforms
                            .get(entity)
                            .unwrap()
                            .matrix()
                            .transform_point(&target);
                        {
                            let transform = transforms.get_mut(entity).unwrap();
                            let end = transform.matrix().transform_point(&end);

                            if let Some((axis, angle)) = UnitQuaternion::rotation_between(
                                &Vector3::from(end - Point3::origin()),
                                &Vector3::from(target - Point3::origin()),
                            )
                                .and_then(|rotation| rotation.axis_angle()) {
                                transform.append_rotation(axis, angle);
                            }
                        }
                        let end = transforms
                            .get(entity)
                            .unwrap()
                            .matrix()
                            .transform_point(&end);
                        (end, target)
                    },
                );
        }
    }
}