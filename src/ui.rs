use crate::{
    common::{Files, LocalFile, RemoteFile},
    ok_or_continue, some_or_continue,
};
use egui::{InnerResponse, Ui};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::watch;

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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for dropped_file in ctx.input().raw.dropped_files.iter() {
            let path = some_or_continue!(dropped_file.path.clone());
            let local_file = ok_or_continue!(LocalFile::new(path));
            self.files.add_local_file(local_file);
        }
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            let remote_files = self.remote_files.borrow().clone();
            let actions = self.draw(ui, &remote_files);
            let _updated = actions
                .into_iter()
                .map(|a| self.handle_action(a))
                .any(|a| a);
        });
    }
}

impl App {
    fn draw(&mut self, ui: &mut egui::Ui, remote_files: &[RemoteFile]) -> Vec<Action> {
        let mut actions = vec![];
        egui::Grid::new("grid").show(ui, |ui| {
            let mut count = 0;
            for remote_file in remote_files.iter() {
                cell(ui, |ui| {
                    ui.label(remote_file.file.clone());
                    if ui.button("Download").clicked() {
                    }
                });
                count = count + 1;
                if count % 4 == 0 {
                    ui.end_row();
                }
            }
            let local_files = self.local_files.borrow();
            for local_file in local_files.iter() {
                cell(ui, |ui| {
                    ui.label(local_file.name.clone());
                    if ui.button("Stop sharing").clicked() {
                        actions.push(Action::RemoveSend(local_file.clone()));
                    }
                });
                count = count + 1;
                if count % 4 == 0 {
                    ui.end_row();
                }
            }
            cell(ui, |ui| {
                ui.label("Share new");
                if ui.button("file").clicked() {
                    let path = FileDialog::new().pick_file();
                    if let Some(path) = path {
                        actions.push(Action::AddSend(path));
                    }
                }
                ui.label("or");
                if ui.button("folder").clicked() {
                    let path = FileDialog::new().pick_folder();
                    if let Some(path) = path {
                        actions.push(Action::AddSend(path));
                    }
                }
            });
        });
        ui.heading("Send");
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
            Action::AddSend(path) => match LocalFile::new(path) {
                Ok(local_file) => self.files.add_local_file(local_file),
                Err(_) => false,
            },
            Action::RemoveSend(local_file) => self.files.remove_local_file(&local_file),
            Action::Download(file, path) => {
                self.files.add_download(file, path);
                false
            }
        }
    }
}

fn cell<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.group(|ui| {
        ui.set_width(128f32);
        ui.set_height(128f32);
        ui.vertical_centered(|ui| add_contents(ui)).inner
    })
}
