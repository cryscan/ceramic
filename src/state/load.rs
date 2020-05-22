use amethyst::{
    assets::{Completion, Handle, PrefabLoader, ProgressCounter},
    ecs::prelude::*,
    input::{ElementState, get_key, is_close_requested, StringBindings, VirtualKeyCode},
    prelude::*,
};

use crate::{
    scene::{SceneAsset, SceneFormat, ScenePrefab},
    state::game::GameState,
};

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        println!("Loading...");
        let handle = self.load_scene(data.world, "model/cat.gltf".into());
        data.world.create_entity().with(handle).build();
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
    fn load_scene(&mut self, world: &mut World, path: String) -> Handle<SceneAsset> {
        world.exec(
            |loader: PrefabLoader<'_, ScenePrefab>| {
                let handle = loader.load(path, SceneFormat::default(), &mut self.progress);
                handle
            },
        )
    }
}
