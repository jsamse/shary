use crate::{
    common::{Files, LocalFile, RemoteFile},
    ok_or_continue, some_or_continue,
};
use eframe::epaint::text::TextWrapping;
use egui::{InnerResponse, Ui, text::LayoutJob, TextFormat};
use rfd::FileDialog;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::watch;

const SIZE: egui::Vec2 = egui::Vec2 {
    x: 384f32,
    y: 384f32,
};
const GRID_COLUMNS: i32 = 3;

pub fn run(files: Arc<Files>) {
    let mut options = eframe::NativeOptions::default();
    options.drag_and_drop_support = true;
    options.min_window_size = Some(SIZE);
    options.max_window_size = Some(SIZE);
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

        let columns = GRID_COLUMNS as f32;
        let spacing = ui.spacing();
        let item_spacing = spacing.item_spacing.x * (columns - 1f32);
        let width = (ui.available_width() - item_spacing) / columns;

        egui::Grid::new("grid")
            .min_col_width(width)
            .max_col_width(width)
            .show(ui, |ui| {
                let mut count = 0;
                for remote_file in remote_files.iter() {
                    cell(ui, |ui| {
                        let mut job = LayoutJob::single_section(remote_file.file.clone(), TextFormat::default());
                        job.wrap = TextWrapping {
                            max_rows: 3,
                            break_anywhere: true,
                            overflow_character: Some('…'),
                            max_width: ui.available_width(),
                        };
                        ui.label(job);
                        ui.add_space(32f32);
                        if ui.button("Download").clicked() {
                            let path = FileDialog::new().pick_folder();
                            if let Some(path) = path {
                                actions.push(Action::Download(remote_file.clone(), path))
                            }
                        }
                    });
                    count = count + 1;
                    if count % GRID_COLUMNS == 0 {
                        ui.end_row();
                    }
                }
                let local_files = self.local_files.borrow();
                for local_file in local_files.iter() {
                    cell(ui, |ui| {
                        let mut job = LayoutJob::single_section(local_file.name.clone(), TextFormat::default());
                        job.wrap = TextWrapping {
                            max_rows: 3,
                            break_anywhere: true,
                            overflow_character: Some('…'),
                            max_width: ui.available_width(),
                        };
                        ui.label(job);
                        ui.add_space(8f32);
                        if ui.button("Stop sharing").clicked() {
                            actions.push(Action::RemoveSend(local_file.clone()));
                        }
                    });
                    count = count + 1;
                    if count % GRID_COLUMNS == 0 {
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
                    ui.label("or drag and drop");
                });
            });
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
        let width = ui.available_width();
        ui.set_width(width);
        ui.set_height(width);
        ui.vertical_centered(|ui| {
            ui.set_height(ui.available_height());
            add_contents(ui)
        }).inner
    })
}
