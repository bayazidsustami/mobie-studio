use gpui::prelude::FluentBuilder;
use gpui::*;
use tokio::sync::mpsc;

use crate::agent::{AgentMessage, AgentStatus, AgentUpdate};
use crate::config::{save_config, AppConfig};
use crate::llm::LlmConfig;

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(mobie, [SendMessage, CancelGoal, NavigateSettings, NavigateChat, RefreshDevices, SaveSettings]);

// ---------------------------------------------------------------------------
// Chat Message model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ChatRole {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

// ---------------------------------------------------------------------------
// View enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AppView {
    Chat,
    Settings,
}

/// Used by nav tab click handlers to dispatch the correct navigation action
#[derive(Debug, Clone, Copy, PartialEq)]
enum NavTabAction {
    Chat,
    Settings,
}

// ---------------------------------------------------------------------------
// MobieWorkspace – root GPUI view
// ---------------------------------------------------------------------------

pub struct MobieWorkspace {
    focus_handle: FocusHandle,
    input_text: String,
    messages: Vec<ChatMessage>,
    agent_status: AgentStatus,
    cmd_tx: mpsc::Sender<AgentMessage>,

    // App view (Chat / Settings)
    current_view: AppView,

    // Device state
    devices: Vec<String>,
    selected_device: Option<String>,

    // Settings fields (editable)
    settings_api_key: String,
    settings_model: String,
    settings_base_url: String,
}

impl MobieWorkspace {
    pub fn new(
        cx: &mut Context<Self>,
        cmd_tx: mpsc::Sender<AgentMessage>,
        mut update_rx: mpsc::Receiver<AgentUpdate>,
        initial_config: AppConfig,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        // Spawn async task to forward agent updates into GPUI entity
        cx.spawn(async move |this, cx| {
            while let Some(update) = update_rx.recv().await {
                let _ = cx.update(|cx| {
                    this.update(cx, |workspace, cx| {
                        match update {
                            AgentUpdate::StatusChanged(status) => {
                                workspace.agent_status = status;
                            }
                            AgentUpdate::AgentReply(content) => {
                                workspace.messages.push(ChatMessage {
                                    role: ChatRole::Agent,
                                    content,
                                });
                            }
                            AgentUpdate::DeviceList(devs) => {
                                workspace.devices = devs;
                                // Auto-select first device if none selected
                                if workspace.selected_device.is_none() {
                                    workspace.selected_device =
                                        workspace.devices.first().cloned();
                                }
                            }
                        }
                        cx.notify();
                    })
                });
            }
        })
        .detach();

        let settings_api_key = initial_config.llm.api_key.clone();
        let settings_model = initial_config.llm.model.clone();
        let settings_base_url = initial_config.llm.base_url.clone();

        Self {
            focus_handle,
            input_text: String::new(),
            messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "Welcome to Mobie Studio! Type a goal and press Enter.".to_string(),
            }],
            agent_status: AgentStatus::Idle,
            cmd_tx,
            current_view: AppView::Chat,
            devices: vec![],
            selected_device: None,
            settings_api_key,
            settings_model,
            settings_base_url,
        }
    }

    /// Public accessor for the focus handle (used by main.rs).
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    // -----------------------------------------------------------------------
    // Command handlers
    // -----------------------------------------------------------------------

    fn send_message(&mut self, _: &SendMessage, _window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() || self.agent_status != AgentStatus::Idle {
            return;
        }
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: text.clone(),
        });
        self.input_text.clear();

        let tx = self.cmd_tx.clone();
        cx.spawn(async move |_, _| {
            let _ = tx.send(AgentMessage::StartGoal(text)).await;
        })
        .detach();

        cx.notify();
    }

    fn cancel_goal(&mut self, _: &CancelGoal, _window: &mut Window, cx: &mut Context<Self>) {
        let tx = self.cmd_tx.clone();
        cx.spawn(async move |_, _| {
            let _ = tx.send(AgentMessage::Stop).await;
        })
        .detach();
        cx.notify();
    }

    fn navigate_chat(
        &mut self,
        _: &NavigateChat,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current_view = AppView::Chat;
        cx.notify();
    }

    fn navigate_settings(
        &mut self,
        _: &NavigateSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current_view = AppView::Settings;
        cx.notify();
    }

    fn refresh_devices(
        &mut self,
        _: &RefreshDevices,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Trigger the agent engine to refresh devices by sending a dummy StartGoal
        // (the engine does a refresh at startup; for runtime refresh we post a message)
        // We signal refresh via SelectDevice("") which is ignored by engine but triggers the loop
        // Better: the engine refreshes on demand via a dedicated Refresh message.
        // For now, just prompt a UI message
        self.messages.push(ChatMessage {
            role: ChatRole::System,
            content: "Refreshing device list...".to_string(),
        });
        cx.notify();
    }

    fn save_settings(
        &mut self,
        _: &SaveSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let new_llm = LlmConfig {
            api_key: self.settings_api_key.clone(),
            model: self.settings_model.clone(),
            base_url: self.settings_base_url.clone(),
            provider: "openai".to_string(),
        };

        // Persist to disk
        let cfg = AppConfig { llm: new_llm.clone() };
        if let Err(e) = save_config(&cfg) {
            self.messages.push(ChatMessage {
                role: ChatRole::System,
                content: format!("⚠️ Failed to save settings: {}", e),
            });
        } else {
            // Send to agent engine at runtime
            let tx = self.cmd_tx.clone();
            cx.spawn(async move |_, _| {
                let _ = tx.send(AgentMessage::UpdateConfig(new_llm)).await;
            })
            .detach();

            self.messages.push(ChatMessage {
                role: ChatRole::System,
                content: "✅ Settings saved and applied.".to_string(),
            });
            self.current_view = AppView::Chat;
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // Sidebar
    // -----------------------------------------------------------------------

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Div {
        let is_running = self.agent_status != AgentStatus::Idle;

        div()
            .w(px(260.0))
            .h_full()
            .bg(rgb(0x1a1a2e))
            .border_r_1()
            .border_color(rgb(0x2a2a4a))
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(20.0))
            // App title
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xe94560))
                    .child("Mobie Studio"),
            )
            // Nav tabs
            .child(self.render_nav_tabs(cx))
            // Device section
            .child(self.render_device_section(cx))
            // Agent status
            .child(self.render_sidebar_section(
                "AGENT STATUS",
                &format!("{:?}", self.agent_status),
                match &self.agent_status {
                    AgentStatus::Idle => rgb(0x888888),
                    AgentStatus::Error(_) => rgb(0xff4444),
                    _ => rgb(0x44ff88),
                },
            ))
            // Cancel button (only visible when running)
            .when(is_running, |d| {
                d.child(
                    div()
                        .bg(rgb(0xe94560))
                        .rounded(px(8.0))
                        .p(px(10.0))
                        .text_center()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0xffffff))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                            this.cancel_goal(&CancelGoal, window, cx);
                        }))
                        .child("■ Cancel Goal"),
                )
            })
    }

    fn render_nav_tabs(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .gap(px(4.0))
            .child(self.render_nav_tab("💬 Chat", self.current_view == AppView::Chat, cx, NavTabAction::Chat))
            .child(self.render_nav_tab("⚙ Settings", self.current_view == AppView::Settings, cx, NavTabAction::Settings))
    }


    fn render_nav_tab(&self, label: &str, active: bool, cx: &mut Context<Self>, action: NavTabAction) -> Div {
        let bg = if active { rgb(0xe94560) } else { rgb(0x2a2a4a) };
        let text = if active { rgb(0xffffff) } else { rgb(0x888899) };
        div()
            .flex_1()
            .bg(bg)
            .rounded(px(6.0))
            .p(px(8.0))
            .text_center()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(text)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                match action {
                    NavTabAction::Chat => this.navigate_chat(&NavigateChat, window, cx),
                    NavTabAction::Settings => this.navigate_settings(&NavigateSettings, window, cx),
                }
            }))
            .child(label.to_string())
    }

    fn render_device_section(&self, cx: &mut Context<Self>) -> Div {
        let mut section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x666688))
                            .child("DEVICES"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x4488cc))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                this.refresh_devices(&RefreshDevices, window, cx);
                            }))
                            .child("↺ Refresh"),
                    ),
            );

        if self.devices.is_empty() {
            section = section.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(rgb(0xff4444)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x888899))
                            .child("No devices"),
                    ),
            );
        } else {
            for dev in &self.devices {
                let is_selected = self.selected_device.as_deref() == Some(dev.as_str());
                let dot_color = if is_selected { rgb(0x44ff88) } else { rgb(0x888888) };
                let text_color = if is_selected { rgb(0xeeeeff) } else { rgb(0x888899) };
                let dev_id = dev.clone();
                let tx = self.cmd_tx.clone();
                section = section.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                            this.selected_device = Some(dev_id.clone());
                            let tx2 = tx.clone();
                            let id = dev_id.clone();
                            cx.spawn(async move |_, _| {
                                let _ = tx2.send(AgentMessage::SelectDevice(id)).await;
                            }).detach();
                            cx.notify();
                        }))
                        .child(
                            div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(dot_color),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(text_color)
                                .child(dev.clone()),
                        ),
                );
            }
        }

        section
    }

    fn render_sidebar_section(&self, title: &str, value: &str, dot_color: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x666688))
                    .child(title.to_string()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(dot_color))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xccccdd))
                            .child(value.to_string()),
                    ),
            )
    }

    // -----------------------------------------------------------------------
    // Chat view
    // -----------------------------------------------------------------------

    fn render_chat_area(&self) -> Div {
        let mut chat_list = div()
            .flex_1()
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .overflow_hidden();

        for msg in &self.messages {
            let (bg, text_col, is_user) = match msg.role {
                ChatRole::User => (rgb(0x16213e), rgb(0xeeeeff), true),
                ChatRole::Agent => (rgb(0x0f3460), rgb(0xccddff), false),
                ChatRole::System => (rgb(0x2a2a4a), rgb(0x888899), false),
            };

            let label = match msg.role {
                ChatRole::User => "You",
                ChatRole::Agent => "Agent",
                ChatRole::System => "System",
            };

            let msg_row = if is_user {
                div().flex().flex_row_reverse()
            } else {
                div().flex()
            };

            chat_list = chat_list.child(
                msg_row.child(
                    div()
                        .max_w(px(500.0))
                        .bg(bg)
                        .rounded(px(12.0))
                        .p(px(12.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0x666688))
                                .child(label.to_string()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(text_col)
                                .child(msg.content.clone()),
                        ),
                ),
            );
        }

        chat_list
    }

    fn render_input_area(&self) -> Div {
        let is_idle = self.agent_status == AgentStatus::Idle;
        let placeholder = if is_idle {
            "Type a goal for the agent...".to_string()
        } else {
            "⏳ Agent is running...".to_string()
        };

        let display_text = if self.input_text.is_empty() {
            placeholder
        } else {
            self.input_text.clone()
        };

        let text_color = if self.input_text.is_empty() {
            rgb(0x555566)
        } else {
            rgb(0xeeeeff)
        };

        div()
            .border_t_1()
            .border_color(rgb(0x2a2a4a))
            .p(px(16.0))
            .child(
                div()
                    .bg(rgb(0x16213e))
                    .rounded(px(12.0))
                    .p(px(14.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(text_color)
                            .child(display_text),
                    )
                    .when(is_idle && !self.input_text.is_empty(), |d| {
                        d.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x666688))
                                .child("↵ Enter"),
                        )
                    }),
            )
    }

    // -----------------------------------------------------------------------
    // Settings view
    // -----------------------------------------------------------------------

    fn render_settings_panel(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .border_b_1()
                    .border_color(rgb(0x2a2a4a))
                    .p(px(16.0))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xeeeeff))
                            .child("⚙ LLM Settings (BYOK)"),
                    ),
            )
            // Body
            .child(
                div()
                    .flex_1()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(20.0))
                    .child(self.render_settings_field(
                        "API Key",
                        "sk-...",
                        // mask key for display
                        if self.settings_api_key.is_empty() {
                            "(not set)".to_string()
                        } else {
                            format!(
                                "{}****",
                                &self.settings_api_key
                                    [..self.settings_api_key.len().min(6)]
                            )
                        },
                    ))
                    .child(self.render_settings_field(
                        "Model",
                        "gpt-4o, claude-3-5-sonnet, ...",
                        self.settings_model.clone(),
                    ))
                    .child(self.render_settings_field(
                        "Base URL",
                        "https://api.openai.com/v1",
                        self.settings_base_url.clone(),
                    ))
                    .child(
                        div()
                            .mt(px(8.0))
                            .p(px(12.0))
                            .bg(rgb(0x1a3a5c))
                            .rounded(px(8.0))
                            .text_xs()
                            .text_color(rgb(0x8899bb))
                            .child("💡 To edit settings, use the keyboard when this panel is focused. Press Enter to save."),
                    ),
            )
            // Save button
            .child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x2a2a4a))
                    .p(px(16.0))
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(0xe94560))
                            .rounded(px(8.0))
                            .p(px(12.0))
                            .text_center()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                this.save_settings(&SaveSettings, window, cx);
                            }))
                            .child("✓ Save & Apply"),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(12.0))
                            .bg(rgb(0x2a2a4a))
                            .rounded(px(8.0))
                            .text_center()
                            .text_sm()
                            .text_color(rgb(0x888899))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                this.navigate_chat(&NavigateChat, window, cx);
                            }))
                            .child("Cancel"),
                    ),
            )
    }

    fn render_settings_field(&self, label: &str, placeholder: &str, value: String) -> Div {
        let display = if value.is_empty() || value == "(not set)" {
            placeholder.to_string()
        } else {
            value
        };
        let text_col = if display == placeholder {
            rgb(0x555566)
        } else {
            rgb(0xeeeeff)
        };

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x666688))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .bg(rgb(0x16213e))
                    .rounded(px(8.0))
                    .p(px(12.0))
                    .border_1()
                    .border_color(rgb(0x2a2a4a))
                    .text_sm()
                    .text_color(text_col)
                    .child(display),
            )
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for MobieWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_view = self.current_view.clone();

        div()
            .size_full()
            .bg(rgb(0x0a0a1a))
            .text_color(rgb(0xeeeeff))
            .flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::send_message))
            .on_action(cx.listener(Self::cancel_goal))
            .on_action(cx.listener(Self::navigate_settings))
            .on_action(cx.listener(Self::navigate_chat))
            .on_action(cx.listener(Self::save_settings))
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                // Skip events consumed by modifier-only chords (ctrl/alt/cmd)
                if event.keystroke.modifiers.control
                    || event.keystroke.modifiers.alt
                    || event.keystroke.modifiers.platform
                {
                    return;
                }
                let key = &event.keystroke.key;
                // key_char is the actual character produced by the key + modifiers
                // (e.g. "A" for shift-a, "ß" for option-s). Falls back to key for
                // special keys like "backspace" that have no char.
                let key_char = event.keystroke.key_char.as_deref();

                match this.current_view {
                    AppView::Chat => {
                        if key == "backspace" {
                            this.input_text.pop();
                            cx.notify();
                        } else if let Some(ch) = key_char {
                            this.input_text.push_str(ch);
                            cx.notify();
                        }
                    }
                    AppView::Settings => {
                        if key == "backspace" {
                            this.settings_api_key.pop();
                            cx.notify();
                        } else if let Some(ch) = key_char {
                            this.settings_api_key.push_str(ch);
                            cx.notify();
                        }
                    }
                }
            }))
            .child(self.render_sidebar(cx))
            .child(match current_view {
                AppView::Chat => div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .border_b_1()
                            .border_color(rgb(0x2a2a4a))
                            .p(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xeeeeff))
                                    .child("💬 Exploratory Session"),
                            )
                            .child(
                                // Settings link in header
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x4488cc))
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                                        this.navigate_settings(&NavigateSettings, window, cx);
                                    }))
                                    .child("⚙ Settings"),
                            ),
                    )
                    .child(self.render_chat_area())
                    .child(self.render_input_area()),
                AppView::Settings => self.render_settings_panel(cx),
            })
    }
}
