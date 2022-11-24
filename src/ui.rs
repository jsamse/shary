use super::Send;
use crate::network::NetworkHandle;
use rfd::FileDialog;
use std::path::PathBuf;
use tracing::info;

pub fn run(port: u16) {
    let options = eframe::NativeOptions::default();
    let app = App::new(port);
    eframe::run_native(&"Shary", options, Box::new(|_cc| Box::new(app)));
}

enum Action {
    AddSend(PathBuf),
    RemoveSend(String),
}

struct App {
    network_handle: NetworkHandle,
    initialized: InitializedApp,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| -> () {
            if true {
                ui.heading("Error:");
                //ui.label(format!("{}", report));
                //ui.label(format!("{}", report.root_cause()));
            }
        });
    }
}

impl App {
    fn new(port: u16) -> Self {
        let network_handle = crate::network::run(port);
        let initialized = InitializedApp {
            actions: vec![],
            sends: vec![],
        };

        Self {
            initialized,
            network_handle,
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
