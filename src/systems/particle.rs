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
    utils::{transform::TransformStorageExt},
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
        data: &mut Self::SystemData,
        _: &[Entity],
        _: &[Entity],
    ) -> Result<Self::Result, Error> {
        let (
            physics_world,
            rigid_bodies
        ) = data;

        let rigid_body = {
            let ref desc = RigidBodyDesc {
                mode: BodyMode::Dynamic,
                mass: self.mass,
                ..Default::default()
            };
            physics_world.rigid_body_server().create(desc)
        };
        rigid_bodies.insert(entity, rigid_body)?;

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Component)]
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
    #[serde(default)]
    pub stiffness: f32,
    #[redirect(skip)]
    #[serde(default)]
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
    );

    fn run(&mut self, (transforms, springs, rigid_bodies, physics_world): Self::SystemData) {
        for (spring, rigid_body) in (&springs, &rigid_bodies).join() {
            let target = transforms.global_position(spring.target);
            let transform = physics_world.rigid_body_server().transform(rigid_body.get());
            let position = Point3::from(transform.translation.vector);
            let ref mut force = (target - position).scale(spring.stiffness);

            let velocity = physics_world.rigid_body_server().linear_velocity(rigid_body.get());
            *force += velocity.scale(-spring.damp);

            physics_world
                .rigid_body_server()
                .apply_force(rigid_body.get(), force);
        }
    }
}