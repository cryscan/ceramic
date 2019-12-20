use amethyst::{
    assets::{AssetStorage, Completion, Loader, ProgressCounter},
    core::{math::Vector3, Transform},
    prelude::{Builder, GameData, SimpleState, SimpleTrans, StateData, Trans, World, WorldExt},
    renderer::{
        Camera,
        light::{DirectionalLight, Light},
        Mesh,
        mtl::{Material, MaterialDefaults},
        palette::{Srgb, Srgba},
        rendy::{
            mesh::{Normal, Position, Tangent, TexCoord},
            texture::palette::load_from_srgba,
        },
        shape::Shape::Sphere, Texture, types::MeshData,
    },
};

use crate::game::GameState;

#[derive(Default)]
pub struct LoadState {
    progress: ProgressCounter,
}

impl SimpleState for LoadState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        println!("Loading...");
        create_camera(data.world);
        create_light(data.world);
        create_sphere(data.world);
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

fn create_camera(world: &mut World) {
    let transform = Transform::default()
        .set_translation_xyz(10., 10., 10.)
        .face_towards(Vector3::new(0., 0., 0.), Vector3::new(0., 1., 0.))
        .clone();

    world
        .create_entity()
        .with(Camera::standard_3d(10., 10.))
        .with(transform)
        .build();
}

fn create_light(world: &mut World) {
    let light: Light = DirectionalLight {
        color: Srgb::new(1., 0.6, 1.),
        direction: Vector3::new(0., -1., -1.).normalize(),
        intensity: 1.,
    }
        .into();
    let transform = Transform::default().set_translation_xyz(2., 4., 0.).clone();

    world.create_entity().with(light).with(transform).build();
}

fn create_sphere(world: &mut World) {
    let mesh = {
        let loader = world.fetch::<Loader>();
        let ref storage = world.fetch::<AssetStorage<Mesh>>();

        let scale = (1., 1., 1.);
        let data: MeshData = Sphere(32, 32)
            .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(Some(scale))
            .into();
        loader.load_from_data(data, (), storage)
    };
    let material = {
        let loader = world.fetch::<Loader>();

        let ref storage = world.fetch::<AssetStorage<Texture>>();
        let color = Srgba::new(1., 1., 1., 1.);
        let albedo = loader.load_from_data(load_from_srgba(color).into(), (), storage);

        let ref storage = world.fetch::<AssetStorage<Material>>();
        let material_defaults = world.fetch::<MaterialDefaults>().0.clone();
        loader.load_from_data(
            Material {
                albedo,
                ..material_defaults
            },
            (),
            storage,
        )
    };
    let transform = Transform::default();

    world
        .create_entity()
        .with(mesh)
        .with(material)
        .with(transform)
        .build();
}
