use egui::panel::PanelState;
use egui::{pos2, vec2, CursorIcon, Id, Rect, Sense};

use crate::config::Config;

use super::titlebar::TITLEBAR_HEIGHT;

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
            .show(ctx, |ui| {
                let mut close_rect = ctx.available_rect();

                if config.terminal.opened_from_close_dragging {
                    close_rect.set_top(close_rect.bottom() - 15.0);
                } else {
                    close_rect.set_top(close_rect.bottom() - 20.0);
                };

                let pointer_pos = ctx.pointer_latest_pos().unwrap_or_default();

                let window_close_bottom = ctx.available_rect().bottom() - 15.0;

                // when mouse is outside of window, as long as we were dragging, pointer_pos is still Some()
                // we can utilize this to allow resizing AS LONG AS mouse isn't below the window in screen coords
                if close_rect.contains(pointer_pos) || pointer_pos.y >= window_close_bottom {
                    config.terminal.open = false;
                    config.terminal.closed_from_open = true;
                }

                let resize_id = id.with("__resize");
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

                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Expandable Upper Panel");
                        });
                    });
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

                        // we need to subtract 11 from the bottom because that's the closing threashold
                        let window_bottom = ctx.available_rect().bottom() - 16.0;

                        if response.drag_delta().y <= -1.5
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
