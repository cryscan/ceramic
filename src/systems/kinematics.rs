use std::ops::Neg;

use amethyst::{
    assets::{PrefabData, ProgressCounter},
    core::{
        math::{Point3, UnitQuaternion, Vector3},
        Parent,
        Transform,
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

use crate::utils::transform::Adaptor;

#[derive(Debug, Copy, Clone)]
pub struct Chain {
    target: Entity,
    length: usize,
}

impl Component for Chain {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ChainPrefab {
    pub target: usize,
    pub length: usize,
}

impl<'a> PrefabData<'a> for ChainPrefab {
    type SystemData = WriteStorage<'a, Chain>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity]) -> Result<Self::Result, Error> {
        let component = Chain {
            length: self.length,
            target: entities[self.target],
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Direction {
    target: Entity,
    rotation: Option<UnitQuaternion<f32>>,
}

impl Component for Direction {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DirectionPrefab {
    pub target: usize,
}

impl<'a> PrefabData<'a> for DirectionPrefab {
    type SystemData = WriteStorage<'a, Direction>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity]) -> Result<Self::Result, Error> {
        let component = Direction {
            target: entities[self.target],
            rotation: None,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Hinge {
    #[serde(skip_deserializing, skip_serializing)]
    axis: Option<Vector3<f32>>,
    limit: Option<[f32; 2]>,
}

impl Component for Hinge {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone)]
pub struct Pole {
    target: Entity,
}

impl Component for Pole {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PolePrefab {
    pub target: usize,
}

impl<'a> PrefabData<'a> for PolePrefab {
    type SystemData = WriteStorage<'a, Pole>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity]) -> Result<Self::Result, Error> {
        let component = Pole { target: entities[self.target] };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[serde(deny_unknown_fields)]
pub enum ConstrainPrefab {
    Direction(DirectionPrefab),
    Hinge(Hinge),
    Pole(PolePrefab),
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl<'a> System<'a> for KinematicsSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Parent>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Chain>,
        WriteStorage<'a, Direction>,
        WriteStorage<'a, Hinge>,
        ReadStorage<'a, Pole>,
        Write<'a, DebugLines>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            parents,
            mut transforms,
            chains,
            mut directions,
            mut hinges,
            poles,
            mut debug_lines,
        ) = data;

        for (entity, chain) in (&*entities, &chains).join() {
            let entities = iterate(
                entity,
                |entity| {
                    parents
                        .get(*entity)
                        .expect("Chain too long")
                        .entity
                })
                .take(chain.length)
                .collect_vec();

            // Render debug lines.
            for (&start, &end) in entities.iter().tuple_windows() {
                let start = transforms.global_position(start);
                let end = transforms.global_position(end);
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            let mut end = Point3::<f32>::origin();
            let ref target = transforms.global_position(chain.target);
            let mut target = transforms.local_transform(entity).transform_point(target);

            // Direction of entity is the rotation of its parent.
            for (&child, &parent) in entities.iter().tuple_windows() {
                // Bring end and target to entity's coordinate.
                {
                    let transform_point = |point| transforms
                        .get(child)
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

                // Align the joint with pole.
                if let Some(pole) = poles.get(parent) {
                    let ref pole = transforms.global_position(pole.target);
                    let ref pole = transforms
                        .local_transform(parent)
                        .transform_point(pole)
                        .coords;
                    let direction = transforms
                        .get(child)
                        .unwrap()
                        .translation();
                    let ref axis = end.coords.normalize();

                    // Draw debug line for pole.
                    {
                        let position = transforms.global_position(child);
                        let direction = transforms.global_transform(parent).transform_vector(pole);
                        let color = Srgba::new(0.0, 1.0, 1.0, 1.0);
                        debug_lines.draw_direction(position, direction, color);
                    }

                    let ref pole = pole - axis.scale(pole.dot(axis));
                    let ref direction = direction - axis.scale(direction.dot(axis));

                    if let Some((axis, angle)) = UnitQuaternion::rotation_between(direction, pole)
                        .and_then(|rotation| rotation.axis_angle()) {
                        transforms
                            .get_mut(parent)
                            .unwrap()
                            .append_rotation(axis, angle);
                        target = UnitQuaternion::from_axis_angle(&axis, -angle)
                            .transform_point(&target);
                    }
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
                    if let Some(ref axis) = hinge.axis {
                        // Draw debug line for hinge axis.
                        {
                            let position = transforms.global_position(parent);
                            let direction = transforms.global_transform(parent).transform_vector(axis);
                            let color = Srgba::new(1.0, 0.0, 0.0, 1.0);
                            debug_lines.draw_direction(position, direction, color);
                        }

                        let ref parent_axis = transforms
                            .get(parent)
                            .unwrap()
                            .rotation()
                            .inverse_transform_vector(axis);

                        if let Some((axis, angle)) = UnitQuaternion::rotation_between(axis, parent_axis)
                            .and_then(|rotation| rotation.axis_angle()) {
                            transforms
                                .get_mut(parent)
                                .unwrap()
                                .append_rotation(axis, angle);
                            target = UnitQuaternion::from_axis_angle(&axis, -angle)
                                .transform_point(&target);
                        }

                        // Apply hinge limit.
                        if let Some([min, max]) = hinge.limit {
                            let transform = transforms
                                .get_mut(parent)
                                .unwrap();
                            let hinge_axis = axis;
                            if let Some((axis, angle)) = transform
                                .rotation()
                                .axis_angle() {
                                let (axis, angle) = {
                                    if axis.dot(hinge_axis) < 0.0 { (axis.neg(), angle.neg()) } else { (axis, angle) }
                                };
                                let angle = angle.min(max).max(min) - angle;

                                transform.append_rotation(axis, angle);
                                target = UnitQuaternion::from_axis_angle(&axis, -angle)
                                    .transform_point(&target);
                            }
                        }
                    }
                }
            }
        }

        for (entity, direction, _) in (&*entities, &mut directions, &chains).join() {
            if direction.rotation.is_none() {
                let transform_vector = |ref vector| {
                    let ref global = transforms
                        .global_transform(direction.target)
                        .transform_vector(vector);
                    transforms.local_transform(entity).transform_vector(global)
                };
                let ref dir = transform_vector(Vector3::z());
                let ref up = transform_vector(Vector3::y());
                direction.rotation.replace(UnitQuaternion::face_towards(dir, up));
            }

            if let Some(ref rotation) = direction.rotation {
                let target_rotation = {
                    let transform_vector = |ref vector| {
                        let ref global = transforms
                            .global_transform(direction.target)
                            .transform_vector(vector);
                        transforms.local_transform(entity).transform_vector(global)
                    };
                    let ref dir = transform_vector(Vector3::z());
                    let ref up = transform_vector(Vector3::y());
                    UnitQuaternion::face_towards(dir, up)
                };

                let rotation = target_rotation * rotation.inverse();
                if let Some((axis, angle)) = rotation.axis_angle() {
                    transforms
                        .get_mut(entity)
                        .unwrap()
                        .append_rotation(axis, angle);
                }
            }
        }
    }
}