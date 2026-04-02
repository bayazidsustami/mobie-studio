use gpui::*;
use mobie::agent::AgentEngine;
use mobie::config::load_config;
use mobie::ui::{
    Backspace, CancelGoal, Copy, Cut, Delete, Enter, MobieWorkspace, MoveEnd, MoveHome, MoveLeft,
    MoveRight, Paste, SaveSettings, SelectAll, SelectEnd, SelectHome, SelectLeft, SelectRight,
    SendMessage,
};
use tracing::info;

fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt::init();
    info!("Starting Mobie Studio...");

    // Load persisted config (or default)
    let initial_config = load_config();

    Application::new().run(move |app| {
        // Bind key actions
        app.bind_keys([
            KeyBinding::new("enter", Enter, None),
            KeyBinding::new("cmd-enter", SendMessage, None),
            KeyBinding::new("ctrl-enter", SendMessage, None),
            KeyBinding::new("escape", CancelGoal, None),
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", MoveLeft, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("right", MoveRight, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("home", MoveHome, None),
            KeyBinding::new("shift-home", SelectHome, None),
            KeyBinding::new("end", MoveEnd, None),
            KeyBinding::new("shift-end", SelectEnd, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("ctrl-a", SelectAll, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("ctrl-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("ctrl-x", Cut, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("ctrl-v", Paste, None),
            KeyBinding::new("cmd-s", SaveSettings, None),
            KeyBinding::new("ctrl-s", SaveSettings, None),
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
                    MobieWorkspace::new(cx, config_for_window.clone(), engine.sender, update_rx)
                });

                let focus_handle = entity.read(app).focus_handle().clone();
                focus_handle.focus(window);

                entity
            },
        )
        .unwrap();
    });
}
