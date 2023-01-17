use egui::{
    lerp, vec2, CentralPanel, Color32, ColorImage, Context, Frame, Id, Image, Rect, Rgba, Sense,
    Stroke, TextureHandle, Ui,
};

use once_cell::sync::OnceCell;
use resvg::{tiny_skia, usvg};
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WINDOWPLACEMENT,
};

pub const TITLEBAR_HEIGHT: i32 = 80;
pub const CAPTION_WIDTH_CLOSE: u32 = 94;
pub const CAPTION_WIDTH_MAXRESTORE: u32 = 87;
pub const CAPTION_WIDTH_MINIMIZE: u32 = 92;
pub const CAPTION_HEIGHT: u32 = 58;
pub const CAPTION_PADDING: u32 = 4;

macro_rules! egui_dimens {
    ($var:ident) => {
        $var as f32 / 2.0
    };
}

pub fn custom_window_frame(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    add_contents: impl FnOnce(&mut Ui),
) {
    let is_maximized = unsafe {
        let hwnd = GetActiveWindow();
        let mut wp = WINDOWPLACEMENT::default();
        GetWindowPlacement(hwnd, &mut wp);

        if wp.showCmd == SW_MAXIMIZE {
            true
        } else {
            false
        }
    };

    // Height of the title bar
    const HEIGHT: f32 = egui_dimens!(TITLEBAR_HEIGHT);
    const CAPT_WIDTH_CLOSE: f32 = egui_dimens!(CAPTION_WIDTH_CLOSE);
    const CAPT_WIDTH_MAXRESTORE: f32 = egui_dimens!(CAPTION_WIDTH_MAXRESTORE);
    const CAPT_WIDTH_MINIMIZE: f32 = egui_dimens!(CAPTION_WIDTH_MINIMIZE);
    let capt_height: f32 = if !is_maximized {
        egui_dimens!(CAPTION_HEIGHT)
    } else {
        CAPTION_HEIGHT as f32 / 1.70
    };
    const CAPT_PAD: f32 = egui_dimens!(CAPTION_PADDING);

    CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, |ui| {
            // on windows, when maximized, there's a gap. So if maximized, we should shrunk the maximum rect
            let rect = if is_maximized {
                ui.max_rect().shrink(6.5)
            } else {
                ui.max_rect()
            };

            let painter = ui.painter();

            // Paint the frame:
            painter.rect(
                rect.shrink(1.0),
                if cfg!(target_os = "windows") {
                    0.0
                } else {
                    10.0
                },
                Color32::TRANSPARENT,
                // todo: None on windows, something on Linux
                Stroke::NONE,
            );

            // Interact with the title bar (drag to move window):
            let title_bar_rect = {
                let mut rect = rect;
                rect.max.y = rect.min.y + HEIGHT;
                rect
            };
            let title_bar_response =
                ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click());
            if title_bar_response.is_pointer_button_down_on() {
                frame.drag_window();
            }

            // Close rect
            let mut close_rect = rect;
            close_rect.set_left(rect.right() - CAPT_WIDTH_CLOSE);
            close_rect.set_bottom(capt_height);

            // Maximize/restore rect
            let mut maximize_rect = rect;
            maximize_rect.set_left(close_rect.left() - CAPT_WIDTH_MAXRESTORE - 1.0);
            maximize_rect.set_right(close_rect.left() - 1.0);
            maximize_rect.set_bottom(capt_height);

            // minimize rect
            let mut minimize_rect = rect;
            minimize_rect.set_left(maximize_rect.left() - CAPT_WIDTH_MINIMIZE - CAPT_PAD);
            minimize_rect.set_right(maximize_rect.left() - CAPT_PAD);
            minimize_rect.set_bottom(capt_height);

            // Handle caption buttons
            //
            // CLOSE BTN
            //
            caption_btn(
                ctx,
                ui,
                CaptionIcon::Close,
                close_rect,
                Color32::from_rgb(196, 43, 28),
                Color32::from_rgb(176, 40, 26),
                "titlebar::close_btn",
                || {
                    frame.close();
                },
            );

            //
            // MAX/RESTORE BTN
            //
            caption_btn(
                ctx,
                ui,
                CaptionIcon::MaximizeRestore,
                maximize_rect,
                Color32::from_rgba_unmultiplied(255, 255, 255, 3),
                Color32::from_rgba_unmultiplied(255, 255, 255, 2),
                "titlebar::maximize_btn",
                || unsafe {
                    let hwnd = GetActiveWindow();

                    if is_maximized {
                        ShowWindow(hwnd, SW_RESTORE);
                    } else {
                        ShowWindow(hwnd, SW_MAXIMIZE);
                    }
                },
            );

            //
            // MINIMIZE BTN
            //
            caption_btn(
                ctx,
                ui,
                CaptionIcon::Minimize,
                minimize_rect,
                Color32::from_rgba_unmultiplied(255, 255, 255, 3),
                Color32::from_rgba_unmultiplied(255, 255, 255, 2),
                "titlebar::minimize_btn",
                || unsafe {
                    ShowWindow(GetActiveWindow(), SW_MINIMIZE);
                },
            );

            // Add the contents:
            let mut content_ui = ui.child_ui(rect, *ui.layout());
            let mut clip_rect = rect;
            clip_rect.set_left(minimize_rect.left() - 10.0);
            clip_rect.set_bottom(capt_height);
            content_ui.set_clip_rect(clip_rect);

            add_contents(&mut content_ui);
        });
}

macro_rules! icon {
    ($ctx:ident, $name:ident) => {{
        paste::paste! {
            const [<$name:upper _ICON_B>]: &[u8] = include_bytes!(concat!("../../resources/titlebar/", stringify!([<$name:lower>]), ".svg"));
            static [<$name:upper _ICON>]: OnceCell<(TextureHandle, (u32, u32))> = OnceCell::new();

            {
                let (texture, size) = [<$name:upper _ICON>].get_or_init(|| {
                    let tree = usvg::Tree::from_data([<$name:upper _ICON_B>], &usvg::Options::default()).unwrap();
                    let pixmap_size = tree.size.to_screen_size();
                    let mut pixmap =
                        tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

                    resvg::render(
                        &tree,
                        usvg::FitTo::Original,
                        tiny_skia::Transform::default(),
                        pixmap.as_mut(),
                    );

                    let texture = $ctx.load_texture(
                        "",
                        ColorImage::from_rgba_unmultiplied(
                            [pixmap_size.width() as usize, pixmap_size.height() as usize],
                            pixmap.data(),
                        ),
                        Default::default(),
                    );

                    (texture, (pixmap_size.width(), pixmap_size.height()))
                });

                Image::new(texture, [size.0 as f32, size.1 as f32])
            }
        }}
    };
}

enum CaptionIcon {
    Close,
    MaximizeRestore,
    Minimize,
}

#[allow(clippy::too_many_arguments)]
fn caption_btn(
    ctx: &Context,
    ui: &mut Ui,
    icon: CaptionIcon,
    rect: Rect,
    color: Color32,
    clicked_color: Color32,
    id: &str,
    mut action: impl FnMut(),
) {
    let close_icon = icon!(ctx, close);
    let minimize_icon = icon!(ctx, minimize);
    let restore_icon = icon!(ctx, restore);
    let maximize_icon = icon!(ctx, maximize);

    let painter = ui.painter();

    let sense = Sense::click_and_drag();

    let id = Id::new(id);

    let response = ui.interact(rect, id, sense);

    // workaround for a problem where checking if hovered, or using hovered pos is imprecise
    // so use the mouse coords and check it's inside the rect to make it exact
    let target_value = if let Some(pos) = ctx.pointer_latest_pos() {
        rect.contains(pos)
    } else {
        false
    };

    let anim = ctx.animate_bool_with_time(id, target_value, 0.1);

    let hover_color = lerp(Rgba::from(Color32::TRANSPARENT)..=Rgba::from(color), anim);

    if response.clicked() {
        painter.rect(rect, 0.0, clicked_color, Stroke::NONE);
        action();
    } else if response.is_pointer_button_down_on() || response.dragged() {
        // only allow dragging as long as mouse is within button
        // unlike other times, dragging out of the area causes it to instantly disappear rather than fade (we're not calling else)
        if rect.contains(ctx.pointer_latest_pos().unwrap_or_default()) {
            painter.rect(rect, 0.0, clicked_color, Stroke::NONE);
        }
    } else {
        painter.rect(rect, 0.0, hover_color, Stroke::NONE);
    }

    let rect_icon = Rect::from_center_size(rect.center(), vec2(10.0, 10.0));

    match icon {
        CaptionIcon::Close => {
            close_icon.paint_at(ui, rect_icon);
        }

        CaptionIcon::MaximizeRestore => unsafe {
            let hwnd = GetActiveWindow();
            let mut wp = WINDOWPLACEMENT::default();
            GetWindowPlacement(hwnd, &mut wp);

            if wp.showCmd == SW_MAXIMIZE {
                restore_icon.paint_at(ui, rect_icon);
            } else {
                maximize_icon.paint_at(ui, rect_icon);
            }
        },

        CaptionIcon::Minimize => {
            minimize_icon.paint_at(ui, rect_icon);
        }
    }
}
