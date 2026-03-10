use gpui::*;
use tokio::sync::mpsc;

use crate::agent::{AgentMessage, AgentStatus, AgentUpdate};

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(mobie, [SendMessage, CancelGoal]);

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
// MobieWorkspace – the root GPUI view
// ---------------------------------------------------------------------------

pub struct MobieWorkspace {
    focus_handle: FocusHandle,
    input_text: String,
    messages: Vec<ChatMessage>,
    agent_status: AgentStatus,
    cmd_tx: mpsc::Sender<AgentMessage>,
}

impl MobieWorkspace {
    pub fn new(
        cx: &mut Context<Self>,
        cmd_tx: mpsc::Sender<AgentMessage>,
        mut update_rx: mpsc::Receiver<AgentUpdate>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        // Spawn an async task to forward agent updates into the GPUI entity
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
                        }
                        cx.notify();
                    })
                });
            }
        })
        .detach();

        Self {
            focus_handle,
            input_text: String::new(),
            messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "Welcome to Mobie Studio! Type a goal and press Enter.".to_string(),
            }],
            agent_status: AgentStatus::Idle,
            cmd_tx,
        }
    }

    /// Public accessor for the focus handle (used by main.rs).
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    fn send_message(&mut self, _: &SendMessage, _window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() {
            return;
        }

        // Add user message to history
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: text.clone(),
        });
        self.input_text.clear();

        // Send goal to the agent engine
        let tx = self.cmd_tx.clone();
        cx.spawn(async move |_, _| {
            let _ = tx.send(AgentMessage::StartGoal(text)).await;
        })
        .detach();

        cx.notify();
    }

    // -----------------------------------------------------------------------
    // UI helpers
    // -----------------------------------------------------------------------

    fn render_sidebar(&self) -> Div {
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
            .child(
                // App title
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xe94560))
                    .child("Mobie Studio"),
            )
            .child(self.render_sidebar_section(
                "DEVICE STATUS",
                "Disconnected",
                rgb(0xff4444),
            ))
            .child(self.render_sidebar_section(
                "AGENT STATUS",
                &format!("{:?}", self.agent_status),
                match self.agent_status {
                    AgentStatus::Idle => rgb(0x888888),
                    AgentStatus::Error(_) => rgb(0xff4444),
                    _ => rgb(0x44ff88),
                },
            ))
            .child(self.render_sidebar_section(
                "LLM CONFIG",
                "BYOK Provider",
                rgb(0x888888),
            ))
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
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(4.0))
                            .bg(dot_color),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xccccdd))
                            .child(value.to_string()),
                    ),
            )
    }

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
                msg_row
                    .child(
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
        let display_text = if self.input_text.is_empty() {
            "Type a goal for the agent...".to_string()
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
                    ),
            )
    }
}

impl Render for MobieWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x0a0a1a))
            .text_color(rgb(0xeeeeff))
            .flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::send_message))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                match &event.keystroke.key {
                    key if key == "backspace" => {
                        this.input_text.pop();
                        cx.notify();
                    }
                    key if key == "space" => {
                        this.input_text.push(' ');
                        cx.notify();
                    }
                    key if key.len() == 1 && !event.keystroke.modifiers.control
                        && !event.keystroke.modifiers.alt
                        && !event.keystroke.modifiers.platform =>
                    {
                        this.input_text.push_str(key);
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            // -- Layout: sidebar + main area --
            .child(self.render_sidebar())
            .child(
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
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xeeeeff))
                                    .child("Exploratory Session"),
                            ),
                    )
                    // Chat messages
                    .child(self.render_chat_area())
                    // Input area
                    .child(self.render_input_area()),
            )
    }
}
