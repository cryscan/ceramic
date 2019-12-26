use amethyst::{
    assets::{Completion, PrefabLoader, ProgressCounter, RonFormat},
    ecs::prelude::*,
    prelude::*,
};

use crate::prefab::scene::{Scene, ScenePrefab};
use crate::state::game::GameState;

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        print!("Loading...");
        self.load_scene(data.world, "prefab/scene.ron".into());
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        match self.progress.complete() {
            Completion::Failed => Trans::Quit,
            Completion::Complete => {
                println!();
                println!("Assets loaded");
                Trans::Switch(Box::new(GameState))
            }
            Completion::Loading => Trans::None,
        }
    }
}

impl LoadState {
    fn load_scene(&mut self, world: &mut World, path: String) {
        world.exec(
            |(loader, mut scene): (PrefabLoader<'_, ScenePrefab>, Write<'_, Scene>)| {
                let handle = loader.load(path, RonFormat, &mut self.progress);
                scene.handle = Some(handle);
            },
        )
    }
}
