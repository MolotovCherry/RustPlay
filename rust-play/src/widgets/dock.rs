use rand::Rng;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use cargo_player::{BuildType, Channel, Edition, File, Project, Subcommand};
use egui::{vec2, Align2, Color32, Id, Ui, Vec2, Window};
use egui_dock::{DockArea, Node, NodeIndex, Style, TabAddAlign};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::config::{Command, Config, GitHub, MenuCommand, TabCommand};
use crate::utils::data::Data;

use super::code_editor::CodeEditor;
use super::titlebar::TITLEBAR_HEIGHT;

pub type Tree = egui_dock::Tree<Tab>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub name: String,
    pub editor: CodeEditor,
    pub id: Id,
    scroll_offset: Option<Vec2>,
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
            editor: CodeEditor::default(),
            id: Id::new("Scratch 1"),
            scroll_offset: None,
        };

        let mut tree = Tree::new(vec![tab]);
        tree.set_focused_node(NodeIndex::root());
        tree
    }
}

pub struct Dock;

impl Dock {
    pub fn show(ctx: &egui::Context, config: &mut Config, ui: &mut Ui) {
        let tree = &mut config.dock.tree;

        let mut style = Style::from_egui(ctx.style().as_ref());

        // important, otherwise it'll draw over the original titlebar
        style.tab_bar_background_color = Color32::TRANSPARENT;
        style.tab_bar_height = TITLEBAR_HEIGHT as f32 / 2.0;
        style.tabs_are_draggable = true;
        style.tab_include_scrollarea = false;
        style.show_add_buttons = true;
        style.add_tab_align = TabAddAlign::Left;
        style.show_context_menu = true;

        let tab_data = TabData::new();

        let mut tab_viewer = TabViewer::new(ctx, &tab_data);

        DockArea::new(tree)
            .style(style)
            .show_inside(ui, &mut tab_viewer);

        // keep the terminal active display on the selected tab
        if let Some((_, tab)) = tree.find_active() {
            config.terminal.active_tab = Some(tab.id);
        }

        // add data to command vec
        config
            .dock
            .commands
            .extend_from_slice(tab_data.borrow().as_slice());
    }
}

type TabData = Data<Command>;

struct TabViewer<'a> {
    _ctx: &'a egui::Context,
    data: &'a TabData,
}

impl<'a> TabViewer<'a> {
    fn new(ctx: &'a egui::Context, data: &'a TabData) -> Self {
        Self { _ctx: ctx, data }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // multiple tabs may be open on the screen, so we need to know if one is focused or not so we don't steal focus
        ui.horizontal(|ui| {
            if ui.button("Play").clicked() {
                let mut data = self.data.borrow_mut();
                data.push(Command::TabCommand(TabCommand::Play(tab.id)));
            }
        });

        ui.vertical_centered(|ui| {
            tab.scroll_offset = Some(tab.editor.show(
                tab.id.with("code_editor"),
                ui,
                tab.scroll_offset.unwrap_or_default(),
            ));
        });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab.name).into()
    }

    fn on_add(&mut self, node: NodeIndex) {
        let mut data = self.data.borrow_mut();
        data.push(Command::TabCommand(TabCommand::Add(node)));
    }

    fn context_menu(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let mut data = self.data.borrow_mut();

        let rename_btn = ui.button("Rename".to_string()).clicked();
        let save_btn = ui.button("Save...".to_string()).clicked();
        let share_btn = ui.button("Share to Playground".to_string()).clicked();

        let mut command = None;

        if rename_btn {
            command = Some(MenuCommand::Rename(tab.id));
        }

        if save_btn || share_btn {
            command = Some(if save_btn {
                MenuCommand::Save(tab.id)
            } else {
                MenuCommand::Share(tab.id)
            });
        }

        if let Some(command) = command {
            data.push(Command::MenuCommand(command));
            ui.close_menu();
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        let mut data = self.data.borrow_mut();
        data.push(Command::TabCommand(TabCommand::Close(tab.id)));

        true
    }
}

#[derive(Debug)]
pub struct TabEvents;

impl TabEvents {
    pub fn show(ctx: &egui::Context, config: &mut Config) {
        // Functions which return false remove their item from the vec.
        config.dock.commands.retain(|i| match i {
            Command::MenuCommand(command) => match command {
                MenuCommand::Rename(v) => Self::show_rename_window(ctx, *v, &mut config.dock.tree),
                MenuCommand::Save(_) => todo!(),
                MenuCommand::Share(v) => {
                    Self::share_scratch(*v, &mut config.dock.tree, &config.github)
                }
            },

            Command::TabCommand(command) => match command {
                TabCommand::Add(v) => {
                    let name = format!("Scratch {}", config.dock.counter);

                    let node_tabs = &config.dock.tree[*v];

                    let tab = Tab {
                        // unique name based on current nodeindex + tabindex
                        id: Id::new(format!("{name}-{}-{}", v.0, node_tabs.tabs_count() + 1)),
                        name,
                        editor: CodeEditor::default(),
                        scroll_offset: None,
                    };

                    config.dock.tree.set_focused_node(*v);
                    config.dock.tree.push_to_focused_leaf(tab);

                    config.dock.counter += 1;

                    false
                }

                TabCommand::Close(id) => {
                    // TODO: Remove TextEditState from closed tabs so they aren't reused with the same ID
                    let editor_id = id.with("code_edit");

                    // cleanup old textedit state

                    //let res = ctx.memory().data.remove::<TextEditState>(editor_id);

                    //ctx.memory().data.remove::<TextEditState>(editor_id);

                    if config.dock.tree.num_tabs() == 0 {
                        let tab = Tab {
                            name: "Scratch 1".to_string(),
                            editor: CodeEditor::default(),
                            id: Id::new("Scratch 1"),
                            scroll_offset: None,
                        };

                        config.dock.tree.set_focused_node(NodeIndex(0));
                        config.dock.tree.push_to_focused_leaf(tab);

                        config.dock.counter = 2;
                    }

                    false
                }

                TabCommand::Play(id) => {
                    let tab = &mut config
                        .dock
                        .tree
                        .iter_mut()
                        .filter_map(|node| {
                            let Node::Leaf { tabs, .. } = node else {
                                return None;
                            };

                            tabs.iter_mut().find(|tab| tab.id == *id)
                        })
                        .collect::<SmallVec<[&mut Tab; 1]>>()[0];

                    let id = *id;
                    let code = tab.editor.code.clone();

                    // this are used as a thread abort signaler
                    let (atx, arx) = channel();

                    let mut rng = rand::thread_rng();
                    let abort_rid: u64 = rng.gen();

                    let abort_id = id.with(format!("_thread_aborter_{abort_rid}"));

                    let prev = config.terminal.abortable.insert(id, abort_id);
                    // if there's a previous process running, send the signal abort
                    type Aborter = Arc<Mutex<Sender<()>>>;
                    if let Some(atx) = prev {
                        let mut mem = ctx.memory();
                        if mem.data.get_temp::<Aborter>(atx).is_some() {
                            mem.data.remove::<Aborter>(atx);
                        }
                    }

                    ctx.memory()
                        .data
                        .insert_temp::<Aborter>(abort_id, Arc::new(Mutex::new(atx)));

                    // these are used to stream the terminal output
                    let queue_stdout = Arc::new(Mutex::new(String::new()));
                    let queue_stderr = Arc::new(Mutex::new(String::new()));

                    let sender_queue_stdout = Arc::clone(&queue_stdout);
                    let sender_queue_stderr = Arc::clone(&queue_stderr);
                    config
                        .terminal
                        .content
                        .insert(id, (sender_queue_stdout, sender_queue_stderr));

                    let owned_ctx = ctx.clone();

                    thread::spawn(move || {
                        let id = Id::new("continuous_mode");

                        let ctx = owned_ctx;

                        // a counter used to indicate when continuous mode is on. It is on as long as any threads are still running
                        {
                            let mut mem = ctx.memory();
                            let counter = mem.data.get_temp_mut_or_default::<u64>(id);
                            *counter += 1;
                        }

                        let mut command = Project::new(id)
                            .build_type(BuildType::Debug)
                            .channel(Channel::Stable)
                            .file(File::new("main", &code))
                            .edition(Edition::E2021)
                            .subcommand(Subcommand::Run)
                            .target_prefix("rust-play")
                            .env_var("CARGO_TERM_COLOR", "always")
                            // .env_var("CARGO_TERM_PROGRESS_WHEN", "always")
                            // .env_var("CARGO_TERM_PROGRESS_WIDTH", "10")
                            .create()
                            .expect("Oh no");

                        let mut child = command
                            .stderr(Stdio::piped())
                            .stdout(Stdio::piped())
                            .spawn()
                            .unwrap();

                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();

                        // special thread which checks for abort code
                        thread::spawn(move || {
                            // blocking wait for abort
                            let _ = arx.recv();
                            let _ = child.kill();
                        });

                        let stdout_handle = thread::spawn(move || {
                            let stdout_reader = BufReader::new(stdout);
                            for line in stdout_reader.lines() {
                                if let Ok(line) = line {
                                    let mut lock = queue_stdout.lock().unwrap();
                                    lock.push_str(&line);
                                    lock.push('\n');
                                } else {
                                    panic!("Unable to send line {line:?}");
                                }
                            }
                        });

                        let stderr_handle = thread::spawn(move || {
                            let stderr_reader = BufReader::new(stderr);
                            for line in stderr_reader.lines() {
                                if let Ok(line) = line {
                                    let mut lock = queue_stderr.lock().unwrap();
                                    lock.push_str(&line);
                                    lock.push('\n');
                                } else {
                                    panic!("Unable to send line {line:?}");
                                }
                            }
                        });

                        // kick off the repaints
                        ctx.request_repaint();
                        let _ = stdout_handle.join();
                        let _ = stderr_handle.join();

                        let mut mem = ctx.memory();
                        let counter = mem.data.get_temp_mut_or_default::<u64>(id);
                        *counter -= 1;

                        let aborter = mem.data.get_temp::<Aborter>(abort_id);
                        if aborter.is_some() {
                            mem.data.remove::<Aborter>(abort_id);
                        }
                    });

                    false
                }
            },
        });
    }

    fn show_rename_window(ctx: &egui::Context, id: Id, tree: &mut Tree) -> bool {
        let tab = &mut tree
            .iter_mut()
            .filter_map(|node| {
                let Node::Leaf { tabs, .. } = node else {
                    return None;
                };

                tabs.iter_mut().find(|tab| tab.id == id)
            })
            .collect::<SmallVec<[&mut Tab; 1]>>()[0];

        Window::new(&format!("Rename {}", tab.name))
            .title_bar(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .auto_sized()
            .show(ctx, |ui| {
                if ui.button("Done").clicked() {
                    tab.name = "nice".to_string();
                    return false;
                }

                true
            })
            .unwrap()
            .inner
            .unwrap()
    }

    fn share_scratch(id: Id, tree: &mut Tree, github: &GitHub) -> bool {
        println!("shared scratch token: {}", github.access_token);

        false
    }
}
