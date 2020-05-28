#![feature(tau_constant, option_zip)]

use amethyst::{
    animation::{AnimationBundle, VertexSkinningBundle},
    controls::ArcBallControlBundle,
    core::{Transform, TransformBundle},
    input::{InputBundle, StringBindings},
    prelude::*,
    renderer::{
        plugins::{RenderDebugLines, RenderPbr3D, RenderSkybox, RenderToWindow},
        RenderingBundle,
        types::DefaultBackend,
    },
    utils::{application_root_dir, auto_fov::AutoFovSystem},
};
use amethyst_nphysics::NPhysicsBackend;
use amethyst_physics::PhysicsBundle;

use crate::{
    scene::SceneLoaderSystemDesc,
    state::load::LoadState,
    systems::{
        animal::{BounceSystem, LocomotionSystem, OscillatorSystem, TrackSystem},
        kinematics::KinematicsSystem,
        player::PlayerSystem,
    },
};

mod scene;
mod state;
mod systems;
mod utils;

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
    ).with_dep(&["gltf_loader"]);

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
        .with_bundle(
            PhysicsBundle::<f32, NPhysicsBackend>::new()
                .with_frames_per_seconds(60)
                .with_in_physics(OscillatorSystem::default(), "oscillator".into(), vec![])
        )?
        .with_system_desc(SceneLoaderSystemDesc::default(), "gltf_loader", &[])
        .with(PlayerSystem::default(), "player", &[])
        .with_bundle(animation_bundle)?
        .with_bundle(ArcBallControlBundle::<StringBindings>::new())?
        .with_bundle(TransformBundle::new().with_dep(&[
            "animation_control",
            "sampler_interpolation",
            "free_rotation",
        ]))?
        .with_bundle(VertexSkinningBundle::new().with_dep(&[
            "transform_system",
            "animation_control",
            "sampler_interpolation",
        ]))?
        .with(KinematicsSystem::default(), "kinematics", &["transform_system"])
        .with(TrackSystem::default(), "track", &["transform_system"])
        .with(BounceSystem::default(), "bounce", &["transform_system"])
        .with(LocomotionSystem::default(), "locomotion", &["transform_system"])
        .with_bundle(input_bundle)?
        .with(AutoFovSystem::new(), "auto_fov", &[]);

    let mut game = Application::new(assets_dir, LoadState::default(), game_data)?;
    game.run();

    Ok(())
}
