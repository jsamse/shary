#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use color_eyre::{eyre::Context, Report, Result};
use egui::Ui;
use rfd::FileDialog;
use std::{net::UdpSocket, path::PathBuf};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    setup_tracing()?;
    info!("Starting");
    let options = eframe::NativeOptions::default();
    let app = App::default();
    eframe::run_native(
        &format!("Shary {}", app.key),
        options,
        Box::new(|_cc| Box::new(App::default())),
    );
    Ok(())
}

fn setup_tracing() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .wrap_err("setting default tracing subscriber failed")
}

struct Send {
    path: PathBuf,
}

struct App {
    key: String,
    state: Result<AppState>,
}

struct AppState {
    sends: Vec<Send>,
    socket: UdpSocket,
}

impl AppState {
    fn add_send<F>(&mut self, path: F) -> Option<()>
    where
        F: Fn() -> Option<PathBuf>,
    {
        let path = path()?;
        for existing in self.sends.iter() {
            if existing.path == path {
                info!("Path already shared: {}", path.to_str().unwrap_or_default());
                return None;
            }
        }
        let send = Send { path };
        self.sends.push(send);
        Some(())
    }

    fn remove_send(&mut self, index: usize) {
        self.sends.remove(index);
    }
}

impl Default for App {
    fn default() -> Self {
        fn new_app_state() -> Result<AppState> {
            let socket =
                UdpSocket::bind("0.0.0.0:0").wrap_err("failed to bind broadcast socket")?;
            socket.set_broadcast(true)?;
            Ok(AppState {
                sends: vec![],
                socket,
            })
        }

        Self {
            key: random_string::generate(6, "ABCDEFGHIJKLMNOPQRSTUVXYZ"),
            state: new_app_state(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| draw_app(ui, self));
    }
}

fn draw_app(ui: &mut Ui, app: &mut App) {
    match app.state.as_mut() {
        Ok(state) => draw_app_state_ok(ui, state),
        Err(report) => draw_app_state_err(ui, report),
    }
}

fn draw_app_state_ok(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Send");
    if ui.button("Send file").clicked() {
        state.add_send(|| FileDialog::new().pick_file());
    }
    if ui.button("Send folder").clicked() {
        state.add_send(|| FileDialog::new().pick_folder());
    }
    ui.label(format!(
        "or you can drag and drop folders or files to start sharing them"
    ));
    if state.sends.len() > 0 {
        ui.add_space(16f32);
    }
    let mut stops = vec![];
    for i in 0..state.sends.len() {
        if let Some(str) = state.sends[i].path.to_str() {
            ui.collapsing(str, |ui| {
                if ui.button("Stop").clicked() {
                    stops.push(i);
                }
            });
        }
    }
    stops.reverse();
    for i in stops {
        state.remove_send(i);
    }
    ui.add_space(16f32);
    ui.separator();
    ui.heading("Receive");
}

fn draw_app_state_err(ui: &mut Ui, report: &mut Report) {
    ui.heading("Failed to initialize network");
    ui.label(format!("{}", report));
    ui.label(format!("{}", report.root_cause()));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn sends_equal() {}
}
