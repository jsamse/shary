use crate::{
    common::{LocalFile, RemoteFile},
    network::NetworkHandle,
};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};

pub fn run(network: NetworkHandle) {
    let options = eframe::NativeOptions::default();
    let app = App {
        network,
        local_files: vec![],
    };
    eframe::run_native(&"Shary", options, Box::new(|_cc| Box::new(app)));
}

enum Action {
    AddSend(PathBuf),
    RemoveSend(String),
    Download(RemoteFile, PathBuf),
}

struct App {
    network: NetworkHandle,
    local_files: Vec<LocalFile>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            let remote_files = self.network.remote_files();
            let actions = self.draw(ui, remote_files);
            let updated = actions
                .into_iter()
                .map(|a| self.handle_action(a))
                .any(|a| a);
            if !updated {
                return;
            }
            let local_files = Arc::new(self.local_files.clone());
            self.network.set_local_files(local_files).unwrap();
        });
    }
}

impl App {
    fn draw(&mut self, ui: &mut egui::Ui, remote_files: Arc<Vec<RemoteFile>>) -> Vec<Action> {
        let mut actions = vec![];
        ui.heading("Send");
        if ui.button("Send file").clicked() {
            let path = FileDialog::new().pick_file();
            if let Some(path) = path {
                actions.push(Action::AddSend(path));
            }
        }
        if ui.button("Send folder").clicked() {
            let path = FileDialog::new().pick_folder();
            if let Some(path) = path {
                actions.push(Action::AddSend(path));
            }
        }
        ui.label(format!(
            "or you can drag and drop folders or files to start sharing them"
        ));
        if self.local_files.len() > 0 {
            ui.add_space(16f32);
        }
        for file in self.local_files.iter() {
            if ui.button(file.name.clone()).clicked() {
                actions.push(Action::RemoveSend(file.name.clone()));
            }
        }
        ui.add_space(16f32);
        ui.separator();
        ui.heading("Receive");

        for file in remote_files.iter() {
            if ui.button(file.file.clone()).clicked() {
                if let Some(path) = FileDialog::new().pick_folder() {
                    actions.push(Action::Download(file.clone(), path));
                }
            }
        }

        actions
    }

    fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::AddSend(path) => {
                let name = path
                    .file_name()
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
            }
            Action::RemoveSend(name) => {
                for i in 0..self.local_files.len() {
                    if self.local_files[i].name == name {
                        self.local_files.remove(i);
                        return true;
                    }
                }
                false
            }
            Action::Download(file, path) => {
                self.network.download(&file, path);
                false
            },
        }
    }
}
