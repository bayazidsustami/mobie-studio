pub mod agent;
pub mod device;
pub mod llm;
pub mod ui;

use gpui::*;
use tracing::info;

use crate::agent::AgentEngine;
use crate::ui::{MobieWorkspace, SendMessage};

fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt::init();
    info!("Starting Mobie Studio...");

    Application::new().run(|app| {
        // Bind key actions
        app.bind_keys([
            KeyBinding::new("enter", SendMessage, None),
        ]);

        app.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.0), px(780.0)),
                    app,
                ))),
                ..Default::default()
            },
            |window, app| {
                // Create channels for agent ↔ UI communication
                let (update_tx, update_rx) = tokio::sync::mpsc::channel(64);
                let (engine, cmd_rx) = AgentEngine::start(update_tx.clone());

                // Spawn the agent loop in the background
                app.background_executor()
                    .spawn(AgentEngine::run_loop(cmd_rx, update_tx))
                    .detach();

                // Focus the window for keyboard input
                let entity = app.new(|cx| {
                    MobieWorkspace::new(cx, engine.sender, update_rx)
                });

                let focus_handle = entity.read(app).focus_handle().clone();
                focus_handle.focus(window);

                entity
            },
        )
        .unwrap();
    });
}
