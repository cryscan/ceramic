use amethyst::{
    assets::{Completion, PrefabLoader, ProgressCounter, RonFormat},
    ecs::prelude::*,
    input::{ElementState, get_key, is_close_requested, StringBindings, VirtualKeyCode},
    prelude::*,
};

use crate::{
    scene::{Scene, ScenePrefab},
    state::game::GameState,
};

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        println!("Loading...");
        self.load_scene(data.world, "prefab/scene.ron".into());
    }

    fn handle_event(
        &mut self,
        _data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent<StringBindings>)
        -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(event) { return Trans::Quit; }
            match get_key(event) {
                Some((VirtualKeyCode::Escape, ElementState::Pressed)) => { return Trans::Quit; }
                _ => {}
            }
        }
        Trans::None
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        match self.progress.complete() {
            Completion::Failed => Trans::Quit,
            Completion::Complete => {
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
