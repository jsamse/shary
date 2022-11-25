use super::Send;
use crate::{network::NetworkHandle, common::{RemoteFile, LocalFile}};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};
use tracing::info;

pub fn run(network_handle: NetworkHandle) {
    let options = eframe::NativeOptions::default();
    let app = App { network_handle, local_files: vec![] };
    eframe::run_native(&"Shary", options, Box::new(|_cc| Box::new(app)));
}

enum Action {
    AddSend(PathBuf),
    RemoveSend(String),
}

struct App {
    network_handle: NetworkHandle,
    local_files: Vec<LocalFile>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            let status = self.network_handle.status.borrow().clone();
            match status {
                crate::network::NetworkStatus::Starting => {
                    ui.spinner();
                },
                crate::network::NetworkStatus::Failed => {
                    ui.heading("Error. Check logs.");
                },
                crate::network::NetworkStatus::Ok(remote_files) => {
                    let actions = self.draw(ui, remote_files);
                    let updated = actions.into_iter()
                        .map(|a| self.handle_action(a))
                        .any(|a| a);
                    if !updated {
                        return;
                    }
                    let local_files = Arc::new(self.local_files.clone());
                    self.network_handle.local_files.send(local_files).unwrap();
                },
            }
        });
    }
}

impl App {
    fn draw(&mut self, ui: &mut egui::Ui, remote_files: Arc<Vec<RemoteFile>>) -> Vec<Action> {
        vec![]
    }

    fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::AddSend(path) => {
                let name = path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| Some(String::from(name)));
                let name = match name {
                    Some(name) => name,
                    None => return false,
                };
                if self.local_files.iter().any(|f| f.name == name) {
                    return false;
                }
                self.local_files.push(LocalFile { path, name });
                true
            },
            Action::RemoveSend(name) => {
                for i in 0..self.local_files.len() {
                    if self.local_files[i].name == name {
                        self.local_files.remove(i);
                        return true;
                    }
                }
                false
            },
        }
    }
}

struct InitializedApp {
    actions: Vec<Action>,
    sends: Vec<Send>,
}

impl InitializedApp {
    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.heading("Send");
        if ui.button("Send file").clicked() {
            let path = FileDialog::new().pick_file();
            if let Some(path) = path {
                self.actions.push(Action::AddSend(path));
            }
        }
        if ui.button("Send folder").clicked() {
            let path = FileDialog::new().pick_folder();
            if let Some(path) = path {
                self.actions.push(Action::AddSend(path));
            }
        }
        ui.label(format!(
            "or you can drag and drop folders or files to start sharing them"
        ));
        if self.sends.len() > 0 {
            ui.add_space(16f32);
        }
        for send in self.sends.iter() {
            ui.collapsing(send.name(), |ui| {
                if ui.button("Stop").clicked() {
                    self.actions
                        .push(Action::RemoveSend(String::from(send.name())));
                }
            });
        }
        ui.add_space(16f32);
        ui.separator();
        ui.heading("Receive");

        self.handle_actions();
    }

    fn handle_actions(&mut self) {
        let mut popped = self.actions.pop();
        while let Some(action) = popped {
            match action {
                Action::AddSend(path) => self.add_send(path),
                Action::RemoveSend(path) => self.remove_send(&path),
            }
            popped = self.actions.pop();
        }
    }

    fn add_send(&mut self, path: PathBuf) {
        if self.sends.iter().any(|s| s.path == path) {
            info!("Path already shared: {}", path.to_str().unwrap_or_default());
            return;
        }
        //self.network.add_send(&path);
        let send = Send { path };
        self.sends.push(send);
    }

    fn remove_send(&mut self, path: &str) {
        let path = PathBuf::from(path);
        //self.network.remove_send(&path);
        self.sends.retain(|s| s.path != path);
    }
}
