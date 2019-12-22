use amethyst::{
    assets::{
        AssetStorage, Completion, Handle, Loader, Prefab, PrefabLoader, ProgressCounter, RonFormat,
    },
    core::{math::Vector3, Transform},
    prelude::{Builder, GameData, SimpleState, SimpleTrans, StateData, Trans, World, WorldExt},
    renderer::{
        Camera,
        light::{DirectionalLight, Light},
        mtl::{Material, MaterialDefaults},
        palette::{Srgb, Srgba},
        rendy::{
            mesh::{Normal, Position, Tangent, TexCoord},
            texture::palette::load_from_srgba,
        },
    },
    utils::scene::BasicScenePrefab,
};

use crate::game::GameState;

type VertexFormat = (Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>);
pub type ScenePrefab = BasicScenePrefab<VertexFormat>;

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        print!("Loading.");
        self.create_camera(data.world);

        let sphere_handle = self.load_scene(data.world, "prefabs/sphere.ron");
        data.world.create_entity().with(sphere_handle).build();
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
    fn create_camera(self: &Self, world: &mut World) {
        let transform = Transform::default()
            .set_translation_xyz(2., 2., 2.)
            .face_towards(Vector3::new(0., 0., 0.), Vector3::new(0., 1., 0.))
            .clone();

        world
            .create_entity()
            .with(Camera::standard_3d(10., 10.))
            .with(transform)
            .build();
    }

    fn load_scene(
        self: &mut Self,
        world: &mut World,
        path: &'static str,
    ) -> Handle<Prefab<ScenePrefab>> {
        world.exec(|loader: PrefabLoader<'_, ScenePrefab>| {
            loader.load(path, RonFormat, &mut self.progress)
        })
    }
}
