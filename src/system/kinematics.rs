use std::{borrow::Cow,
          ops::Neg,
};

use amethyst::{
    assets::PrefabData,
    core::{
        math::{Point3, Unit, UnitQuaternion, Vector3},
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Hinge {
    axis: Option<Vector3<f32>>,
    limit: Option<(f32, f32)>,
}

impl Component for Hinge {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct BallJoint {
    axis: Option<Vector3<f32>>,
    pole: Vector3<f32>,
    axis_limit: (f32, f32),
    pole_limit: (f32, f32),
}

impl Component for BallJoint {
    type Storage = DenseVecStorage<Self>;
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
        WriteStorage<'a, Hinge>,
        WriteStorage<'a, BallJoint>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            parents,
            mut transforms,
            binders,
            chains,
            mut hinges,
            mut ball_joints,
            mut debug_lines,
        ) = data;

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
                .transform_point(&global);

            let fix_axis_angle = |direction, axis: Unit<Vector3<f32>>, angle: f32| {
                if axis.dot(&direction) < 0.0 {
                    (axis.neg(), angle.neg())
                } else { (axis, angle) }
            };

            let offset = Vector3::new(2.0, 0.0, 0.0);

            // Render debug lines.
            for (start, end) in entities.iter().tuple_windows() {
                let start = global_position(*start) + offset;
                let end = global_position(*end) + offset;
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            let mut end = Point3::<f32>::origin();
            let mut target = local_position(entity, global_position(chain.target));

            // Direction of entity is the rotation of its parent.
            for (&entity, &parent) in entities.iter().tuple_windows() {
                // Bring end and target to entity's coordinate.
                {
                    let transform_point = |point| transforms
                        .get(entity)
                        .unwrap()
                        .matrix()
                        .transform_point(&point);
                    end = transform_point(end);
                    target = transform_point(target);
                }

                // Align the end with the target.
                if let Some((axis, angle)) = UnitQuaternion::rotation_between(&end.coords, &target.coords)
                    .and_then(|rotation| rotation.axis_angle()) {
                    transforms
                        .get_mut(parent)
                        .unwrap()
                        .append_rotation(axis, angle);
                    target = UnitQuaternion::from_axis_angle(&axis, -angle)
                        .transform_point(&target);
                }

                // Auto-derive hinge axis.
                if let Some(hinge) = hinges.get_mut(parent) {
                    if hinge.axis.is_none() {
                        hinge.axis = transforms
                            .get(parent)
                            .unwrap()
                            .rotation()
                            .axis()
                            .map(|axis| axis.into_inner());
                    }
                }

                // Apply hinge constraint.
                if let Some(hinge) = hinges.get(parent) {
                    if let Some(axis) = hinge.axis.clone() {
                        // Draw debug line for hinge axis.
                        {
                            let start = transforms
                                .get(parent)
                                .unwrap()
                                .global_matrix()
                                .transform_point(&Point3::origin()) + offset;
                            let axis = transforms
                                .get(parent)
                                .unwrap()
                                .global_matrix()
                                .transform_vector(&axis);
                            let end = start + axis;
                            let color = Srgba::new(1.0, 0.0, 0.0, 1.0);
                            debug_lines.draw_line(start, end, color);
                        }

                        let parent_axis = transforms
                            .get(parent)
                            .unwrap()
                            .rotation()
                            .inverse_transform_vector(&axis);

                        if let Some((axis, angle)) = UnitQuaternion::rotation_between(&axis, &parent_axis)
                            .and_then(|rotation| rotation.axis_angle()) {
                            transforms
                                .get_mut(parent)
                                .unwrap()
                                .append_rotation(axis, angle);
                            target = UnitQuaternion::from_axis_angle(&axis, -angle)
                                .transform_point(&target);
                        }

                        // Apply hinge limit.
                        if let Some((min, max)) = hinge.limit {
                            let transform = transforms
                                .get_mut(parent)
                                .unwrap();
                            let hinge_axis = axis;
                            if let Some((axis, angle)) = transform
                                .rotation()
                                .axis_angle() {
                                let (axis, angle) = fix_axis_angle(hinge_axis, axis, angle);
                                let angle = angle.min(max).max(min) - angle;

                                transform.append_rotation(axis, angle);
                                target = UnitQuaternion::from_axis_angle(&axis, -angle)
                                    .transform_point(&target);
                            }
                        }
                    }
                }

                // Auto-deduce ball joint axis.
                if let Some(ball_joint) = ball_joints.get_mut(parent) {
                    if ball_joint.axis.is_none() {
                        ball_joint.axis.replace(transforms
                            .get(entity)
                            .unwrap()
                            .translation()
                            .clone_owned()
                        );
                    }
                }

                // Apply ball joint constrain.
                if let Some((axis, pole, axis_limit, pole_limit)) = ball_joints.get(parent)
                    .and_then(|ball_joint| ball_joint.axis
                        .map(|axis| (
                            axis,
                            ball_joint.pole.clone_owned(),
                            ball_joint.axis_limit,
                            ball_joint.pole_limit)
                        )
                    ) {
                    let parent_rotation = transforms.get(parent).unwrap().rotation();

                    // Constrain the rotation to some angle around an axis.
                    let enforce_constrain = |axis, (min, max)| {
                        let parent_axis = parent_rotation.inverse_transform_vector(&axis);
                        UnitQuaternion::rotation_between(&axis, &parent_axis)
                            .and_then(|rotation| (rotation * parent_rotation).axis_angle())
                            .map(|(real_axis, angle)| {
                                let (axis, angle) = fix_axis_angle(axis, real_axis, angle);
                                let angle = angle.min(max).max(min) - angle;
                                UnitQuaternion::from_axis_angle(&axis, angle)
                            })
                    };
                    let left = pole.cross(&axis);

                    let axis_constrain = enforce_constrain(left, axis_limit);
                    let pole_constrain = enforce_constrain(pole, pole_limit);

                    let mut rotation = UnitQuaternion::identity();
                    if let Some(constrain) = axis_constrain { rotation = constrain * rotation; }
                    if let Some(constrain) = pole_constrain { rotation = constrain * rotation; }
                    if let Some((axis, angle)) = rotation.axis_angle() {
                        transforms
                            .get_mut(parent)
                            .unwrap()
                            .append_rotation(axis, angle);
                        target = rotation.inverse_transform_point(&target);
                    }
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
        WriteStorage<'a, Hinge>,
        WriteStorage<'a, BallJoint>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            binders,
            names,
            mut chains,
            mut hinges,
            mut ball_joints
        ) = data;

        for (entity, binder) in (&*entities, &binders).join() {
            let chain = chains.get(entity).cloned();
            let hinge = hinges.get(entity).cloned();
            let ball_joint = ball_joints.get(entity).cloned();
            for (entity, name) in (&*entities, &names).join() {
                if binder.name == name.name {
                    if let Some(chain) = chain { chains.insert(entity, chain).unwrap(); }
                    if let Some(hinge) = hinge { hinges.insert(entity, hinge).unwrap(); }
                    if let Some(ball_joint) = ball_joint { ball_joints.insert(entity, ball_joint).unwrap(); }
                }
            }
            entities.delete(entity).unwrap();
        }
    }
}