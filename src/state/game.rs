use amethyst::{
    input::{get_key, is_close_requested, is_key_down, StringBindings, VirtualKeyCode},
    prelude::*,
};

use crate::scene::Scene;

pub struct GameState;

impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let scene_handle = data.world.fetch::<Scene>().handle.as_ref().unwrap().clone();
        data.world.create_entity().with(scene_handle).build();
    }

    fn handle_event(
        &mut self,
        _data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent<StringBindings>,
    ) -> Trans<GameData<'static, 'static>, StateEvent<StringBindings>> {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(event) || is_key_down(event, VirtualKeyCode::Escape) {
                return Trans::Quit;
            }
            match get_key(&event) {
                Some(_) => {}
                None => {}
            }
        }
        Trans::None
    }
}
