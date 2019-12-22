use amethyst::{
    assets::{Completion, Handle, Prefab, PrefabLoader, ProgressCounter, RonFormat},
    core::{math::Vector3, Transform},
    prelude::*,
    renderer::{
        Camera,
        rendy::mesh::{Normal, Position, Tangent, TexCoord},
    },
    utils::{auto_fov::AutoFov, scene::BasicScenePrefab},
};

use crate::game::GameState;

type VertexFormat = (Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>);
pub type ScenePrefab = BasicScenePrefab<VertexFormat>;

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
    scene: Option<Handle<Prefab<ScenePrefab>>>,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        print!("Loading.");
        self.create_camera(data.world);

        self.scene = Some(self.load_scene(data.world, "prefabs/sphere.ron".into()));
        data.world
            .create_entity()
            .with(self.scene.as_ref().unwrap().clone())
            .build();
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        match self.progress.complete() {
            Completion::Failed => Trans::Quit,
            Completion::Complete => {
                println!();
                println!("Assets loaded");
                Trans::Switch(Box::new(GameState))
            }
            Completion::Loading => {
                print!(".");
                Trans::None
            }
        }
    }
}

impl LoadState {
    fn create_camera(&self, world: &mut World) {
        let transform = Transform::default()
            .set_translation_xyz(2., 2., 2.)
            .face_towards(Vector3::new(0., 0., 0.), Vector3::new(0., 1., 0.))
            .clone();

        world
            .create_entity()
            .with(Camera::standard_3d(10., 10.))
            .with(AutoFov::default())
            .with(transform)
            .build();
    }

    fn load_scene(&mut self, world: &mut World, path: String) -> Handle<Prefab<ScenePrefab>> {
        world.exec(|loader: PrefabLoader<'_, ScenePrefab>| {
            loader.load(path, RonFormat, &mut self.progress)
        })
    }
}
