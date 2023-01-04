use std::sync::mpsc::Sender;

use egui::{vec2, Color32, Rect, TextStyle, Ui, Vec2};
use egui_dock::{DockArea, Node, NodeIndex, Style, TabAddAlign};

#[cfg(target_os = "windows")]
use {
    crate::os::windows::custom_frame::{self, win32_captionbtn_rect},
    smallvec::SmallVec,
    windows::Win32::{
        Foundation::RECT,
        UI::{Input::KeyboardAndMouse::GetActiveWindow, WindowsAndMessaging::GetWindowRect},
    },
};

// Height of the title bar
#[cfg(target_os = "windows")]
const TITLEBAR_HEIGHT: f32 = (custom_frame::TITLEBAR_HEIGHT / 2) as f32;
#[cfg(not(target_os = "windows"))]
const TITLEBAR_HEIGHT: f32 = 40.0 as f32;
// private constant in egui_dock
#[cfg(target_os = "windows")]
const TAB_PLUS_SIZE: f32 = 24.0;

pub type Tab = String;
pub type Tree = egui_dock::Tree<Tab>;
// Each rectangle is an entire tree; not a single tab
#[cfg(target_os = "windows")]
pub type CoveredRects = SmallVec<[Rect; 10]>;

pub trait TreeTabs {
    fn init() -> Self;
}

impl TreeTabs for Tree {
    fn init() -> Self {
        let tree = Tree::new(vec![
            "tab1".to_owned(),
            "tab2".to_owned(),
            "tab34444".to_owned(),
        ]);

        tree
    }
}

pub struct Dock<'app> {
    #[cfg(target_os = "windows")]
    tx: &'app Sender<CoveredRects>,
    tree: &'app mut Tree,
}

impl<'app> Dock<'app> {
    #[cfg(target_os = "windows")]
    pub fn new(tree: &'app mut Tree, tx: &'app Sender<CoveredRects>) -> Self {
        Self { tx, tree }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new(tree: &'app mut Tree) -> Self {
        Self { tree }
    }

    pub fn show(mut self, ctx: &egui::Context) {
        let mut style = Style::from_egui(ctx.style().as_ref());

        // important, otherwise it'll draw over the original titlebar
        style.tab_bar_background_color = Color32::TRANSPARENT;
        style.tab_bar_height = TITLEBAR_HEIGHT;
        style.tabs_are_draggable = true;
        style.tab_include_scrollarea = false;
        style.show_add_buttons = true;
        style.add_tab_align = TabAddAlign::Left;
        style.show_context_menu = true;

        let mut tab_viewer = TabViewer {};

        let ui = DockArea::new(&mut self.tree)
            .style(style.clone())
            .show(ctx, &mut tab_viewer);

        // get list of covered rectangles for decorator
        #[cfg(target_os = "windows")]
        {
            let covered_rects = self.tree.covered(ui, style, &mut tab_viewer);
            let _ = self.tx.send(covered_rects);
        }
    }
}

#[derive(Debug)]
struct TabViewer {}

impl egui_dock::TabViewer for TabViewer {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }
}

trait TreeCoveredArea {
    fn covered(
        &mut self,
        ui: Ui,
        style: Style,
        viewer: &mut impl egui_dock::TabViewer<Tab = Tab>,
    ) -> CoveredRects;
}

impl TreeCoveredArea for Tree {
    // Calculate the covered surface area for the entire tree, and return it in a list
    fn covered(
        &mut self,
        ui: Ui,
        style: Style,
        viewer: &mut impl egui_dock::TabViewer<Tab = Tab>,
    ) -> CoveredRects {
        // Update and send over covered rectangles for the win32 decorator to properly handle ca in nca
        let mut covered_rects = CoveredRects::new();

        for node_index in 0..self.len() {
            let node_index = NodeIndex(node_index);
            if let Node::Leaf { rect, tabs, .. } = &mut self[node_index] {
                // Make sure the rect coords are actual coods, and they're on the top bar (we don't care otherwise if they're not in the decorator)
                if rect.is_finite() && rect.top() == 0.0 {
                    let mut total_tabs_size = Rect::NOTHING;
                    total_tabs_size.set_left(rect.left());
                    total_tabs_size.set_top(0.0);
                    total_tabs_size.set_bottom(style.tab_bar_height);
                    total_tabs_size.set_right(0.0);

                    let height_topbar = style.tab_bar_height;

                    let bottom_y = rect.min.y + height_topbar;
                    let tabbar = rect.intersect(Rect::everything_above(bottom_y));

                    let mut available_width = tabbar.max.x - tabbar.min.x;
                    if style.show_add_buttons {
                        available_width -= TAB_PLUS_SIZE;
                    }
                    let expanded_width = available_width / (tabs.len() as f32);

                    // add up the individual tab sizes
                    for tab in tabs.iter_mut() {
                        let label = viewer.title(tab);

                        let galley = label.into_galley(&ui, None, f32::INFINITY, TextStyle::Button);

                        let x_size = Vec2::splat(galley.size().y / 1.3);

                        let offset = vec2(8.0, 0.0);

                        let desired_size = if style.expand_tabs {
                            vec2(expanded_width, style.tab_bar_height)
                        } else if style.show_close_buttons {
                            vec2(
                                galley.size().x + offset.x * 2.0 + x_size.x + 5.0,
                                style.tab_bar_height,
                            )
                        } else {
                            vec2(galley.size().x + offset.x * 2.0, style.tab_bar_height)
                        };

                        // increase the right edge size by x

                        total_tabs_size.set_right(
                            total_tabs_size.left() + total_tabs_size.right() + desired_size.x,
                        );
                    }

                    if style.show_add_buttons {
                        total_tabs_size.set_right(total_tabs_size.right() + TAB_PLUS_SIZE);
                    }

                    // multiply it by 2 to get the total screen size for win32
                    total_tabs_size.set_right(total_tabs_size.right() * 2.0);
                    total_tabs_size.set_bottom(total_tabs_size.bottom() * 2.0);

                    if total_tabs_size.left() > 0.0 {
                        // 10 is used to allow for resize handle in titlebar
                        total_tabs_size.set_left(total_tabs_size.left() * 2.0 - 10.0);
                    }

                    // now we got all the dimensions for the rectangle, but we should check if we need to clip it
                    // due to us having a titlebar and all. Let's not go over the minimize, maximize/window, close buttons
                    let hwnd = unsafe { GetActiveWindow() };
                    let caption_rect = unsafe { win32_captionbtn_rect(hwnd) };
                    if let Some(caption_rect) = caption_rect {
                        // note that the caption rect is in screen coords!
                        let mut rc_window = RECT::default();
                        unsafe {
                            GetWindowRect(hwnd, &mut rc_window);
                        }

                        // now convert the screen coords to local window coords
                        let mut local_caption_rect = Rect::NOTHING;
                        local_caption_rect.set_left((caption_rect.left - rc_window.left) as f32);
                        local_caption_rect.set_right((caption_rect.right - rc_window.left) as f32);
                        local_caption_rect.set_top((caption_rect.top - rc_window.top) as f32);
                        local_caption_rect.set_bottom((caption_rect.bottom - rc_window.top) as f32);

                        // the right side is really the only one that ever clips into it, so..
                        if total_tabs_size.right() >= (local_caption_rect.left() - 30.0) {
                            // the right edge of the client area cannot go beyond this
                            total_tabs_size.set_right(local_caption_rect.left() - 30.0);
                        }

                        covered_rects.push(total_tabs_size);
                    }
                }
            }
        }

        covered_rects
    }
}
