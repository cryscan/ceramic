use amethyst::{
    core::transform::TransformBundle,
    prelude::{Application, GameDataBuilder},
    renderer::{
        plugins::{RenderFlat3D, RenderShaded3D, RenderToWindow},
        RenderingBundle,
        types::DefaultBackend,
    },
    utils::application_root_dir,
};

use load::LoadState;

mod game;
mod load;

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let config_dir = app_root.join("config");
    let display_config_path = config_dir.join("display.ron");
    let assets_dir = app_root.join("assets");

    let game_data = GameDataBuilder::default()
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderShaded3D::default()),
        )?
        .with_bundle(TransformBundle::new())?;

    let mut game = Application::new(assets_dir, LoadState::default(), game_data)?;
    game.run();

    Ok(())
}
