use crate::common::{AppState, AppStateData, RemoteFile};
use rfd::FileDialog;
use std::path::PathBuf;

pub fn run(state: AppState) {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        &"Shary",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            state.add_listener(move |_| {
                ctx.request_repaint();
            });
            let app = App { state };
            Box::new(app)
        }),
    );
}

enum Action {
    AddSend(PathBuf),
    RemoveSend(String),
    Download(RemoteFile, PathBuf),
}

struct App {
    state: AppState,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let data = self.state.data();
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            let actions = self.draw(ui, &data);
            let updated = actions
                .into_iter()
                .map(|a| self.handle_action(&data, a))
                .any(|a| a);
        });
    }
}

impl App {
    fn draw(&mut self, ui: &mut egui::Ui, data: &AppStateData) -> Vec<Action> {
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
        if data.local_files.len() > 0 {
            ui.add_space(16f32);
        }
        for file in data.local_files.iter() {
            if ui.button(file.name.clone()).clicked() {
                actions.push(Action::RemoveSend(file.name.clone()));
            }
        }
        ui.add_space(16f32);
        ui.separator();
        ui.heading("Receive");

        for file in data.remote_files.iter() {
            if ui.button(file.file.clone()).clicked() {
                if let Some(path) = FileDialog::new().pick_folder() {
                    actions.push(Action::Download(file.clone(), path));
                }
            }
        }

        actions
    }

    fn handle_action(&mut self, data: &AppStateData, action: Action) -> bool {
        match action {
            Action::AddSend(path) => self.state.add_local_file(path),
            Action::RemoveSend(name) => {
                self.state.remove_local_file(&name);
                true
            }
            Action::Download(file, path) => {
                //self.network.download(&file, path);
                false
            }
        }
    }
}
