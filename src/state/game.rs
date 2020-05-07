use amethyst::{
    input::{ElementState, get_key, is_close_requested, StringBindings, VirtualKeyCode},
    prelude::*,
};

use crate::system::kinematics::bind_chains;

pub struct GameState;

impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        data.world.exec(bind_chains);
    }

    fn handle_event(
        &mut self,
        _data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent<StringBindings>,
    ) -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(event) { return Trans::Quit; }
            match get_key(&event) {
                Some((VirtualKeyCode::Escape, ElementState::Pressed)) => { return Trans::Quit; }
                _ => {}
            }
        }
        Trans::None
    }
}
