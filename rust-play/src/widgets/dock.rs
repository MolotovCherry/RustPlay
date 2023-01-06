use std::sync::mpsc::Sender;

use egui::{vec2, Color32, Rect, TextStyle, Ui, Vec2, Window};
use egui_dock::{DockArea, Node, NodeIndex, Style, TabAddAlign, TabIndex};
use serde::{Deserialize, Serialize};

use crate::config::{Command, Config, MenuCommand, TabCommand};
use crate::utils::data::Data;

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

pub type Tree = egui_dock::Tree<Tab>;

// Each rectangle is an entire tree; not a single tab
#[cfg(target_os = "windows")]
pub type CoveredRects = SmallVec<[Rect; 10]>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    name: String,
    content: String,
}

pub trait TreeTabs
where
    Self: Sized,
{
    fn init() -> Self;
}

// Initialize the initial tabs / tab data
impl TreeTabs for Tree {
    fn init() -> Self {
        let tab = Tab {
            name: "Scratch 1".to_string(),
            content: "".to_string(),
        };

        Tree::new(vec![tab])
    }
}

pub struct Dock<'app> {
    #[cfg(target_os = "windows")]
    tx: &'app Sender<CoveredRects>,
}

impl<'app> Dock<'app> {
    pub fn new(#[cfg(target_os = "windows")] tx: &'app Sender<CoveredRects>) -> Self {
        Self {
            #[cfg(target_os = "windows")]
            tx,
        }
    }

    pub fn show(self, ctx: &egui::Context, config: &mut Config) {
        let tree = &mut config.dock.tree;

        let mut style = Style::from_egui(ctx.style().as_ref());

        // important, otherwise it'll draw over the original titlebar
        style.tab_bar_background_color = Color32::TRANSPARENT;
        style.tab_bar_height = TITLEBAR_HEIGHT;
        style.tabs_are_draggable = true;
        style.tab_include_scrollarea = false;
        style.show_add_buttons = true;
        style.add_tab_align = TabAddAlign::Left;
        style.show_context_menu = true;

        let tab_data = TabData::new();
        let mut tab_viewer = TabViewer::new(ctx, &tab_data);

        let ui = DockArea::new(tree)
            .style(style.clone())
            .show(ctx, &mut tab_viewer);

        // get list of covered rectangles for decorator
        #[cfg(target_os = "windows")]
        {
            let covered_rects = tree.covered(ui, style, &mut tab_viewer);
            let _ = self.tx.send(covered_rects);
        }

        // add data to command vec
        config
            .dock
            .command
            .extend_from_slice(tab_data.borrow().as_slice());
    }
}

type TabData = Data<Command>;

struct TabViewer<'a> {
    ctx: &'a egui::Context,
    data: &'a TabData,
}

impl<'a> TabViewer<'a> {
    fn new(ctx: &'a egui::Context, data: &'a TabData) -> Self {
        Self { ctx, data }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {}", tab.name));
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab.name).into()
    }

    fn on_add(&mut self, node: NodeIndex) {
        let mut data = self.data.borrow_mut();
        data.push(Command::TabCommand(TabCommand::Add(node)));
    }

    fn context_menu(
        &mut self,
        ui: &mut Ui,
        tab: &mut Self::Tab,
        tabindex: TabIndex,
        nodeindex: NodeIndex,
    ) {
        let mut data = self.data.borrow_mut();

        let rename_btn = ui.button("Rename".to_string()).clicked();
        let save_btn = ui.button("Save...".to_string()).clicked();
        let share_btn = ui.button("Share to Playground".to_string()).clicked();

        let mut command = None;

        if rename_btn {
            command = Some(MenuCommand::Rename((nodeindex, tabindex, ui.min_rect())));
        }

        if save_btn || share_btn {
            command = Some(if save_btn {
                MenuCommand::Save((nodeindex, tabindex))
            } else {
                MenuCommand::Share((nodeindex, tabindex))
            });
        }

        if let Some(command) = command {
            data.push(Command::MenuCommand(command));
            ui.close_menu();
        }
    }
}

#[cfg(target_os = "windows")]
trait TreeCoveredArea {
    fn covered(
        &mut self,
        ui: Ui,
        style: Style,
        viewer: &mut impl egui_dock::TabViewer<Tab = Tab>,
    ) -> CoveredRects;
}

#[cfg(target_os = "windows")]
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

#[derive(Debug, Clone, Default)]
struct TabEvents {
    rename: Option<(NodeIndex, TabIndex, Rect)>,
}

impl TabEvents {
    fn show(&mut self, ctx: &egui::Context, tree: &mut Tree) {
        if self.rename.is_some() {
            self.show_rename_window(ctx, tree);
        }
    }

    fn show_rename_window(&mut self, ctx: &egui::Context, tree: &mut Tree) {
        let (nodeindex, tabindex, rect) = self.rename.unwrap();

        let tab = tree.get_tab_mut(nodeindex, tabindex).unwrap();
        dbg!("foo");

        Window::new(&tab.name).title_bar(true).show(ctx, |ui| {});
    }
}
