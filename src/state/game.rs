use amethyst::{
    core::math::{Point3, Vector3},
    input::{ElementState, get_key, is_close_requested, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        debug_drawing::DebugLinesComponent,
        palette::Srgba,
    },
};

pub struct GameState;

impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let mut debug_lines_component = DebugLinesComponent::with_capacity(100);
        let width: u32 = 100;
        let depth: u32 = 100;
        let main_color = Srgba::new(0.4, 0.4, 0.4, 1.0);

        // Grid lines in X-axis
        for x in 0..=width {
            let (x, width, depth) = (x as f32, width as f32, depth as f32);

            let position = Point3::new(x - width / 2.0, 0.0, -depth / 2.0);
            let direction = Vector3::new(0.0, 0.0, depth);

            debug_lines_component.add_direction(position, direction, main_color);
        }

        // Grid lines in Z-axis
        for z in 0..=depth {
            let (z, width, depth) = (z as f32, width as f32, depth as f32);

            let position = Point3::new(-width / 2.0, 0.0, z - depth / 2.0);
            let direction = Vector3::new(width, 0.0, 0.0);

            debug_lines_component.add_direction(position, direction, main_color);
        }
        data.world.register::<DebugLinesComponent>();
        data.world
            .create_entity()
            .with(debug_lines_component)
            .build();
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
