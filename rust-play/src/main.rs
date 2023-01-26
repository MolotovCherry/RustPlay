// Hide the console window on Windows when in release mode
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// For specific OS support, like custom windows titlebars
mod os;

mod config;
mod panic;
mod popup;
mod utils;
mod widgets;

#[cfg(target_os = "windows")]
use {
    os::windows::{
        custom_frame::{self},
        init::load_app_icon,
        win_version::is_supported_os,
    },
    std::sync::mpsc::{channel, Sender},
};

use std::env;
use std::fs;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use config::Config;
use egui::{Rect, Ui};
use panic::set_hook;
use popup::{display_popup, MessageBoxIcon};
use widgets::dock::{Dock, TabEvents};

use eframe::{egui, NativeOptions};
use widgets::titlebar::custom_window_frame;

// Each rectangle is an entire tree; not a single tab
#[cfg(target_os = "windows")]
pub type CaptionMaxRect = Rect;

fn main() {
    // set up custom panic hook
    set_hook();

    // check windows version
    #[cfg(target_os = "windows")]
    if !is_supported_os() {
        display_popup(
            "Error",
            "Sorry, your OS is not supported.\n\nThis program requires win10 1809 or greater.",
            MessageBoxIcon::Error,
        );
        return;
    }

    #[cfg(target_os = "windows")]
    let app = {
        let (app, rx) = App::new();

        custom_frame::init(rx);

        app
    };

    #[cfg(not(target_os = "windows"))]
    let app = App::new();

    tracing_subscriber::fmt::init();

    let options = NativeOptions {
        icon_data: Some(load_app_icon()),
        //min_window_size: Some(Vec2::new(500.0, 400.0)),
        initial_window_size: Some(Vec2::new(600.0, 400.0)),
        transparent: true,
        resizable: true,
        centered: true,
        #[cfg(not(target_os = "windows"))]
        decorated: false,
        ..Default::default()
    };

    eframe::run_native("Rust Play", options, Box::new(|_cc| Box::new(app)));
}

struct App {
    config: Config,
    // sends the covered tab area over to the custom frames hit testing code so we can differenitate between
    // tab and uncovered titlebar
    #[cfg(target_os = "windows")]
    tx: Rc<Sender<CaptionMaxRect>>,
}

impl App {
    #[cfg(target_os = "windows")]
    fn new() -> (Self, Receiver<CaptionMaxRect>) {
        let (tx, rx) = channel();

        let current_dir = env::current_exe().unwrap().parent().unwrap().to_owned();
        let file = current_dir.join("settings.toml");

        let mut config = if file.exists() {
            let content = fs::read_to_string(file).expect("Failed to read config file");
            toml::from_str::<Config>(&content).expect("Failed to deserialize Config")
        } else {
            Config::default()
        };

        config.dock.counter = 2;

        let app = Self {
            tx: Rc::new(tx),
            config,
        };

        (app, rx)
    }

    #[cfg(not(target_os = "windows"))]
    fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    fn show_dock(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        Dock::show(ctx, &mut self.config, ui);
    }

    fn handle_tabs(&mut self, ctx: &egui::Context) {
        TabEvents::show(ctx, &mut self.config);
    }
}

impl eframe::App for App {
    fn on_close_event(&mut self) -> bool {
        // Write config to settings.toml

        let config_string =
            toml::to_string(&self.config).expect("Failed to convert config to toml");

        let current_dir = env::current_exe().unwrap().parent().unwrap().to_owned();
        let file = current_dir.join("settings.toml");

        fs::write(file, config_string).expect("Failed to write config file");

        true
    }

    // Clear the overlay over the entire background so we have a blank slate to work with
    fn clear_color(&self, _: &egui::Visuals) -> egui::Rgba {
        egui::Rgba::TRANSPARENT
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        custom_window_frame(
            ctx,
            frame,
            #[cfg(target_os = "windows")]
            Rc::clone(&self.tx),
            |ui| {
                self.show_dock(ctx, ui);
            },
        );

        self.handle_tabs(ctx);
    }
}
