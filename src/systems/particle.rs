use std::convert::identity;

use amethyst::{
    assets::PrefabData,
    core::{math::Point3, Parent, Transform},
    derive::SystemDesc,
    ecs::{Component, prelude::*},
    error::Error,
};
use amethyst_physics::prelude::*;
use serde::{Deserialize, Serialize};

use ceramic_derive::Redirect;
use redirect::Redirect;

use crate::{
    scene::RedirectField,
    utils::{match_shape, transform::TransformTrait},
};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParticlePrefab {
    pub mass: f32,
}

impl<'a> PrefabData<'a> for ParticlePrefab {
    type SystemData = (
        ReadExpect<'a, PhysicsWorld<f32>>,
        WriteStorage<'a, PhysicsHandle<PhysicsRigidBodyTag>>,
    );
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        (physics_world, bodies): &mut Self::SystemData,
        _: &[Entity],
        _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let body = {
            let ref desc = RigidBodyDesc {
                mode: BodyMode::Dynamic,
                mass: self.mass,
                ..Default::default()
            };
            physics_world.rigid_body_server().create(desc)
        };
        bodies.insert(entity, body)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Deform {
    targets: Vec<Entity>,
    vertices: Vec<Entity>,
    stiffness: f32,
    damp: f32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Redirect)]
#[serde(default)]
pub struct DeformPrefab {
    pub targets: Vec<RedirectField>,
    pub vertices: Vec<RedirectField>,
    #[redirect(skip)]
    pub stiffness: f32,
    #[redirect(skip)]
    pub damp: f32,
}

impl<'a> PrefabData<'a> for DeformPrefab {
    type SystemData = WriteStorage<'a, Deform>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let targets = self.targets
            .iter()
            .map(|field| field.clone().into_entity(entities))
            .collect();
        let vertices = self.vertices
            .iter()
            .map(|field| field.clone().into_entity(entities))
            .collect();
        let component = Deform {
            targets,
            vertices,
            stiffness: self.stiffness,
            damp: self.damp,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Default, SystemDesc)]
pub struct ParticleSystem;

impl<'a> System<'a> for ParticleSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Parent>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Deform>,
        ReadStorage<'a, PhysicsHandle<PhysicsRigidBodyTag>>,
        ReadExpect<'a, PhysicsWorld<f32>>,
        ReadExpect<'a, PhysicsTime>,
    );

    fn run(&mut self, (entities, parents, transforms, deforms, bodies, physics_world, time): Self::SystemData) {
        for (entity, deform) in (&*entities, &deforms).join() {
            if deform.targets
                .iter()
                .any(|child| parents
                    .get(*child)
                    .map(|parent| parent.entity != entity)
                    .unwrap_or(true)
                ) { continue; }

            // Targets matches rigid bodies.
            let targets: Vec<_> = if parents.get(entity).is_none() {
                let origins = deform.targets
                    .iter()
                    .map(|entity| transforms
                        .get(*entity)
                        .map(|transform| transform.global_position()))
                    .filter_map(identity)
                    .map(|point| vec![point.x, point.y, point.z])
                    .flatten()
                    .collect();
                let targets = deform.vertices
                    .iter()
                    .map(|entity| transforms
                        .get(*entity)
                        .map(|transform| transform.global_position()))
                    .filter_map(identity)
                    .map(|point| vec![point.x, point.y, point.z])
                    .flatten()
                    .collect();
                let (translation, rotation) = match_shape(origins, targets, 0.01, 10);
                deform.targets
                    .iter()
                    .map(|entity| transforms
                        .get(*entity)
                        .map(|transform| transform.global_position()))
                    .filter_map(identity)
                    .map(|ref point| rotation.transform_point(point))
                    .map(|point| point + translation)
                    .collect()
            } else {
                deform.targets
                    .iter()
                    .map(|entity| transforms
                        .get(*entity)
                        .map(|transform| transform.global_position()))
                    .filter_map(identity)
                    .collect()
            };

            // Rigid bodies matches targets.
            for (target, vertex) in targets.iter().zip(deform.vertices.iter()) {
                if let Some(body) = bodies.get(*vertex) {
                    let position = Point3::from(
                        physics_world
                            .rigid_body_server()
                            .transform(body.get())
                            .translation
                            .vector
                    );
                    let ref impulse = (target - position).scale(deform.stiffness / time.delta_seconds());
                    physics_world.rigid_body_server().apply_impulse(body.get(), impulse);

                    let velocity = physics_world.rigid_body_server().linear_velocity(body.get());
                    let ref force = velocity.scale(-deform.damp);
                    physics_world.rigid_body_server().apply_force(body.get(), force);
                }
            }
        }
    }
}