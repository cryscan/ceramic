use amethyst::{
    assets::PrefabData,
    derive::SystemDesc,
    ecs::{Component, prelude::*},
    error::Error,
};
use easer::functions::{Easing, Expo};
use serde::{Deserialize, Serialize};

use ceramic_derive::Redirect;
use redirect::Redirect;

use crate::{
    scene::RedirectField,
    systems::{particle::Spring, player::Player},
};

#[derive(Debug, Copy, Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Tail {
    player: Entity,
    stiffness: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, Redirect)]
pub struct TailPrefab {
    pub player: RedirectField,
    #[redirect(skip)]
    pub stiffness: [f32; 2],
}

impl<'a> PrefabData<'a> for TailPrefab {
    type SystemData = WriteStorage<'a, Tail>;
    type Result = ();

    fn add_to_entity(
        &self,
        entity: Entity,
        data: &mut Self::SystemData,
        entities: &[Entity],
        _children: &[Entity],
    ) -> Result<Self::Result, Error> {
        let component = Tail {
            player: self.player.clone().into_entity(entities),
            stiffness: self.stiffness,
        };
        data.insert(entity, component).map(|_| ()).map_err(Into::into)
    }
}

#[derive(Default, SystemDesc)]
pub struct TailSystem;

impl<'a> System<'a> for TailSystem {
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Tail>,
        WriteStorage<'a, Spring>,
    );

    fn run(&mut self, (players, tails, mut springs): Self::SystemData) {
        for (tail, spring) in (&tails, &mut springs).join() {
            if let Some(player) = players.get(tail.player) {
                let speed = player.velocity().norm();
                let [min, max] = player.speed_limit();
                let [loose, tight] = tail.stiffness;
                let stiffness = Expo::ease_in(speed - min, loose, tight - loose, max - min);
                spring.set_stiffness(stiffness);
            }
        }
    }
}