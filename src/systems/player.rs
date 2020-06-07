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
#[get_copy = "pub"]
pub struct Player {
    linear_speed: f32,
    angular_speed: f32,

    stiffness: f32,
    speed_limit: [f32; 2],
    acceleration: f32,

    #[serde(skip, default = "Vector3::zero")]
    movement: Vector3<f32>,
    #[serde(skip, default = "UnitQuaternion::identity")]
    spinning: UnitQuaternion<f32>,
}

impl Player {
    pub fn velocity(&self) -> Vector3<f32> {
        self.movement.scale(self.linear_speed)
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
            )
                .try_normalize(EPSILON)
                .unwrap_or(Vector3::zero());
            let spinning = UnitQuaternion::from_euler_angles(
                0.0,
                player.angular_speed * input.axis_value("move_x").unwrap_or(0.0),
                0.0,
            );

            let delta_seconds = time.delta_seconds();
            let [min, max] = player.speed_limit;
            player.linear_speed += input.axis_value("move_y").unwrap_or(0.0) * delta_seconds * player.acceleration;
            player.linear_speed = player.linear_speed.min(max).max(min);

            let decay = 1.0 - (-player.stiffness * delta_seconds).exp();
            player.movement += decay * (movement - player.movement.clone());
            player.spinning *= (player.spinning.inverse() * spinning).powf(decay);

            transform.append_translation(delta_seconds * player.linear_speed * &player.movement);
            if let Some((axis, angle)) = player.spinning.axis_angle() {
                transform.append_rotation(axis, angle * delta_seconds);
            }
        }
    }
}