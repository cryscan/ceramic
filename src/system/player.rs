use std::f32::EPSILON;

use amethyst::{
    assets::PrefabData,
    core::{
        math::{UnitQuaternion, Vector3},
        timing::Time,
        transform::Transform,
    },
    derive::{PrefabData, SystemDesc},
    ecs::prelude::*,
    error::Error,
    input::{InputHandler, StringBindings},
};
use getset::{CopyGetters, Getters};
use num_traits::identities::Zero;
use serde::{Deserialize, Serialize};

#[derive(Getters, CopyGetters, Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Player {
    #[get_copy = "pub"]
    linear_speed: f32,
    #[get_copy = "pub"]
    angular_speed: f32,
    #[serde(skip, default = "Vector3::zero")]
    #[get = "pub"]
    translation: Vector3<f32>,
    #[serde(skip, default = "UnitQuaternion::identity")]
    #[get = "pub"]
    rotation: UnitQuaternion<f32>,
}

impl Player {
    pub fn velocity(&self) -> Vector3<f32> {
        self.translation.scale(self.linear_speed)
    }
}

impl Component for Player {
    type Storage = VecStorage<Self>;
}

#[derive(Default, SystemDesc)]
pub struct PlayerSystem;

impl<'a> System<'a> for PlayerSystem {
    type SystemData = (
        WriteStorage<'a, Player>,
        WriteStorage<'a, Transform>,
        Read<'a, InputHandler<StringBindings>>,
        Read<'a, Time>,
    );

    fn run(&mut self, (mut players, mut transforms, input, time): Self::SystemData) {
        for (player, transform) in (&mut players, &mut transforms).join() {
            let movement = Vector3::new(
                0.0,
                0.0,
                input.axis_value("move_z").unwrap_or(0.0),
            );
            player.rotation = UnitQuaternion::from_euler_angles(
                0.0,
                input.axis_value("move_x").unwrap_or(0.0),
                0.0,
            );
            player.translation = movement.try_normalize(EPSILON).unwrap_or(Vector3::zero());
            transform.append_translation(time.delta_seconds() * player.linear_speed * &player.translation);
            if let Some((axis, angle)) = player.rotation.axis_angle() {
                transform.append_rotation(axis, angle * time.delta_seconds());
            }
        }
    }
}