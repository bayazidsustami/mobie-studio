use gpui::prelude::FluentBuilder;
use gpui::*;
use smallvec::SmallVec;
use std::ops::Range;
use tokio::sync::mpsc;
use tracing::info;

use crate::agent::{AgentMessage, AgentStatus, AgentUpdate};
use crate::config::{save_config, AppConfig};
use crate::device::DeviceStatus;
use crate::llm::LlmConfig;

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(
    mobie,
    [
        SendMessage,
        CancelGoal,
        NavigateSettings,
        NavigateChat,
        RefreshDevices,
        SaveSettings,
        Backspace,
        Delete,
        SelectAll,
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
    ]
);

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
// TextInput View
// ---------------------------------------------------------------------------

use std::cell::RefCell;

pub struct TextInput {
    focus_handle: FocusHandle,
    text: String,
    cursor_offset: usize,            // Character offset
    selection_anchor: Option<usize>, // Character offset
    scroll_offset_y: Pixels,
    placeholder: String,
    is_masked: bool,

    // Caching for performance - using RefCell to allow updating from read() context
    last_wrapped_lines: RefCell<Option<WrappedLinesCache>>,
    last_layout_width: RefCell<Option<Pixels>>,
}

type WrappedLinesCache = (String, Pixels, SmallVec<[WrappedLine; 1]>);

impl TextInput {
    pub fn new(cx: &mut Context<Self>, placeholder: String, initial_value: String) -> Self {
        let char_len = initial_value.chars().count();
        Self {
            focus_handle: cx.focus_handle(),
            text: initial_value,
            cursor_offset: char_len,
            selection_anchor: None,
            scroll_offset_y: px(0.0),
            placeholder,
            is_masked: false,
            last_wrapped_lines: RefCell::new(None),
            last_layout_width: RefCell::new(None),
        }
    }

    pub fn set_masked(&mut self, masked: bool) {
        self.is_masked = masked;
        self.invalidate_cache();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        let anchor = self.selection_anchor?;
        let start = anchor.min(self.cursor_offset);
        let end = anchor.max(self.cursor_offset);

        if start != end {
            return Some(start..end);
        }
        None
    }

    fn byte_offset_for_char_offset(&self, char_offset: usize) -> usize {
        self.text
            .char_indices()
            .map(|(i, _)| i)
            .nth(char_offset)
            .unwrap_or(self.text.len())
    }

    fn char_offset_for_byte_offset(&self, byte_offset: usize) -> usize {
        self.text
            .char_indices()
            .take_while(|(i, _)| *i < byte_offset)
            .count()
    }

    fn invalidate_cache(&self) {
        *self.last_wrapped_lines.borrow_mut() = None;
    }

    fn backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            let start_byte = self.byte_offset_for_char_offset(range.start);
            let end_byte = self.byte_offset_for_char_offset(range.end);
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
        } else if self.cursor_offset > 0 {
            let start_byte = self.byte_offset_for_char_offset(self.cursor_offset - 1);
            let end_byte = self.byte_offset_for_char_offset(self.cursor_offset);
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor_offset -= 1;
        }
        self.invalidate_cache();
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            let start_byte = self.byte_offset_for_char_offset(range.start);
            let end_byte = self.byte_offset_for_char_offset(range.end);
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
        } else if self.cursor_offset < self.text.chars().count() {
            let start_byte = self.byte_offset_for_char_offset(self.cursor_offset);
            let end_byte = self.byte_offset_for_char_offset(self.cursor_offset + 1);
            self.text.replace_range(start_byte..end_byte, "");
        }
        self.invalidate_cache();
        cx.notify();
    }

    fn move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
        }
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
        }
        cx.notify();
    }

    fn move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = None;
        if self.cursor_offset < self.text.chars().count() {
            self.cursor_offset += 1;
        }
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        if self.cursor_offset < self.text.chars().count() {
            self.cursor_offset += 1;
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
        self.cursor_offset = self.text.chars().count();
        cx.notify();
    }

    fn select_end(&mut self, _: &SelectEnd, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_offset);
        }
        self.cursor_offset = self.text.chars().count();
        cx.notify();
    }

    pub fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.selection_anchor = Some(0);
        self.cursor_offset = self.text.chars().count();
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            let start_byte = self.byte_offset_for_char_offset(range.start);
            let end_byte = self.byte_offset_for_char_offset(range.end);
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[start_byte..end_byte].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(range) = self.selection_range() {
            let start_byte = self.byte_offset_for_char_offset(range.start);
            let end_byte = self.byte_offset_for_char_offset(range.end);
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.text[start_byte..end_byte].to_string(),
            ));
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor_offset = range.start;
            self.selection_anchor = None;
            self.invalidate_cache();
            cx.notify();
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                let range = self.selection_range();
                self.replace_text_in_range(range, &text, window, cx);
            }
        }
    }

    fn shape_text(&self, window: &mut Window, width: Pixels) -> SmallVec<[WrappedLine; 1]> {
        let display_text = if self.text.is_empty() {
            self.placeholder.clone()
        } else if self.is_masked {
            "*".repeat(self.text.chars().count())
        } else {
            self.text.clone()
        };

        {
            let cache = self.last_wrapped_lines.borrow();
            if let Some((ref last_text, last_width, ref last_lines)) = *cache {
                // Use a small epsilon for width comparison to avoid cache thrashing
                if last_text == &display_text && (last_width - width).abs() < px(0.1) {
                    return last_lines.clone();
                }
            }
        }

        let text_color = if self.text.is_empty() {
            rgb(0x555566)
        } else {
            rgb(0xeeeeff)
        };

        let font_size = px(14.0);
        let wrapped_lines = window
            .text_system()
            .shape_text(
                display_text.clone().into(),
                font_size,
                &[TextRun {
                    len: display_text.len(),
                    color: text_color.into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                    font: window.text_style().font(),
                }],
                Some(width),
                None,
            )
            .unwrap_or_default();

        *self.last_wrapped_lines.borrow_mut() = Some((display_text, width, wrapped_lines.clone()));
        wrapped_lines
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();

        div()
            .id("text-input")
            .flex()
            .flex_1()
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
        let line_height = window.line_height();
        let height = {
            let this = self.view.read(app);
            // Ensure we have a sane width for the first layout pass
            let available_width = this
                .last_layout_width
                .borrow()
                .filter(|w| *w > px(1.0))
                .unwrap_or_else(|| window.viewport_size().width - px(320.0));

            let lines = this.shape_text(window, available_width);
            let num_lines = lines.len().max(1);
            // Limit to 3 lines height
            line_height * (num_lines.min(3) as f32)
        };

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = height.into();
        style.min_size.height = line_height.into();
        style.overflow.x = Overflow::Hidden;
        style.overflow.y = Overflow::Hidden;

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
        let (
            is_focused,
            cursor_offset,
            selection_range,
            focus_handle,
            is_masked,
            text_len,
            mut scroll_offset_y,
        ) = {
            let state = self.view.read(cx);
            (
                state.focus_handle.is_focused(window),
                state.cursor_offset,
                state.selection_range(),
                state.focus_handle.clone(),
                state.is_masked,
                state.text.chars().count(),
                state.scroll_offset_y,
            )
        };

        let wrapped_lines = {
            let this = self.view.read(cx);
            *this.last_layout_width.borrow_mut() = Some(bounds.size.width);
            this.shape_text(window, bounds.size.width)
        };

        let line_height = window.line_height();

        // Calculate auto-scroll to keep cursor in view
        let cursor_y = if text_len == 0 {
            px(0.0)
        } else {
            let this = self.view.read(cx);
            let target_byte_offset = this.byte_offset_for_char_offset(cursor_offset);
            let mut y = px(0.0);
            let mut start_byte = 0;
            for line in &wrapped_lines {
                let end_byte = start_byte + line.len();
                if target_byte_offset >= start_byte && target_byte_offset <= end_byte {
                    let pos = line
                        .position_for_index(target_byte_offset - start_byte, line_height)
                        .unwrap_or(point(px(0.0), px(0.0)));
                    y += pos.y;
                    break;
                }
                y += line.size(line_height).height;
                start_byte = end_byte;
            }
            y
        };

        // Scroll logic: if cursor is above view, scroll up. If below, scroll down.
        if cursor_y < scroll_offset_y {
            scroll_offset_y = cursor_y;
        } else if cursor_y + line_height > scroll_offset_y + bounds.size.height {
            scroll_offset_y = cursor_y + line_height - bounds.size.height;
        }

        // Clamp scroll offset
        let total_height = wrapped_lines
            .iter()
            .fold(px(0.0), |acc, l| acc + l.size(line_height).height);
        scroll_offset_y = scroll_offset_y
            .max(px(0.0))
            .min((total_height - bounds.size.height).max(px(0.0)));

        // Update the view's scroll offset if it changed
        self.view.update(cx, |this, _| {
            if this.scroll_offset_y != scroll_offset_y {
                this.scroll_offset_y = scroll_offset_y;
            }
        });

        let (state_text, placeholder) = {
            let state = self.view.read(cx);
            (state.text.clone(), state.placeholder.clone())
        };

        let state_text_ref = if state_text.is_empty() {
            placeholder.clone()
        } else if is_masked {
            "*".repeat(text_len)
        } else {
            state_text.clone()
        };

        // Handle mouse clicks to set cursor position
        window.on_mouse_event({
            let view = self.view.clone();
            let focus_handle = focus_handle.clone();
            let wrapped_lines = wrapped_lines.clone();
            let state_text_ref = state_text_ref.clone();
            let placeholder = placeholder.clone();
            let scroll_offset_y = scroll_offset_y;
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    window.focus(&focus_handle);
                    let local_point =
                        event.position - bounds.origin + point(px(0.0), scroll_offset_y);

                    let mut new_offset_chars = 0;
                    let mut current_y = px(0.0);
                    let mut current_line_start_byte = 0;
                    let line_height = window.line_height();

                    // Don't set cursor based on placeholder text
                    if state_text_ref == placeholder {
                        return;
                    }

                    for line in &wrapped_lines {
                        let line_size = line.size(line_height);
                        let line_len_bytes = line.len();
                        if local_point.y >= current_y
                            && local_point.y < current_y + line_size.height
                        {
                            let local_line_point = point(local_point.x, local_point.y - current_y);
                            if let Ok(index_bytes) =
                                line.index_for_position(local_line_point, line_height)
                            {
                                let line_text = &state_text_ref[current_line_start_byte
                                    ..current_line_start_byte + line_len_bytes];
                                let char_offset_in_line =
                                    line_text[..index_bytes.min(line_len_bytes)].chars().count();
                                new_offset_chars += char_offset_in_line;
                                break;
                            }
                        }
                        let line_text = &state_text_ref
                            [current_line_start_byte..current_line_start_byte + line_len_bytes];
                        new_offset_chars += line_text.chars().count();
                        current_y += line_size.height;
                        current_line_start_byte += line_len_bytes;
                    }

                    view.update(cx, |this, cx| {
                        let actual_index = new_offset_chars;

                        if event.modifiers.shift {
                            if this.selection_anchor.is_none() {
                                this.selection_anchor = Some(this.cursor_offset);
                            }
                        } else {
                            this.selection_anchor = None;
                        }
                        if this.cursor_offset != actual_index {
                            this.cursor_offset = actual_index;
                            cx.notify();
                        }
                    });
                }
            }
        });

        // Map mouse drag to selection
        window.on_mouse_event({
            let view = self.view.clone();
            let wrapped_lines = wrapped_lines.clone();
            let state_text_ref = state_text_ref.clone();
            let placeholder = placeholder.clone();
            let scroll_offset_y = scroll_offset_y;
            move |event: &MouseMoveEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.pressed_button == Some(MouseButton::Left)
                    && bounds.contains(&event.position)
                {
                    let local_point =
                        event.position - bounds.origin + point(px(0.0), scroll_offset_y);
                    let mut new_offset_chars = 0;
                    let mut current_y = px(0.0);
                    let mut current_line_start_byte = 0;
                    let line_height = _window.line_height();

                    // Don't set selection based on placeholder text
                    if state_text_ref == placeholder {
                        return;
                    }

                    for line in &wrapped_lines {
                        let line_size = line.size(line_height);
                        let line_len_bytes = line.len();
                        if local_point.y >= current_y
                            && local_point.y < current_y + line_size.height
                        {
                            let local_line_point = point(local_point.x, local_point.y - current_y);
                            if let Ok(index_bytes) =
                                line.index_for_position(local_line_point, line_height)
                            {
                                let line_text = &state_text_ref[current_line_start_byte
                                    ..current_line_start_byte + line_len_bytes];
                                let char_offset_in_line =
                                    line_text[..index_bytes.min(line_len_bytes)].chars().count();
                                new_offset_chars += char_offset_in_line;
                                break;
                            }
                        }
                        let line_text = &state_text_ref
                            [current_line_start_byte..current_line_start_byte + line_len_bytes];
                        new_offset_chars += line_text.chars().count();
                        current_y += line_size.height;
                        current_line_start_byte += line_len_bytes;
                    }

                    view.update(cx, |this, cx| {
                        let actual_index = new_offset_chars;

                        if this.selection_anchor.is_none() {
                            this.selection_anchor = Some(this.cursor_offset);
                        }
                        if this.cursor_offset != actual_index {
                            this.cursor_offset = actual_index;
                            cx.notify();
                        }
                    });
                }
            }
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            // Paint selection highlight
            if is_focused && !state_text.is_empty() {
                if let Some(ref char_range) = selection_range {
                    // Convert character range to byte range for the painting loop
                    let this = self.view.read(cx);
                    let range = this.byte_offset_for_char_offset(char_range.start)
                        ..this.byte_offset_for_char_offset(char_range.end);

                    let mut current_y = bounds.origin.y - scroll_offset_y;
                    let mut line_start_byte_offset = 0;

                    for line in &wrapped_lines {
                        let line_len_bytes = line.len();
                        let line_end_byte_offset = line_start_byte_offset + line_len_bytes;
                        let line_size = line.size(line_height);

                        // Calculate intersection in byte space
                        let sel_start_byte = range
                            .start
                            .max(line_start_byte_offset)
                            .min(line_end_byte_offset);
                        let sel_start_byte = sel_start_byte.min(
                            range
                                .end
                                .max(line_start_byte_offset)
                                .min(line_end_byte_offset),
                        );
                        let sel_end_byte = range
                            .end
                            .max(line_start_byte_offset)
                            .min(line_end_byte_offset);

                        if sel_start_byte < sel_end_byte {
                            let relative_start_byte = sel_start_byte - line_start_byte_offset;
                            let relative_end_byte = sel_end_byte - line_start_byte_offset;

                            let start_pos = line
                                .position_for_index(relative_start_byte, line_height)
                                .unwrap_or(point(px(0.0), px(0.0)));
                            let end_pos = line
                                .position_for_index(relative_end_byte, line_height)
                                .unwrap_or(point(line_size.width, px(0.0)));

                            let start_row = (start_pos.y / line_height).round() as i32;
                            let end_row = (end_pos.y / line_height).round() as i32;

                            for row in start_row..=end_row {
                                let row_y = line_height * row as f32;
                                let row_start_x = if row == start_row {
                                    start_pos.x
                                } else {
                                    px(0.0)
                                };

                                let row_end_x = if row == end_row {
                                    end_pos.x
                                } else {
                                    line_size.width
                                };

                                if row_start_x < row_end_x {
                                    window.paint_quad(fill(
                                        Bounds {
                                            origin: point(
                                                bounds.origin.x + row_start_x,
                                                current_y + row_y,
                                            ),
                                            size: size(row_end_x - row_start_x, line_height),
                                        },
                                        rgba(0x4488cc88),
                                    ));
                                }
                            }
                        }
                        line_start_byte_offset = line_end_byte_offset;
                        current_y += line_size.height;
                    }
                }
            }

            // Paint wrapped lines
            let mut current_origin = bounds.origin - point(px(0.0), scroll_offset_y);
            for line in &wrapped_lines {
                let line_size = line.size(line_height);
                let _ = line.paint(
                    current_origin,
                    line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
                current_origin.y += line_size.height;
            }

            // Render cursor
            if is_focused && selection_range.is_none() {
                let mut current_y = bounds.origin.y - scroll_offset_y;
                let mut line_start_byte_offset = 0;
                let mut cursor_pos = None;

                if state_text.is_empty() {
                    cursor_pos = Some(bounds.origin);
                } else {
                    let target_byte_offset = self
                        .view
                        .read(cx)
                        .byte_offset_for_char_offset(cursor_offset);
                    for line in &wrapped_lines {
                        let line_len_bytes = line.len();
                        let line_end_byte_offset = line_start_byte_offset + line_len_bytes;
                        let line_size = line.size(line_height);

                        if target_byte_offset >= line_start_byte_offset
                            && target_byte_offset <= line_end_byte_offset
                        {
                            let relative_byte = target_byte_offset - line_start_byte_offset;
                            let pos = line
                                .position_for_index(relative_byte, line_height)
                                .unwrap_or(point(px(0.0), px(0.0)));
                            cursor_pos = Some(point(bounds.origin.x + pos.x, current_y + pos.y));
                            break;
                        }
                        line_start_byte_offset = line_end_byte_offset;
                        current_y += line_size.height;
                    }
                }

                if let Some(pos) = cursor_pos {
                    window.paint_quad(fill(
                        Bounds {
                            origin: pos,
                            size: size(px(2.0), line_height),
                        },
                        rgb(0xe94560),
                    ));
                }
            }
        });

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.view.clone()),
            cx,
        );
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text_utf16: Vec<u16> = self.text.encode_utf16().collect();
        let start = range.start.min(text_utf16.len());
        let end = range.end.min(text_utf16.len());
        String::from_utf16(&text_utf16[start..end]).ok()
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = if let Some(r) = self.selection_range() {
            self.byte_offset_for_char_offset(r.start)..self.byte_offset_for_char_offset(r.end)
        } else {
            let offset = self.byte_offset_for_char_offset(self.cursor_offset);
            offset..offset
        };

        let start_utf16 = self.text[..range.start].encode_utf16().count();
        let end_utf16 = self.text[..range.end].encode_utf16().count();

        Some(UTF16Selection {
            range: start_utf16..end_utf16,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let char_range = range.map(|r| {
            let text_utf16: Vec<u16> = self.text.encode_utf16().collect();
            let start_byte = String::from_utf16(&text_utf16[..r.start.min(text_utf16.len())])
                .map(|s| s.len())
                .unwrap_or(0);
            let end_byte = String::from_utf16(&text_utf16[..r.end.min(text_utf16.len())])
                .map(|s| s.len())
                .unwrap_or(0);

            let start_char = self.char_offset_for_byte_offset(start_byte);
            let end_char = self.char_offset_for_byte_offset(end_byte);
            start_char..end_char
        });

        if let Some(r) = char_range {
            let start_byte = self.byte_offset_for_char_offset(r.start);
            let end_byte = self.byte_offset_for_char_offset(r.end);
            if self.text.is_char_boundary(start_byte) && self.text.is_char_boundary(end_byte) {
                self.text.replace_range(start_byte..end_byte, text);
                self.cursor_offset = r.start + text.chars().count();
                self.selection_anchor = None;
            }
        } else {
            let byte_offset = self.byte_offset_for_char_offset(self.cursor_offset);
            if self.text.is_char_boundary(byte_offset) {
                self.text.insert_str(byte_offset, text);
                self.cursor_offset += text.chars().count();
                self.selection_anchor = None;
            } else {
                self.text.push_str(text);
                self.cursor_offset = self.text.chars().count();
                self.selection_anchor = None;
            }
        }
        self.invalidate_cache();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

// ---------------------------------------------------------------------------
// Main Workspace View
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone)]
enum AppView {
    Chat,
    Settings,
    History,
}

#[derive(Debug, PartialEq, Clone)]
enum NavTabAction {
    Chat,
    Settings,
    History,
}

pub struct MobieWorkspace {
    focus_handle: FocusHandle,
    chat_scroll_handle: ScrollHandle,
    messages: Vec<ChatMessage>,
    agent_status: AgentStatus,
    cmd_tx: mpsc::Sender<AgentMessage>,
    current_view: AppView,
    devices: Vec<(String, DeviceStatus)>,
    selected_device: Option<String>,
    latest_test: Option<std::path::PathBuf>,

    // History
    sessions: Vec<crate::db::Session>,
    selected_session: Option<crate::db::Session>,
    selected_test_case: Option<crate::yaml_exporter::TestCase>,
    
    // Image Preview
    preview_image_path: Option<String>,
    preview_zoom: f32,

    // Inputs
    chat_input: Entity<TextInput>,
    settings_api_key: Entity<TextInput>,
    settings_model: Entity<TextInput>,
    settings_base_url: Entity<TextInput>,
}

impl MobieWorkspace {
    pub fn new(
        cx: &mut Context<Self>,
        initial_config: AppConfig,
        cmd_tx: mpsc::Sender<AgentMessage>,
        mut update_rx: mpsc::Receiver<AgentUpdate>,
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
                                let count = devs.len();
                                workspace.devices = devs;
                                // Auto-select first online device if none selected
                                if workspace.selected_device.is_none() {
                                    workspace.selected_device = workspace
                                        .devices
                                        .iter()
                                        .find(|(_, s)| *s == DeviceStatus::Online)
                                        .map(|(id, _)| id.clone());
                                }
                                info!("Found {} device(s)/AVDs.", count);
                            }
                            AgentUpdate::TestGenerated(path) => {
                                workspace.latest_test = Some(path.clone());
                                workspace.messages.push(ChatMessage {
                                    role: ChatRole::System,
                                    content: format!(
                                        "💾 YAML test case saved: {}",
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    ),
                                });
                                workspace.chat_scroll_handle.scroll_to_bottom();
                            }
                            AgentUpdate::SessionSaved => {
                                let db_path = crate::config::db_path();
                                if let Ok(mgr) = crate::db::SessionManager::new(db_path) {
                                    if let Ok(sessions) = mgr.get_all_sessions() {
                                        workspace.sessions = sessions;
                                    }
                                }
                            }
                        }
                        cx.notify();
                    })
                });
            }
        })
        .detach();

        let chat_input =
            cx.new(|cx| TextInput::new(cx, "Type a goal for the agent...".into(), String::new()));
        let settings_api_key = cx.new(|cx| {
            let mut input = TextInput::new(cx, "sk-...".into(), initial_config.llm.api_key.clone());
            input.set_masked(true);
            input
        });
        let settings_model =
            cx.new(|cx| TextInput::new(cx, "gpt-4o".into(), initial_config.llm.model.clone()));
        let settings_base_url = cx.new(|cx| {
            TextInput::new(
                cx,
                "https://api.openai.com/v1".into(),
                initial_config.llm.base_url.clone(),
            )
        });

        // Load initial sessions
        let mut sessions = vec![];
        let db_path = crate::config::db_path();
        if let Ok(mgr) = crate::db::SessionManager::new(db_path) {
            if let Ok(s) = mgr.get_all_sessions() {
                sessions = s;
            }
        }

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
            latest_test: None,
            sessions,
            selected_session: None,
            selected_test_case: None,
            preview_image_path: None,
            preview_zoom: 1.0,
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
            input.invalidate_cache();
            cx.notify();
        });

        let lower_text = text.to_lowercase();
        if lower_text == "/retest" || lower_text == "retest the latest scenario" || lower_text == "run again" {
            if let Some(path) = self.latest_test.clone() {
                let tx = self.cmd_tx.clone();
                cx.spawn(async move |_, _| {
                    let _ = tx.send(AgentMessage::RetestScenario(path)).await;
                })
                .detach();
            } else {
                self.messages.push(ChatMessage {
                    role: ChatRole::System,
                    content: "❌ No previous test scenario available to retest.".to_string(),
                });
                self.chat_scroll_handle.scroll_to_bottom();
            }
        } else {
            let tx = self.cmd_tx.clone();
            cx.spawn(async move |_, _| {
                let _ = tx.send(AgentMessage::StartGoal(text, true)).await;
            })
            .detach();
        }

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

    fn navigate_history(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current_view = AppView::History;
        
        // Fetch sessions from DB
        let db_path = crate::config::db_path();
        if let Ok(mgr) = crate::db::SessionManager::new(db_path) {
            if let Ok(sessions) = mgr.get_all_sessions() {
                self.sessions = sessions;
            }
        }
        cx.notify();
    }

    fn navigate_chat(&mut self, _: &NavigateChat, _window: &mut Window, cx: &mut Context<Self>) {
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

        let tx = self.cmd_tx.clone();
        cx.spawn(async move |_, _| {
            let _ = tx.send(AgentMessage::RefreshDevices).await;
        })
        .detach();

        cx.notify();
    }

    fn save_settings(&mut self, _: &SaveSettings, _window: &mut Window, cx: &mut Context<Self>) {
        let new_llm = LlmConfig {
            api_key: self.settings_api_key.read(cx).text().to_string(),
            model: self.settings_model.read(cx).text().to_string(),
            base_url: self.settings_base_url.read(cx).text().to_string(),
            provider: "openai".to_string(),
        };

        // Persist to disk
        let cfg = AppConfig {
            version: 1,
            llm: new_llm.clone(),
        };
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
            // Sessions Section
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x666688))
                            .child("SESSIONS"),
                    )
                    .child(
                        div()
                            .id("sidebar-sessions")
                            .flex_1()
                            .overflow_y_scroll()
                            .children(self.sessions.iter().map(|session| {
                                let is_selected = self.selected_session.as_ref().map(|s| &s.id) == Some(&session.id);
                                let session_clone = session.clone();
                                
                                div()
                                    .p(px(8.0))
                                    .mb(px(4.0))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(is_selected, |s| s.bg(rgba(0x4488cc22)))
                                    .hover(|s| s.bg(rgba(0x4488cc11)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.selected_session = Some(session_clone.clone());
                                            this.selected_test_case = None;
                                            
                                            if let Some(yaml_path) = &session_clone.yaml_path {
                                                if let Ok(yaml) = std::fs::read_to_string(yaml_path) {
                                                    if let Ok(mut tc) = serde_yaml::from_str::<crate::yaml_exporter::TestCase>(&yaml) {
                                                        // Load screenshots from disk if they exist
                                                        let p = std::path::PathBuf::from(yaml_path);
                                                        if let Some(stem) = p.file_stem() {
                                                            if let Some(parent) = p.parent() {
                                                                let screenshots_dir = parent.join("screenshots").join(stem);
                                                                for (i, step) in tc.steps.iter_mut().enumerate() {
                                                                    let name = format!("step_{:02}_{}.png", i + 1, crate::yaml_exporter::slugify(&step.action));
                                                                    let screenshot_path = screenshots_dir.join(name);
                                                                    if screenshot_path.exists() {
                                                                        step.screenshot = std::fs::read(screenshot_path).ok();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        this.selected_test_case = Some(tc);
                                                    }
                                                }
                                            }
                                            
                                            this.current_view = AppView::History;
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x888899))
                                                    .child(session.timestamp.format("%m-%d %H:%M").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(if is_selected { rgb(0xeeeeff) } else { rgb(0xccccdd) })
                                                    .overflow_hidden()
                                                    .child(session.goal.clone()),
                                            ),
                                    )
                            })),
                    )
            )
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
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.cancel_goal(&CancelGoal, window, cx);
                            }),
                        )
                        .child("■ Cancel Goal"),
                )
            })
    }

    fn render_nav_tabs(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .gap(px(4.0))
            .child(self.render_nav_tab(
                "💬 Chat",
                self.current_view == AppView::Chat,
                cx,
                NavTabAction::Chat,
            ))
            .child(self.render_nav_tab(
                "⚙ Settings",
                self.current_view == AppView::Settings,
                cx,
                NavTabAction::Settings,
            ))
    }

    fn render_nav_tab(
        &self,
        label: &str,
        active: bool,
        cx: &mut Context<Self>,
        action: NavTabAction,
    ) -> Div {
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
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| match action {
                    NavTabAction::Chat => this.navigate_chat(&NavigateChat, window, cx),
                    NavTabAction::Settings => this.navigate_settings(&NavigateSettings, window, cx),
                    NavTabAction::History => this.navigate_history(window, cx),
                }),
            )
            .child(label.to_string())
    }

    fn render_device_section(&self, cx: &mut Context<Self>) -> Div {
        let mut section = div().flex().flex_col().gap(px(8.0)).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb_2()
                .border_b_1()
                .border_color(rgba(0x2a2a4a88))
                .mb_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x666688))
                        .child("DEVICES"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x4488cc))
                        .cursor_pointer()
                        .hover(|s| s.text_color(rgb(0x66aaff)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.refresh_devices(&RefreshDevices, window, cx);
                            }),
                        )
                        .child("↺ Refresh"),
                ),
        );

        if self.devices.is_empty() {
            section = section.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .py_2()
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(4.0))
                            .bg(rgb(0xff4444)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x888899))
                            .child("No devices"),
                    ),
            );
        } else {
            let cmd_tx = self.cmd_tx.clone();
            for (dev, status) in &self.devices {
                let is_selected = self.selected_device.as_deref() == Some(dev.as_str());

                let (dot_color, dot_border) = match status {
                    DeviceStatus::Online => (rgb(0x44ff88), rgb(0x228844)),
                    DeviceStatus::Launching => (rgb(0xffcc44), rgb(0x886622)),
                    DeviceStatus::Offline => (rgb(0x555566), rgb(0x333344)),
                };

                let text_color = if is_selected {
                    rgb(0xeeeeff)
                } else {
                    rgb(0x888899)
                };

                let dev_id = dev.clone();
                let status_val = *status;
                let tx = cmd_tx.clone();

                section = section.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(4.0))
                        .py_2()
                        .px_1()
                        .rounded(px(6.0))
                        .when(is_selected, |s| s.bg(rgba(0x4488cc22)))
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w_0()
                                .items_center()
                                .gap(px(8.0))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        let dev_id = dev_id.clone();
                                        let tx = tx.clone();
                                        move |this, _, _window, cx| {
                                            if status_val == DeviceStatus::Online {
                                                this.selected_device = Some(dev_id.clone());
                                                let tx2 = tx.clone();
                                                let id = dev_id.clone();
                                                cx.spawn(async move |_, _| {
                                                    let _ = tx2
                                                        .send(AgentMessage::SelectDevice(id))
                                                        .await;
                                                })
                                                .detach();
                                                cx.notify();
                                            }
                                        }
                                    }),
                                )
                                .child(
                                    div()
                                        .w(px(10.0))
                                        .h(px(10.0))
                                        .rounded(px(5.0))
                                        .bg(dot_color)
                                        .border_1()
                                        .border_color(dot_border)
                                        .flex_shrink_0(),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .text_sm()
                                        .font_weight(if is_selected {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(text_color)
                                        .child(dev.clone()),
                                ),
                        )
                        .child(
                            div().flex_shrink_0().child(match status {
                                DeviceStatus::Offline => {
                                    let name = dev.clone();
                                    let tx = tx.clone();
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded(px(4.0))
                                        .bg(rgba(0x44ff8811))
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x44ff88))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgba(0x44ff8822)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |_, _, _, cx| {
                                                let tx2 = tx.clone();
                                                let n = name.clone();
                                                cx.background_executor()
                                                    .spawn(async move {
                                                        let _ = tx2
                                                            .send(AgentMessage::LaunchEmulator(n))
                                                            .await;
                                                    })
                                                    .detach();
                                            }),
                                        )
                                        .child("▶ Start")
                                }
                                DeviceStatus::Online => {
                                    let id = dev.clone();
                                    let tx = tx.clone();
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded(px(4.0))
                                        .bg(rgba(0xff444411))
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xff4444))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgba(0xff444422)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |_, _, _, cx| {
                                                let tx2 = tx.clone();
                                                let i = id.clone();
                                                cx.background_executor()
                                                    .spawn(async move {
                                                        let _ = tx2
                                                            .send(AgentMessage::StopEmulator(i))
                                                            .await;
                                                    })
                                                    .detach();
                                            }),
                                        )
                                        .child("■ Stop")
                                }
                                DeviceStatus::Launching => div()
                                    .px_2()
                                    .py_1()
                                    .text_xs()
                                    .text_color(rgb(0x888899))
                                    .child("..."),
                            }),
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
                    .gap(px(16.0))
                    .children(self.messages.iter().map(|msg| {
                        let is_user = matches!(msg.role, ChatRole::User);
                        let (bg, text_col) = match msg.role {
                            ChatRole::User => (rgb(0x16213e), rgb(0xeeeeff)),
                            ChatRole::Agent => (rgb(0x0f3460), rgb(0xccddff)),
                            ChatRole::System => (rgb(0x2a2a4a), rgb(0x888899)),
                        };

                        let label = match msg.role {
                            ChatRole::User => "You",
                            ChatRole::Agent => "Agent",
                            ChatRole::System => "System",
                        };

                        div()
                            .flex()
                            .flex_col()
                            .w_full()
                            .flex_shrink_0()
                            .items_end()
                            .when(!is_user, |d| d.items_start())
                            .child(
                                div()
                                    .max_w(px(600.0))
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .bg(bg)
                                    .rounded(px(12.0))
                                    .p(px(12.0))
                                    .gap(px(4.0))
                                    .text_sm()
                                    .text_color(text_col)
                                    .whitespace_normal()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0x666688))
                                            .child(label.to_string()),
                                    )
                                    .child(msg.content.clone()),
                            )
                    })),
            )
    }

    fn render_input_area(&self, _window: &Window, cx: &mut Context<Self>) -> Div {
        let is_idle = self.agent_status == AgentStatus::Idle;
        let has_text = !self.chat_input.read(cx).text().is_empty();
        let can_send = is_idle && has_text;

        div()
            .border_t_1()
            .border_color(rgb(0x2a2a4a))
            .p(px(16.0))
            .w_full()
            .flex()
            .items_end()
            .gap(px(12.0))
            .child(
                div()
                    .flex_1()
                    .bg(rgb(0x16213e))
                    .rounded(px(12.0))
                    .p(px(14.0))
                    .child(self.chat_input.clone()),
            )
            .child(
                div()
                    .cursor_pointer()
                    .bg(if can_send {
                        rgb(0xe94560)
                    } else {
                        rgb(0x2a2a4a)
                    })
                    .hover(|s| {
                        if can_send {
                            s.bg(rgb(0xff5c77))
                        } else {
                            s
                        }
                    })
                    .rounded(px(12.0))
                    .py(px(14.0))
                    .px(px(20.0))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if can_send {
                        rgb(0xffffff)
                    } else {
                        rgb(0x888899)
                    })
                    .child("Send")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            this.send_message(&SendMessage, window, cx);
                        }),
                    ),
            )
    }

    // -----------------------------------------------------------------------
    // History view
    // -----------------------------------------------------------------------

    fn render_history_panel(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex_1()
            .min_w_0() // Allow container to shrink
            .h_full()
            .bg(rgb(0x0a0a1a))
            .child(match &self.selected_session {
                Some(session) => self.render_session_detail(session, cx),
                None => div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(rgb(0x555566))
                            .child("Select a session from the sidebar to view details"),
                    ),
            })
    }

    fn render_session_detail(&self, session: &crate::db::Session, cx: &mut Context<Self>) -> Div {
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_w_0() // Allow container to shrink
            .child(
                div()
                    .p(px(20.0))
                    .border_b_1()
                    .border_color(rgb(0x2a2a4a))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .min_w_0()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x666688))
                            .child(format!("SESSION ID: {}", session.id)),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xeeeeff))
                            .overflow_hidden()
                            .child(session.goal.clone()),
                    ),
            )
            .child(
                div()
                    .id("session-detail-scroll")
                    .flex_1()
                    .p(px(20.0))
                    .overflow_y_scroll()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(16.0))
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x888899))
                                    .child("METADATA"),
                            )
                            .child(
                                div()
                                    .bg(rgb(0x16213e))
                                    .rounded(px(8.0))
                                    .p(px(12.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .min_w_0()
                                    .child(self.render_detail_item("Timestamp", &session.timestamp.to_rfc3339()))
                                    .child(self.render_detail_item("Status", &session.status))
                                    .child(self.render_detail_item("YAML Path", session.yaml_path.as_deref().unwrap_or("None"))),
                            )
                            .child(
                                div()
                                    .mt(px(20.0))
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x888899))
                                    .child("RESULTS"),
                            )
                            .child(
                                div()
                                    .bg(rgb(0x16213e))
                                    .rounded(px(8.0))
                                    .p(px(12.0))
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(0xccccdd))
                                            .overflow_hidden()
                                            .child(match &session.yaml_path {
                                                Some(path) => format!("✅ Test case generated and saved to: {}", path),
                                                None => "No test case generated for this session.".to_string(),
                                            })
                                    )
                            )
                            .when_some(self.selected_test_case.as_ref(), |this, tc| {
                                this.child(
                                    div()
                                        .mt(px(20.0))
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0x888899))
                                        .child("EXECUTION TIMELINE"),
                                )
                                .child(self.render_execution_timeline(tc, session.yaml_path.as_deref(), cx))
                            }),
                    ),
            )
    }

    fn render_execution_timeline(&self, tc: &crate::yaml_exporter::TestCase, yaml_path: Option<&str>, cx: &mut Context<Self>) -> Div {
        // Construct screenshots directory path if available
        let screenshots_dir = yaml_path.and_then(|p| {
            let p = std::path::PathBuf::from(p);
            let stem = p.file_stem()?.to_string_lossy();
            let parent = p.parent()?;
            Some(parent.join("screenshots").join(stem.to_string()))
        });

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(tc.steps.iter().enumerate().map(|(i, step)| {
                let screenshot_path = screenshots_dir.as_ref().and_then(|dir| {
                    let name = format!("step_{:02}_{}.png", i + 1, crate::yaml_exporter::slugify(&step.action));
                    let path = dir.join(name);
                    if path.exists() {
                        Some(format!("file://{}", path.to_string_lossy()))
                    } else {
                        None
                    }
                });

                div()
                    .bg(rgb(0x16213e))
                    .rounded(px(8.0))
                    .p(px(12.0))
                    .flex()
                    .gap(px(16.0))
                    .child(
                        // Left: Step Metadata
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .bg(rgb(0xe94560))
                                            .text_color(rgb(0xffffff))
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .px(px(6.0))
                                            .py(px(2.0))
                                            .rounded(px(4.0))
                                            .child(format!("{}", i + 1)),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xeeeeff))
                                            .child(step.action.to_uppercase()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888899))
                                    .child(format!("{:?}", step.params)),
                            )
                            .child(
                                div()
                                    .mt(px(4.0))
                                    .text_sm()
                                    .italic()
                                    .text_color(rgb(0xaaaabb))
                                    .child(step.reasoning.clone()),
                            ),
                    )
                    .when_some(screenshot_path, |this, path| {
                        let preview_path = path.clone();
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .w(px(140.0))
                                        .h(px(240.0))
                                        .flex_shrink_0()
                                        .bg(rgb(0x000000))
                                        .rounded(px(6.0))
                                        .overflow_hidden()
                                        .child(
                                            img(path)
                                                .size_full()
                                        )
                                )
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .bg(rgb(0xe94560))
                                        .hover(|s| s.bg(rgb(0xff5c77)))
                                        .text_color(rgb(0xffffff))
                                        .text_xs()
                                        .font_weight(FontWeight::BOLD)
                                        .py(px(6.0))
                                        .rounded(px(6.0))
                                        .flex()
                                        .justify_center()
                                        .child("SEE FULL")
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.preview_image_path = Some(preview_path.clone());
                                                this.preview_zoom = 1.0;
                                                cx.notify();
                                            }),
                                        )
                                )
                        )
                    })
            }))
    }

    fn render_detail_item(&self, label: &str, value: &str) -> Div {
        div()
            .flex()
            .gap(px(12.0))
            .min_w_0()
            .child(
                div()
                    .w(px(100.0))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x666688))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(rgb(0xaaaabb))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(value.to_string()),
            )
    }

    // -----------------------------------------------------------------------
    // Image Preview Overlay
    // -----------------------------------------------------------------------

    fn render_image_preview(&self, cx: &mut Context<Self>) -> Div {
        let path = match &self.preview_image_path {
            Some(p) => p.clone(),
            None => return div(),
        };

        div()
            .absolute()
            .size_full()
            .bg(rgba(0x000000dd))
            .flex()
            .flex_col()
            .child(
                // Toolbar
                div()
                    .p(px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .cursor_pointer()
                                    .bg(rgb(0x2a2a4a))
                                    .text_color(rgb(0xffffff))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .child("Zoom -")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.preview_zoom = (this.preview_zoom - 0.2).max(0.2);
                                            cx.notify();
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaabb))
                                    .child(format!("{:.0}%", self.preview_zoom * 100.0)),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .bg(rgb(0x2a2a4a))
                                    .text_color(rgb(0xffffff))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(6.0))
                                    .child("Zoom +")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.preview_zoom = (this.preview_zoom + 0.2).min(5.0);
                                            cx.notify();
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .bg(rgb(0xe94560))
                            .text_color(rgb(0xffffff))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Close")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.preview_image_path = None;
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .child(
                // Image Area
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id("image-preview-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .overflow_x_scroll()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img(path)
                                    .w(px(600.0 * self.preview_zoom))
                            )
                    )
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.save_settings(&SaveSettings, window, cx);
                                }),
                            )
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.navigate_chat(&NavigateChat, window, cx);
                                }),
                            )
                            .child("Cancel"),
                    ),
            )
    }

    fn render_settings_field(
        &self,
        label: &str,
        input: Entity<TextInput>,
        _window: &Window,
        _cx: &mut Context<Self>,
    ) -> Div {
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
                    .child(input),
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
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, window, cx| {
                                            this.navigate_settings(&NavigateSettings, window, cx);
                                        }),
                                    )
                                    .child("⚙ Settings"),
                            ),
                    )
                    .child(self.render_chat_area())
                    .child(self.render_input_area(_window, cx)),
                AppView::Settings => self.render_settings_panel(_window, cx),
                AppView::History => self.render_history_panel(cx),
            })
            .when(self.preview_image_path.is_some(), |this| {
                this.child(self.render_image_preview(cx))
            })
    }
}
