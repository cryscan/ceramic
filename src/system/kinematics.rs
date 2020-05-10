use std::{
    borrow::Cow,
    marker::PhantomData,
    ops::Neg,
};

use amethyst::{
    assets::{PrefabData, ProgressCounter},
    core::{
        bundle::SystemBundle,
        math::{Matrix4, Point3, UnitQuaternion, Vector3},
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ChainPrefab {
    length: usize,
    target: usize,
}

impl<'a> PrefabData<'a> for ChainPrefab {
    type SystemData = WriteStorage<'a, Chain>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Chain {
            length: self.length,
            target: entities[self.target],
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Direction {
    axes: Option<[Vector3<f32>; 2]>,
    target: Entity,
}

impl Component for Direction {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DirectionPrefab {
    axes: Option<[Vector3<f32>; 2]>,
    target: usize,
}

impl<'a> PrefabData<'a> for DirectionPrefab {
    type SystemData = WriteStorage<'a, Direction>;
    type Result = ();

    fn add_to_entity(&self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Direction {
            axes: self.axes.clone(),
            target: entities[self.target],
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Hinge {
    axis: Option<Vector3<f32>>,
    limit: Option<[f32; 2]>,
}

impl Component for Hinge {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone)]
pub struct Pole {
    pub target: Entity,
}

impl Component for Pole {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PolePrefab {
    target: usize,
}

impl<'a> PrefabData<'a> for PolePrefab {
    type SystemData = WriteStorage<'a, Pole>;
    type Result = ();

    fn add_to_entity(
        &self, entity: Entity, data: &mut Self::SystemData, entities: &[Entity], _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Pole { target: entities[self.target] };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[serde(deny_unknown_fields)]
pub enum ConstrainPrefab {
    Chain(ChainPrefab),
    Direction(DirectionPrefab),
    Hinge(Hinge),
    Pole(PolePrefab),
}

#[derive(Debug, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Binder {
    pub name: Cow<'static, str>,
}

impl Component for Binder {
    type Storage = DenseVecStorage<Self>;
}

trait Helper {
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32>;
    fn local_transform(&self, entity: Entity) -> Matrix4<f32>;
}

impl Helper for WriteStorage<'_, Transform> {
    #[inline]
    fn global_transform(&self, entity: Entity) -> &Matrix4<f32> {
        self.get(entity).unwrap().global_matrix()
    }

    #[inline]
    fn local_transform(&self, entity: Entity) -> Matrix4<f32> {
        self.get(entity).unwrap().global_view_matrix()
    }
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
            binders,
            chains,
            mut directions,
            mut hinges,
            poles,
            mut debug_lines,
        ) = data;

        for (entity, chain, _) in (&*entities, &chains, !&binders).join() {
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

            let origin = &Point3::<f32>::origin();

            // Render debug lines.
            for (&start, &end) in entities.iter().tuple_windows() {
                let start = transforms.global_transform(start).transform_point(origin);
                let end = transforms.global_transform(end).transform_point(origin);
                let color = Srgba::new(0.0, 0.0, 0.0, 1.0);
                debug_lines.draw_line(start, end, color);
            }

            let mut end = origin.clone();
            let target = transforms.global_transform(chain.target).transform_point(origin);
            let mut target = transforms.local_transform(entity).transform_point(&target);

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

                // Auto-derive axes. Axes are local axes of target relating to parent.
                if let Some(direction) = directions.get_mut(parent) {
                    if direction.axes.is_none() {
                        let transform_vector = |vector| {
                            let global = transforms
                                .global_transform(direction.target)
                                .transform_vector(&vector);
                            transforms.local_transform(parent).transform_vector(&global)
                        };
                        let dir = transform_vector(Vector3::z());
                        let up = transform_vector(Vector3::y());
                        direction.axes.replace([dir, up]);
                    }
                }

                if let Some(direction) = directions.get(parent) {
                    if let Some([dir, up]) = &direction.axes {
                        let transform_vector = |vector| {
                            let global = transforms
                                .global_transform(direction.target)
                                .transform_vector(&vector);
                            transforms.local_transform(parent).transform_vector(&global)
                        };
                        let target_dir = transform_vector(Vector3::z());
                        let target_up = transform_vector(Vector3::y());

                        let rotation = UnitQuaternion::rotation_between(&dir, &target_dir)
                            .unwrap_or(UnitQuaternion::identity());
                        let target_up = rotation.inverse_transform_vector(&target_up);
                        let rotation = UnitQuaternion::rotation_between(&up, &target_up)
                            .unwrap_or(UnitQuaternion::identity())
                            * rotation;

                        if let Some((axis, angle)) = rotation.axis_angle() {
                            transforms
                                .get_mut(parent)
                                .unwrap()
                                .append_rotation(axis, angle);
                            target = UnitQuaternion::from_axis_angle(&axis, -angle)
                                .transform_point(&target);
                        }
                    }
                } else {
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
                }

                // Align the joint with pole.
                if let Some(pole) = poles.get(parent) {
                    let pole = transforms.global_transform(pole.target).transform_point(origin);
                    let pole = transforms.local_transform(parent).transform_point(&pole).coords;
                    let direction = transforms
                        .get(child)
                        .unwrap()
                        .translation();
                    let axis = end.coords.normalize();

                    // Draw debug line for pole.
                    {
                        let start = transforms.global_transform(child).transform_point(origin);
                        let end = &start + &transforms.global_transform(parent).transform_vector(&pole);
                        let color = Srgba::new(0.0, 1.0, 1.0, 1.0);
                        debug_lines.draw_line(start, end, color);
                    }

                    let pole = pole - axis.scale(pole.dot(&axis));
                    let direction = direction - axis.scale(direction.dot(&axis));

                    if let Some((axis, angle)) = UnitQuaternion::rotation_between(&direction, &pole)
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
                    if let Some(axis) = &hinge.axis {
                        // Draw debug line for hinge axis.
                        {
                            let transform = transforms.global_transform(parent);
                            let start = transform.transform_point(origin);
                            let end = &start + transform.transform_vector(axis);
                            let color = Srgba::new(1.0, 0.0, 0.0, 1.0);
                            debug_lines.draw_line(start, end, color);
                        }

                        let parent_axis = transforms
                            .get(parent)
                            .unwrap()
                            .rotation()
                            .inverse_transform_vector(axis);

                        if let Some((axis, angle)) = UnitQuaternion::rotation_between(axis, &parent_axis)
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
    }
}

#[derive(SystemDesc)]
pub struct BinderSystem<T: Component + Clone> {
    _marker: PhantomData<T>,
}

impl<T: Component + Clone> Default for BinderSystem<T> {
    fn default() -> Self {
        BinderSystem { _marker: PhantomData }
    }
}

impl<'a, T: Component + Clone> System<'a> for BinderSystem<T> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Binder>,
        ReadStorage<'a, Named>,
        WriteStorage<'a, T>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            binders,
            names,
            mut storage,
        ) = data;

        for (entity, binder) in (&*entities, &binders).join() {
            let component = storage.get(entity).cloned();
            for (entity, name) in (&*entities, &names).join() {
                if binder.name == name.name {
                    if let Some(component) = component {
                        storage.insert(entity, component).unwrap();
                    }
                    break;
                }
            }
            entities.delete(entity).unwrap();
        }
    }
}

#[derive(Default)]
pub struct BinderBundle;

impl BinderBundle {
    pub fn new() -> Self { BinderBundle }
}

impl<'a, 'b> SystemBundle<'a, 'b> for BinderBundle {
    fn build(self, _world: &mut World, builder: &mut DispatcherBuilder<'a, 'b>) -> Result<(), Error> {
        builder.add(BinderSystem::<Chain>::default(), "chain_binder", &[]);
        builder.add(BinderSystem::<Direction>::default(), "direction_binder", &[]);
        builder.add(BinderSystem::<Hinge>::default(), "hinge_binder", &[]);
        builder.add(BinderSystem::<Pole>::default(), "pole_binder", &[]);
        Ok(())
    }
}