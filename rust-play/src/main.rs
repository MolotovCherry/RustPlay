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
    widgets::dock::CoveredRects,
};

use std::sync::mpsc::Receiver;

use config::Config;
use panic::set_hook;
use popup::{display_popup, MessageBoxIcon};
use widgets::dock::Dock;

use eframe::{egui, NativeOptions};

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
        transparent: true,
        resizable: true,
        centered: true,
        ..Default::default()
    };

    eframe::run_native("Rust Play", options, Box::new(|_cc| Box::new(app)));
}

struct App {
    config: Config,
    // sends the covered tab area over to the custom frames hit testing code so we can differenitate between
    // tab and uncovered titlebar
    #[cfg(target_os = "windows")]
    tx: Sender<CoveredRects>,
}

impl App {
    #[cfg(target_os = "windows")]
    fn new() -> (Self, Receiver<CoveredRects>) {
        let (tx, rx) = channel();

        let app = Self {
            tx,
            config: Config::default(),
        };

        (app, rx)
    }

    #[cfg(not(target_os = "windows"))]
    fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }
}

impl eframe::App for App {
    // Clear the overlay over the entire background so we have a blank slate to work with
    fn clear_color(&self, _: &egui::Visuals) -> egui::Rgba {
        egui::Rgba::TRANSPARENT
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_dock(ctx);

        //dbg!(&self.config.dock.tab_command);
    }
}

impl App {
    fn show_dock(&mut self, ctx: &egui::Context) {
        Dock::new(
            #[cfg(target_os = "windows")]
            &self.tx,
        )
        .show(ctx, &mut self.config);
    }
}
