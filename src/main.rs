use amethyst::{
    animation::{AnimationBundle, VertexSkinningBundle},
    assets::PrefabLoaderSystemDesc,
    controls::ArcBallControlBundle,
    core::{Transform, TransformBundle},
    gltf::GltfSceneLoaderSystemDesc,
    input::{InputBundle, StringBindings},
    prelude::*,
    renderer::{
        plugins::{RenderDebugLines, RenderPbr3D, RenderSkybox, RenderToWindow},
        RenderingBundle,
        types::DefaultBackend,
    },
    utils::{application_root_dir, auto_fov::AutoFovSystem},
};

use crate::{
    scene::ScenePrefab,
    state::load::LoadState,
    system::kinematics::KinematicsSystem};
use crate::system::player::PlayerSystem;

mod scene;
mod state;
mod system;

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let config_dir = app_root.join("config");
    let display_config_path = config_dir.join("display.ron");
    let bindings_path = config_dir.join("bindings.ron");
    let assets_dir = app_root.join("assets");

    let animation_bundle = AnimationBundle::<usize, Transform>::new(
        "animation_control",
        "sampler_interpolation",
    )
        .with_dep(&["gltf_loader", "kinematics"]);

    let input_bundle = InputBundle::<StringBindings>::new()
        .with_bindings_from_file(bindings_path)?;

    let game_data = GameDataBuilder::default()
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(RenderToWindow::from_config_path(display_config_path)?)
                .with_plugin(RenderPbr3D::default().with_skinning())
                .with_plugin(RenderDebugLines::default())
                .with_plugin(RenderSkybox::default()),
        )?
        .with_system_desc(
            PrefabLoaderSystemDesc::<ScenePrefab>::default(),
            "scene_loader",
            &[],
        )
        .with_system_desc(
            GltfSceneLoaderSystemDesc::default(),
            "gltf_loader",
            &["scene_loader"],
        )
        .with(PlayerSystem::default(), "player", &[])
        .with(KinematicsSystem::default(), "kinematics", &[])
        .with_bundle(animation_bundle)?
        .with_bundle(ArcBallControlBundle::<StringBindings>::new())?
        .with_bundle(TransformBundle::new().with_dep(&[
            "animation_control",
            "sampler_interpolation",
            "free_rotation",
            "player",
            "kinematics",
        ]))?
        .with_bundle(VertexSkinningBundle::new().with_dep(&[
            "transform_system",
            "animation_control",
            "sampler_interpolation",
        ]))?
        .with_bundle(input_bundle)?
        .with(AutoFovSystem::new(), "auto_fov", &[]);

    let mut game = Application::new(assets_dir, LoadState::default(), game_data)?;
    game.run();

    Ok(())
}
