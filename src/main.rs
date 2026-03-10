use gpui::*;

pub mod agent;
pub mod device;
pub mod llm;
pub mod ui;

struct MobieStudio {
    text: SharedString,
}

impl Render for MobieStudio {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .flex()
            .bg(rgb(0x2e2e2e))
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(self.text.clone())
    }
}

fn main() {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Mobie Studio...");

    let app = Application::new();
    
    app.run(|cx: &mut App| {
        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Mobie Studio".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            cx.new(|_cx| MobieStudio {
                text: "Welcome to Mobie Studio (Agent Ready)".into(),
            })
        }).unwrap();
    });
}
