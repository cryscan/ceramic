use std::ops::Neg;

use amethyst::{
    assets::{PrefabData, ProgressCounter},
    core::{
        ArcThreadPool,
        bundle::SystemBundle,
        math::{Point3, UnitQuaternion, Vector3},
        transform::{Parent, Transform, TransformSystemDesc},
    },
    derive::{PrefabData, SystemDesc},
    ecs::{
        AccessorCow, BatchAccessor, BatchController, BatchUncheckedWorld, Component, Dispatcher, prelude::*, RunningTime},
    error::Error,
};
use amethyst::prelude::SystemDesc;
use getset::CopyGetters;
use itertools::{iterate, Itertools};
use serde::{Deserialize, Serialize};

use ceramic_derive::Redirect;
use redirect::Redirect;

use crate::{scene::RedirectField, utils::transform::TransformTrait};

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Chain {
    target: Entity,
    length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct ChainPrefab {
    pub target: RedirectField,
    #[redirect(skip)]
    pub length: usize,
}

impl<'a> PrefabData<'a> for ChainPrefab {
    type SystemData = WriteStorage<'a, Chain>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Chain {
            target: self.target.clone().into_entity(entities),
            length: self.length,
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

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Pole {
    target: Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct PolePrefab {
    pub target: RedirectField,
}

impl<'a> PrefabData<'a> for PolePrefab {
    type SystemData = WriteStorage<'a, Pole>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Pole { target: self.target.clone().into_entity(entities) };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Direction {
    target: Entity,
    rotation: Option<UnitQuaternion<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct DirectionPrefab {
    pub target: RedirectField,
}

impl<'a> PrefabData<'a> for DirectionPrefab {
    type SystemData = WriteStorage<'a, Direction>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Direction {
            target: self.target.clone().into_entity(entities),
            rotation: None,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Distance {
    target: Entity,
    distance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct DistancePrefab {
    pub target: RedirectField,
    #[redirect(skip)]
    #[serde(default)]
    pub distance: f32,
}

impl<'a> PrefabData<'a> for DistancePrefab {
    type SystemData = WriteStorage<'a, Distance>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Distance {
            target: self.target.clone().into_entity(entities),
            distance: self.distance,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PrefabData, Redirect)]
#[serde(deny_unknown_fields)]
pub enum ConstrainPrefab {
    #[redirect(skip)]
    Hinge(Hinge),
    Pole(PolePrefab),
    Direction(DirectionPrefab),
    Distance(DistancePrefab),
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSetupSystem;

impl KinematicsSetupSystem {
    pub fn setup_direction(
        entity: Entity,
        transforms: ReadStorage<'_, Transform>,
        direction: &mut Direction,
    ) -> Option<()> {
        if direction.rotation.is_none() {
            let transform_vector = |ref vector| {
                let ref global = transforms
                    .get(direction.target)?
                    .global_matrix()
                    .transform_vector(vector);
                transforms
                    .get(entity)
                    .map(|transform| transform.global_view_matrix().transform_vector(global))
            };
            let ref dir = transform_vector(Vector3::z())?;
            let ref up = transform_vector(Vector3::y())?;
            direction.rotation.replace(UnitQuaternion::face_towards(dir, up));
        }
        Some(())
    }
}

impl<'a> System<'a> for KinematicsSetupSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Transform>,
        WriteStorage<'a, Hinge>,
        WriteStorage<'a, Direction>,
    );

    fn run(&mut self, (entities, transforms, mut hinges, mut directions): Self::SystemData) {
        for (transform, hinge) in (&transforms, &mut hinges).join() {
            if hinge.axis.is_none() {
                hinge.axis = transform
                    .rotation()
                    .axis()
                    .map(|axis| axis.into_inner());
            }
        }

        for (entity, direction) in (&*entities, &mut directions).join() {
            Self::setup_direction(entity, transforms.clone(), direction);
        }
    }
}

#[derive(Default, SystemDesc)]
pub struct KinematicsSystem;

impl KinematicsSystem {
    fn collect_entities(
        parents: ReadStorage<'_, Parent>,
        entity: Entity,
        length: usize,
    ) -> Option<Vec<Entity>> {
        iterate(Some(entity), |entity| {
            match entity {
                None => None,
                Some(entity) => parents
                    .get(*entity)
                    .map(|parent| parent.entity),
            }
        })
            .take(length)
            .collect()
    }

    fn solve_inverse_kinematics(
        entities: Vec<Entity>,
        chain: &Chain,
        config: &Config,
        transforms: &mut WriteStorage<'_, Transform>,
        hinges: ReadStorage<'_, Hinge>,
        poles: ReadStorage<'_, Pole>,
    ) -> Option<()> {
        let mut end = Point3::<f32>::origin();
        let ref target = transforms.get(chain.target)?.global_position();
        let mut target = transforms
            .get(*entities.first()?)?
            .global_view_matrix()
            .transform_point(target);

        if target.coords.norm() < config.eps { return Some(()); }

        for (child, parent) in entities.into_iter().tuple_windows() {
            end = transforms.get(child)?.matrix().transform_point(&end);
            target = transforms.get(child)?.matrix().transform_point(&target);

            // Align the end with the target.
            if let Some((axis, angle)) = UnitQuaternion::rotation_between(&end.coords, &target.coords)
                .and_then(|rotation| rotation.axis_angle()) {
                transforms
                    .get_mut(parent)?
                    .append_rotation(axis, angle);
                target = UnitQuaternion::from_axis_angle(&axis, -angle)
                    .transform_point(&target);
            }

            // Align the joint with pole.
            if let Some(pole) = poles.get(parent) {
                let ref pole = transforms.get(pole.target)?.global_position();
                let ref pole = transforms
                    .get(parent)?
                    .global_view_matrix()
                    .transform_point(pole)
                    .coords;
                let direction = transforms
                    .get(child)?
                    .translation();
                let ref axis = end.coords.normalize();

                let ref pole = pole - axis.scale(pole.dot(axis));
                let ref direction = direction - axis.scale(direction.dot(axis));

                if let Some((axis, angle)) = UnitQuaternion::rotation_between(direction, pole)
                    .and_then(|rotation| rotation.axis_angle()) {
                    transforms
                        .get_mut(parent)?
                        .append_rotation(axis, angle);
                    target = UnitQuaternion::from_axis_angle(&axis, -angle)
                        .transform_point(&target);
                }
            }

            // Apply hinge constraint.
            if let Some(hinge) = hinges.get(parent) {
                if let Some(ref axis) = hinge.axis {
                    let ref parent_axis = transforms
                        .get(parent)?
                        .rotation()
                        .inverse_transform_vector(axis);

                    if let Some((axis, angle)) = UnitQuaternion::rotation_between(axis, parent_axis)
                        .and_then(|rotation| rotation.axis_angle()) {
                        transforms
                            .get_mut(parent)?
                            .append_rotation(axis, angle);
                        target = UnitQuaternion::from_axis_angle(&axis, -angle)
                            .transform_point(&target);
                    }

                    // Apply hinge limit.
                    if let Some([min, max]) = hinge.limit {
                        let transform = transforms
                            .get_mut(parent)?;
                        let hinge_axis = axis;
                        if let Some((axis, angle)) = transform
                            .rotation()
                            .axis_angle() {
                            let (axis, angle) = if axis.dot(hinge_axis) < 0.0 {
                                (axis.neg(), angle.neg())
                            } else {
                                (axis, angle)
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
        Some(())
    }

    fn solve_direction(
        entity: Entity,
        direction: &Direction,
        transforms: &mut WriteStorage<'_, Transform>,
    ) -> Option<()> {
        if let Some(ref rotation) = direction.rotation {
            let target_rotation = {
                let transform_vector = |ref vector| {
                    let ref global = transforms
                        .get(direction.target)?
                        .global_matrix()
                        .transform_vector(vector);
                    transforms
                        .get(entity)
                        .map(|transform| transform.global_view_matrix().transform_vector(global))
                };
                let ref dir = transform_vector(Vector3::z())?;
                let ref up = transform_vector(Vector3::y())?;
                UnitQuaternion::face_towards(dir, up)
            };

            let rotation = target_rotation * rotation.inverse();
            if let Some((axis, angle)) = rotation.axis_angle() {
                transforms
                    .get_mut(entity)?
                    .append_rotation(axis, angle);
            }
        }
        Some(())
    }
}

impl<'a> System<'a> for KinematicsSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Parent>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, Chain>,
        ReadStorage<'a, Hinge>,
        ReadStorage<'a, Pole>,
        ReadStorage<'a, Direction>,
        ReadExpect<'a, Config>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            parents,
            mut transforms,
            chains,
            hinges,
            poles,
            directions,
            config,
        ) = data;

        // Solve inverse kinematics constrains.
        for (entity, chain) in (&*entities, &chains).join() {
            Self::collect_entities(parents.clone(), entity, chain.length)
                .and_then(|entities| Self::solve_inverse_kinematics(
                    entities,
                    chain,
                    &config,
                    &mut transforms,
                    hinges.clone(),
                    poles.clone(),
                ));
        }

        // Solve direction constrains.
        for (entity, direction) in (&*entities, &directions).join() {
            Self::solve_direction(entity, direction, &mut transforms);
        }
    }
}

#[derive(Debug, Copy, Clone, CopyGetters)]
#[get_copy = "pub"]
pub struct Config {
    iter: usize,
    eps: f32,
}

pub struct KinematicsBatchSystem<'a, 'b> {
    accessor: BatchAccessor,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> BatchController<'a, 'b> for KinematicsBatchSystem<'a, 'b> {
    type BatchSystemData = ReadExpect<'a, Config>;

    unsafe fn create(accessor: BatchAccessor, dispatcher: Dispatcher<'a, 'b>) -> Self {
        KinematicsBatchSystem {
            accessor,
            dispatcher,
        }
    }
}

impl<'a> System<'a> for KinematicsBatchSystem<'_, '_> {
    type SystemData = BatchUncheckedWorld<'a>;

    fn run(&mut self, data: Self::SystemData) {
        let config = data.0.fetch::<Config>();

        for _ in 0..config.iter {
            self.dispatcher.dispatch(data.0);
        }
    }

    fn running_time(&self) -> RunningTime {
        RunningTime::VeryLong
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Ref(&self.accessor)
    }

    fn setup(&mut self, world: &mut World) {
        self.dispatcher.setup(world);
    }
}

unsafe impl Send for KinematicsBatchSystem<'_, '_> {}

pub struct KinematicsBundle {
    iter: usize,
    eps: f32,
}

impl KinematicsBundle {
    pub fn new(iter: usize, eps: f32) -> Self {
        KinematicsBundle { iter, eps }
    }
}

impl SystemBundle<'static, 'static> for KinematicsBundle {
    fn build(
        self,
        world: &mut World,
        builder: &mut DispatcherBuilder<'static, 'static>,
    ) -> Result<(), Error> {
        world.insert(Config { iter: self.iter, eps: self.eps });

        let kinematics_builder = DispatcherBuilder::new()
            .with(TransformSystemDesc::default().build(world), "transform", &[])
            .with(KinematicsSetupSystem::default(), "setup", &["transform"])
            .with(KinematicsSystem, "kinematics", &["transform", "setup"])
            .with_pool((*world.fetch::<ArcThreadPool>()).clone());

        builder.add_batch::<KinematicsBatchSystem<'static, 'static>>(
            kinematics_builder,
            "kinematics_batch",
            &[],
        );

        Ok(())
    }
}