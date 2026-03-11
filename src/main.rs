pub mod agent;
pub mod config;
pub mod device;
pub mod llm;
pub mod ui;
pub mod yaml_exporter;

use gpui::*;
use tracing::info;

use crate::agent::AgentEngine;
use crate::config::load_config;
use crate::ui::{MobieWorkspace, CancelGoal, SendMessage};

fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt::init();
    info!("Starting Mobie Studio...");

    // Load persisted config (or default)
    let initial_config = load_config();

    Application::new().run(move |app| {
        // Bind key actions
        app.bind_keys([
            KeyBinding::new("enter", SendMessage, None),
            KeyBinding::new("escape", CancelGoal, None),
        ]);

        let config_for_window = initial_config.clone();

        app.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.0), px(780.0)),
                    app,
                ))),
                ..Default::default()
            },
            move |window, app| {
                // Create channels for agent ↔ UI communication
                let (update_tx, update_rx) = tokio::sync::mpsc::channel(64);
                let (engine, cmd_rx) =
                    AgentEngine::start(update_tx.clone(), config_for_window.clone());

                // Spawn the agent loop on a dedicated OS thread with its own Tokio runtime.
                // GPUI's background_executor has no Tokio reactor, so tokio APIs would panic there.
                std::thread::spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to build Tokio runtime for agent")
                        .block_on(AgentEngine::run_loop(cmd_rx, update_tx))
                });

                // Build and focus the root workspace view
                let entity = app.new(|cx| {
                    MobieWorkspace::new(cx, engine.sender, update_rx, config_for_window.clone())
                });

                let focus_handle = entity.read(app).focus_handle().clone();
                focus_handle.focus(window);

                entity
            },
        )
        .unwrap();
    });
}
