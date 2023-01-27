use egui::panel::PanelState;
use egui::{pos2, vec2, CursorIcon, Id, Rect, Sense, TextBuffer, Vec2};

use crate::config::Config;

use super::titlebar::TITLEBAR_HEIGHT;

// A read only string for multiline textedit
struct ReadOnlyString<'a> {
    content: &'a str,
}

impl<'a> TextBuffer for ReadOnlyString<'a> {
    fn is_mutable(&self) -> bool {
        false
    }

    fn as_str(&self) -> &str {
        self.content
    }

    fn insert_text(&mut self, _: &str, _: usize) -> usize {
        0
    }

    fn delete_char_range(&mut self, _: std::ops::Range<usize>) {}

    fn clear(&mut self) {}

    fn replace(&mut self, _: &str) {}
}

impl<'a> ReadOnlyString<'a> {
    fn new(content: &'a str) -> Self {
        Self { content }
    }
}

pub struct Terminal;

impl Terminal {
    pub fn show(ctx: &egui::Context, config: &mut Config) {
        let id = Id::new("terminal");

        if config.terminal.opened_from_close {
            // we need to reset the panel state position to be where the mouse pointer is to make it seamless
            // on open, so it doesn't flash when opening by opening big then resetting to where the mouse is
            let coords = ctx.pointer_latest_pos().unwrap_or_default();
            let window_rect = ctx.available_rect();
            let rect = Rect::from_two_pos(
                pos2(0.0, coords.y),
                pos2(window_rect.right(), window_rect.bottom()),
            );

            ctx.data().insert_persisted(id, PanelState { rect });
        }

        egui::TopBottomPanel::bottom(id)
            .resizable(true)
            .default_height(0.0)
            .min_height(0.0)
            .max_height(ctx.available_rect().height() - (TITLEBAR_HEIGHT as f32 / 2.0))
            .show_separator_line(false)
            .show(ctx, |ui| {
                //
                // Panel handling code
                //

                let mut close_rect = ctx.available_rect();

                let close_threshold = if config.terminal.opened_from_close_dragging {
                    16.0
                } else {
                    20.0
                };

                close_rect.set_top(close_rect.bottom() - close_threshold);

                let pointer_pos = ctx.pointer_latest_pos().unwrap_or_default();

                let window_close_bottom = ctx.available_rect().bottom() - close_threshold;

                let resize_id = id.with("__resize");

                // when mouse is outside of window, as long as we were dragging, pointer_pos is still Some()
                // we can utilize this to allow resizing AS LONG AS mouse isn't below the window in screen coords
                if (close_rect.contains(pointer_pos) || pointer_pos.y >= window_close_bottom)
                    && ctx.memory().is_being_dragged(resize_id)
                {
                    config.terminal.open = false;
                    config.terminal.closed_from_open = true;
                }

                if config.terminal.opened_from_close {
                    let mut memory = ui.memory();
                    memory.set_dragged_id(resize_id);

                    config.terminal.opened_from_close = false;
                }

                if config.terminal.opened_from_close_dragging
                    && !ui.memory().is_being_dragged(resize_id)
                {
                    config.terminal.opened_from_close_dragging = false;
                }

                //
                // Scrollbar and panel contents
                //

                let mut frame_rect = ui.max_rect();
                frame_rect.set_left(frame_rect.left() + 2.0);
                frame_rect.set_right(frame_rect.right() - 2.0);
                frame_rect.set_bottom(frame_rect.bottom() - 10.0);
                frame_rect.set_top(frame_rect.top() + 10.0);

                let active_tab = config.terminal.active_tab.unwrap();
                let offset = *config
                    .terminal
                    .scroll_offset
                    .get_mut(&active_tab)
                    .unwrap_or(&mut Vec2::default());

                let terminal_output = config.terminal.content.entry(active_tab).or_default();
                let stream = config.terminal.streamable.get_mut(&active_tab);
                if let Some((rx, _)) = stream {
                    if let Ok(output) = rx.try_recv() {
                        terminal_output.push_str(&output);
                        terminal_output.push('\n');

                        // as long as there's something more in the queue, keep requesting repaints
                        ctx.request_repaint();
                    }
                }

                let mut read_only_term = ReadOnlyString::new(terminal_output);

                let text_widget = egui::TextEdit::multiline(&mut read_only_term)
                    .font(egui::TextStyle::Monospace) // for cursor height
                    // remove the frame and draw our own
                    .frame(false)
                    .desired_width(f32::INFINITY)
                    //.layouter(&mut layouter)
                    .id(id.with("term_output"))
                    .interactive(true);

                let scrollarea = egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .auto_shrink([false, false])
                    .scroll_offset(offset)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add(text_widget);
                    });

                config
                    .terminal
                    .scroll_offset
                    .insert(active_tab, scrollarea.state.offset);
            });
    }

    pub fn show_closed_handle(ctx: &egui::Context, config: &mut Config) {
        let id = Id::new("terminal-closed");

        egui::TopBottomPanel::bottom(id)
            .resizable(false)
            .default_height(13.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        let center_id = id.with("center_line");

                        let sense = Sense::click_and_drag();
                        let hover_sense = Sense::hover();

                        let (alloc_id, center_line) = ui.allocate_space(vec2(75.0, 2.0));
                        let response = ui.interact(center_line, alloc_id, sense);
                        let h_response =
                            ui.interact(center_line, center_id.with("hover"), hover_sense);

                        if config.terminal.closed_from_open {
                            ui.memory().set_dragged_id(alloc_id);
                            config.terminal.closed_from_open = false;
                        }

                        let is_dragging = response.dragged();

                        if is_dragging || h_response.hovered() {
                            ui.output().cursor_icon = CursorIcon::ResizeVertical;
                        }

                        // we need to subtract the closing threshold from the bottom
                        let window_bottom = ctx.available_rect().bottom() - 17.0;

                        if response.drag_delta().y <= -0.5
                            && ctx.pointer_latest_pos().unwrap_or_default().y <= window_bottom
                        {
                            config.terminal.open = true;
                            config.terminal.opened_from_close = true;
                            config.terminal.opened_from_close_dragging = true;
                        }

                        let stroke = if is_dragging {
                            ui.style().visuals.widgets.active.bg_stroke
                        } else if h_response.hovered() {
                            ui.style().visuals.widgets.hovered.bg_stroke
                        } else {
                            ui.style().visuals.widgets.noninteractive.bg_stroke
                        };

                        ui.painter().rect_filled(center_line, 2.0, stroke.color);
                    });
                });
            });
    }
}
