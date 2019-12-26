use amethyst::prelude::*;

use crate::prefab::scene::Scene;

pub struct GameState;

impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let scene_handle = data.world.fetch::<Scene>().handle.as_ref().unwrap().clone();
        data.world.create_entity().with(scene_handle).build();
    }
}
