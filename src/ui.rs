use crate::{
    common::{LocalFile, RemoteFile, Files},
};
use rfd::FileDialog;
use tokio::sync::watch;
use std::{path::PathBuf, sync::Arc};

pub fn run(files: Arc<Files>) {
    let mut options = eframe::NativeOptions::default();
    options.drag_and_drop_support = true;
    let local_files = files.get_local_files();
    let remote_files = files.get_remote_files();
    let app = App {
        files,
        local_files,
        remote_files,
    };
    eframe::run_native(&"Shary", options, Box::new(|_cc| Box::new(app)));
}

enum Action {
    AddSend(PathBuf),
    RemoveSend(LocalFile),
    Download(RemoteFile, PathBuf),
}

struct App {
    files: Arc<Files>,
    local_files: watch::Receiver<Vec<LocalFile>>,
    remote_files: watch::Receiver<Arc<Vec<RemoteFile>>>,
}

macro_rules! some_or_continue {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => continue,
        }
    };
}

macro_rules! ok_or_continue {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => continue,
        }
    };
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for dropped_file in ctx.input().raw.dropped_files.iter() {
            let path = some_or_continue!(dropped_file.path.clone());
            let local_file = ok_or_continue!(LocalFile::new(path));
            self.files.add_local_file(local_file);
        }
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            let remote_files = self.remote_files.borrow().clone();
            let actions = self.draw(ui, remote_files);
            let _updated = actions
                .into_iter()
                .map(|a| self.handle_action(a))
                .any(|a| a);
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
        let local_files = self.local_files.borrow();
        if local_files.len() > 0 {
            ui.add_space(16f32);
        }
        for file in local_files.iter() {
            if ui.button(file.name.clone()).clicked() {
                actions.push(Action::RemoveSend(file.clone()));
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
                match LocalFile::new(path) {
                    Ok(local_file) => self.files.add_local_file(local_file),
                    Err(_) => false,
                }
            }
            Action::RemoveSend(local_file) => {
                self.files.remove_local_file(&local_file)
            }
            Action::Download(file, path) => {
                self.files.add_download(file, path);
                false
            },
        }
    }
}