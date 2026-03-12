use gpui::prelude::FluentBuilder;
use gpui::*;
use tokio::sync::mpsc;
use std::ops::Range;

use crate::agent::{AgentMessage, AgentStatus, AgentUpdate};
use crate::config::{save_config, AppConfig};
use crate::llm::LlmConfig;

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(mobie, [
    SendMessage, 
    CancelGoal, 
    NavigateSettings, 
    NavigateChat, 
    RefreshDevices, 
    SaveSettings,
    Backspace,
    Delete,
    SelectAll,
    Enter,
    Copy,
    Cut,
    Paste,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    SelectLeft,
    SelectRight,
    SelectHome,
    SelectEnd
]);

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
// TextInput View
// ---------------------------------------------------------------------------

pub struct TextInput {
    focus_handle: FocusHandle,
    text: String,
    cursor_offset: usize, // Byte offset
    selection_anchor: Option<usize>, // Byte offset
    placeholder: String,
    is_masked: bool,
}

impl TextInput {
    fn new(cx: &mut Context<Self>, placeholder: String, initial_value: String) -> Self {
        let len = initial_value.len();
        Self {
            focus_handle: cx.focus_handle(),
            text: initial_value,
            cursor_offset: len,
            selection_anchor: None,
            placeholder,
            is_masked: false,
        }
    }

    pub fn set_masked(&mut self, masked: bool) {
        self.is_masked = masked;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn selection_range(&self) -> Option<Range<usize>> {
        let anchor = self.selection_anchor?;
        let start = anchor.min(self.cursor_offset);
        let end = anchor.max(self.cursor_offset);
        
        if self.text.is_char_boundary(start) && self.text.is_char_boundary(end) {
            if start != end {
                return Some(start..end);
            }
        }
        None
    }

    fn backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            self.text.replace_range(range.clone(), "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
        } else if self.cursor_offset > 0 {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i < self.cursor_offset);
            if let Some((prev_offset, _)) = char_indices.next_back() {
                self.text.replace_range(prev_offset..self.cursor_offset, "");
                self.cursor_offset = prev_offset;
            }
        }
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            self.text.replace_range(range.clone(), "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
        } else if self.cursor_offset < self.text.len() {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i > self.cursor_offset);
            if let Some((next_offset, _)) = char_indices.next() {
                self.text.replace_range(self.cursor_offset..next_offset, "");
            } else {
                self.text.replace_range(self.cursor_offset.., "");
            }
        }
        cx.notify();
    }

    fn move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        if self.cursor_offset > 0 {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i < self.cursor_offset);
            if let Some((prev_offset, _)) = char_indices.next_back() {
                self.cursor_offset = prev_offset;
            }
        }
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        if self.cursor_offset > 0 {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i < self.cursor_offset);
            if let Some((prev_offset, _)) = char_indices.next_back() {
                self.cursor_offset = prev_offset;
            }
        }
        cx.notify();
    }

    fn move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        if self.cursor_offset < self.text.len() {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i > self.cursor_offset);
            if let Some((next_offset, _)) = char_indices.next() {
                self.cursor_offset = next_offset;
            } else {
                self.cursor_offset = self.text.len();
            }
        }
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        if self.cursor_offset < self.text.len() {
            let mut char_indices = self.text.char_indices().filter(|(i, _)| *i > self.cursor_offset);
            if let Some((next_offset, _)) = char_indices.next() {
                self.cursor_offset = next_offset;
            } else {
                self.cursor_offset = self.text.len();
            }
        }
        cx.notify();
    }

    fn move_home(&mut self, _: &MoveHome, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        self.cursor_offset = 0;
        cx.notify();
    }

    fn select_home(&mut self, _: &SelectHome, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        self.cursor_offset = 0;
        cx.notify();
    }

    fn move_end(&mut self, _: &MoveEnd, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        self.cursor_offset = self.text.len();
        cx.notify();
    }

    fn select_end(&mut self, _: &SelectEnd, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        self.cursor_offset = self.text.len();
        cx.notify();
    }

    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = Some(0);
        self.cursor_offset = self.text.len();
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.text[range].to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.text[range.clone()].to_string()));
            self.text.replace_range(range.clone(), "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
            cx.notify();
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.replace_text_in_range(self.selection_range(), &text, window, cx);
            }
        }
    }

    fn enter(&mut self, _: &Enter, _window: &mut Window, cx: &mut Context<Self>) {
        cx.dispatch_action(&SendMessage);
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        if event.modifiers.shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_offset);
            }
        } else {
            self.selection_anchor = None;
        }
        cx.notify();
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        
        div()
            .id("text-input")
            .flex()
            .items_center()
            .flex_1()
            .h_full()
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::move_home))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::move_end))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::enter))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .child(TextInputElement {
                view: cx.entity().clone(),
            })
    }
}

struct TextInputElement {
    view: Entity<TextInput>,
}

impl IntoElement for TextInputElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        app: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], app), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _app: &mut App,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (text, is_focused, text_is_empty, cursor_offset, selection_range, focus_handle, _is_masked) = {
            let state = self.view.read(cx);
            let display_text = if state.text.is_empty() {
                state.placeholder.clone()
            } else if state.is_masked {
                "*".repeat(state.text.chars().count())
            } else {
                state.text.clone()
            };

            // If masked, we need to map the byte offsets to character counts because the masked string
            // has 1-byte chars ('*') for every char in the original string.
            let (mapped_cursor, mapped_selection) = if state.is_masked && !state.text.is_empty() {
                let char_count_to_cursor = state.text[..state.cursor_offset].chars().count();
                let mapped_sel = state.selection_range().map(|r| {
                    let start = state.text[..r.start].chars().count();
                    let end = state.text[..r.end].chars().count();
                    start..end
                });
                (char_count_to_cursor, mapped_sel)
            } else {
                (state.cursor_offset, state.selection_range())
            };

            (
                display_text,
                state.focus_handle.is_focused(window),
                state.text.is_empty(),
                mapped_cursor,
                mapped_selection,
                state.focus_handle.clone(),
                state.is_masked,
            )
        };

        let text_color = if text_is_empty {
            rgb(0x555566)
        } else {
            rgb(0xeeeeff)
        };

        let font_size = px(14.0);
        let shaped_line = window.text_system().shape_line(
            text.clone().into(),
            font_size,
            &[TextRun {
                len: text.len(),
                color: text_color.into(),
                background_color: None,
                underline: None,
                strikethrough: None,
                font: window.text_style().font(),
            }],
            None
        );

        // Map mouse clicks to cursor position
        window.on_mouse_event({
            let view = self.view.clone();
            let shaped_line = shaped_line.clone();
            let focus_handle = focus_handle.clone();
            let bounds = bounds;
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    window.focus(&focus_handle);
                    let local_x = event.position.x - bounds.origin.x;
                    if let Some(index) = shaped_line.index_for_x(local_x) {
                        view.update(cx, |this, cx| {
                            // If masked, 'index' is the char count. We need to convert it back to byte offset.
                            let actual_index = if this.is_masked && !this.text.is_empty() {
                                this.text.char_indices().map(|(i, _)| i).nth(index).unwrap_or(this.text.len())
                            } else {
                                index
                            };

                            if event.modifiers.shift {
                                if this.selection_anchor.is_none() {
                                    this.selection_anchor = Some(this.cursor_offset);
                                }
                            } else {
                                this.selection_anchor = None;
                            }
                            this.cursor_offset = actual_index;
                            cx.notify();
                        });
                    }
                }
            }
        });

        // Map mouse drag to selection
        window.on_mouse_event({
            let view = self.view.clone();
            let shaped_line = shaped_line.clone();
            let bounds = bounds;
            move |event: &MouseMoveEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble && event.pressed_button == Some(MouseButton::Left) && bounds.contains(&event.position) {
                    let local_x = event.position.x - bounds.origin.x;
                    if let Some(index) = shaped_line.index_for_x(local_x) {
                        view.update(cx, |this, cx| {
                            let actual_index = if this.is_masked && !this.text.is_empty() {
                                this.text.char_indices().map(|(i, _)| i).nth(index).unwrap_or(this.text.len())
                            } else {
                                index
                            };

                            if this.selection_anchor.is_none() {
                                this.selection_anchor = Some(this.cursor_offset);
                            }
                            this.cursor_offset = actual_index;
                            cx.notify();
                        });
                    }
                }
            }
        });

        // Paint selection highlight
        if is_focused {
            if let Some(ref range) = selection_range {
                let start_x = shaped_line.x_for_index(range.start);
                let end_x = shaped_line.x_for_index(range.end);
                window.paint_quad(fill(
                    Bounds {
                        origin: point(bounds.origin.x + start_x, bounds.origin.y),
                        size: size(end_x - start_x, window.line_height()),
                    },
                    rgba(0x4488cc44),
                ));
            }
        }

        let _ = shaped_line.paint(bounds.origin, window.line_height(), window, cx);

        window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.view.clone()), cx);

        if is_focused && selection_range.is_none() {
            let cursor_x = shaped_line.x_for_index(cursor_offset);
            window.paint_quad(fill(
                Bounds {
                    origin: point(bounds.origin.x + cursor_x, bounds.origin.y),
                    size: size(px(2.0), window.line_height()),
                },
                rgb(0xe94560),
            ));
        }
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(&mut self, range: Range<usize>, _adjusted_range: &mut Option<Range<usize>>, _window: &mut Window, _cx: &mut Context<Self>) -> Option<String> {
        self.text.get(range).map(|s| s.to_string())
    }

    fn selected_text_range(&mut self, _ignore_disabled_input: bool, _window: &mut Window, _cx: &mut Context<Self>) -> Option<UTF16Selection> {
        let range = if let Some(r) = self.selection_range() {
            r
        } else {
            self.cursor_offset..self.cursor_offset
        };
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = range {
            let start = r.start.min(self.text.len());
            let end = r.end.min(self.text.len());
            if self.text.is_char_boundary(start) && self.text.is_char_boundary(end) {
                self.text.replace_range(start..end, text);
                self.cursor_offset = start + text.len();
                self.selection_anchor = None;
            }
        } else {
            if self.text.is_char_boundary(self.cursor_offset) {
                self.text.insert_str(self.cursor_offset, text);
                self.cursor_offset += text.len();
                self.selection_anchor = None;
            } else {
                self.cursor_offset = self.text.len();
                self.text.push_str(text);
                self.cursor_offset = self.text.len();
                self.selection_anchor = None;
            }
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, new_text: &str, _new_selected_range: Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(range, new_text, window, cx);
    }

    fn bounds_for_range(&mut self, _range_utf16: Range<usize>, _element_bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(&mut self, _point: Point<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) -> Option<usize> {
        None
    }
}

// ---------------------------------------------------------------------------
// MobieWorkspace – root GPUI view
// ---------------------------------------------------------------------------

pub struct MobieWorkspace {
    focus_handle: FocusHandle,
    chat_scroll_handle: ScrollHandle,
    messages: Vec<ChatMessage>,
    agent_status: AgentStatus,
    cmd_tx: mpsc::Sender<AgentMessage>,

    // App view (Chat / Settings)
    current_view: AppView,

    // Device state
    devices: Vec<String>,
    selected_device: Option<String>,

    // Input fields
    chat_input: Entity<TextInput>,
    settings_api_key: Entity<TextInput>,
    settings_model: Entity<TextInput>,
    settings_base_url: Entity<TextInput>,
}

impl MobieWorkspace {
    pub fn new(
        cx: &mut Context<Self>,
        cmd_tx: mpsc::Sender<AgentMessage>,
        mut update_rx: mpsc::Receiver<AgentUpdate>,
        initial_config: AppConfig,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let chat_scroll_handle = ScrollHandle::new();

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
                                workspace.chat_scroll_handle.scroll_to_bottom();
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

        let chat_input = cx.new(|cx| TextInput::new(cx, "Type a goal for the agent...".into(), String::new()));
        let settings_api_key = cx.new(|cx| {
            let mut input = TextInput::new(cx, "sk-...".into(), initial_config.llm.api_key.clone());
            input.set_masked(true);
            input
        });
        let settings_model = cx.new(|cx| TextInput::new(cx, "gpt-4o".into(), initial_config.llm.model.clone()));
        let settings_base_url = cx.new(|cx| TextInput::new(cx, "https://api.openai.com/v1".into(), initial_config.llm.base_url.clone()));

        Self {
            focus_handle,
            chat_scroll_handle,
            messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "Welcome to Mobie Studio! Type a goal and press Enter.".to_string(),
            }],
            agent_status: AgentStatus::Idle,
            cmd_tx,
            current_view: AppView::Chat,
            devices: vec![],
            selected_device: None,
            chat_input,
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
        let text = self.chat_input.read(cx).text().trim().to_string();
        if text.is_empty() || self.agent_status != AgentStatus::Idle {
            return;
        }
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: text.clone(),
        });
        self.chat_scroll_handle.scroll_to_bottom();
        self.chat_input.update(cx, |input, cx| {
            input.text.clear();
            input.cursor_offset = 0;
            input.selection_anchor = None;
            cx.notify();
        });

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
            api_key: self.settings_api_key.read(cx).text().to_string(),
            model: self.settings_model.read(cx).text().to_string(),
            base_url: self.settings_base_url.read(cx).text().to_string(),
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
        div()
            .flex_1()
            .min_h_0() // Critical for scrolling in flex layouts
            .child(
                div()
                    .id("chat-list")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.chat_scroll_handle)
                    .p(px(16.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .children(self.messages.iter().map(|msg| {
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

                        msg_row.child(
                            div()
                                .w(px(500.0)) // Use fixed width to force wrapping
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
                                        .whitespace_normal()
                                        .child(msg.content.clone()),
                                ),
                        )
                    }))
            )
    }

    fn render_input_area(&self, _window: &Window, cx: &mut Context<Self>) -> Div {
        let is_idle = self.agent_status == AgentStatus::Idle;

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
                    .child(self.chat_input.clone())
                    .when(is_idle && !self.chat_input.read(cx).text().is_empty(), |d| {
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

    fn render_settings_panel(&self, window: &Window, cx: &mut Context<Self>) -> Div {
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
                        self.settings_api_key.clone(),
                        window,
                        cx,
                    ))
                    .child(self.render_settings_field(
                        "Model",
                        self.settings_model.clone(),
                        window,
                        cx,
                    ))
                    .child(self.render_settings_field(
                        "Base URL",
                        self.settings_base_url.clone(),
                        window,
                        cx,
                    ))
                    .child(
                        div()
                            .mt(px(8.0))
                            .p(px(12.0))
                            .bg(rgb(0x1a3a5c))
                            .rounded(px(8.0))
                            .text_xs()
                            .text_color(rgb(0x8899bb))
                            .child("💡 Click a field to focus and edit. Press Enter to save."),
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

    fn render_settings_field(&self, label: &str, input: Entity<TextInput>, _window: &Window, _cx: &mut Context<Self>) -> Div {
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
                    .child(input)
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
            .child(self.render_sidebar(cx))
            .child(match current_view {
                AppView::Chat => div()
                    .flex_1()
                    .h_full()
                    .min_h_0() // Ensure children can scroll
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
                    .child(self.render_input_area(_window, cx)),
                AppView::Settings => self.render_settings_panel(_window, cx),
            })
    }
}
