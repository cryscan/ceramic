use amethyst::{
    assets::PrefabData,
    core::{math::Point3, Transform},
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
    utils::transform::TransformTrait,
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
pub struct Spring {
    target: Entity,
    stiffness: f32,
    damp: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct SpringPrefab {
    pub target: RedirectField,
    #[redirect(skip)]
    pub stiffness: f32,
    #[redirect(skip)]
    pub damp: f32,
}

impl<'a> PrefabData<'a> for SpringPrefab {
    type SystemData = WriteStorage<'a, Spring>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Spring {
            target: self.target.clone().into_entity(entities),
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
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Spring>,
        ReadStorage<'a, PhysicsHandle<PhysicsRigidBodyTag>>,
        ReadExpect<'a, PhysicsWorld<f32>>,
        ReadExpect<'a, PhysicsTime>,
    );

    fn run(&mut self, (transforms, springs, bodies, physics_world, time): Self::SystemData) {
        for (spring, body) in (&springs, &bodies).join() {
            if let Some(target) = transforms
                .get(spring.target)
                .map(|transform| transform.global_position()) {
                let position = Point3::from(
                    physics_world
                        .rigid_body_server()
                        .transform(body.get())
                        .translation
                        .vector
                );
                let ref impulse = (target - position).scale(spring.stiffness / time.delta_seconds());
                physics_world.rigid_body_server().apply_impulse(body.get(), impulse);
            }

            let velocity = physics_world.rigid_body_server().linear_velocity(body.get());
            let ref damp = velocity.scale(-spring.damp);
            physics_world.rigid_body_server().apply_force(body.get(), damp);
        }
    }
}