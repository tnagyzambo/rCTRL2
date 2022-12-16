use crate::connection::{Connection, ConnectionManager};
use crate::logger::LoggerApp;
use crate::remote::RemoteApp;
use crate::telemetry::TelemetryApp;
use bincode;
use eframe::egui;
use ewebsock::WsMessage;
use rctrl_api::remote::Data;
use tracing::{event, Level};

/// Main GUI data structure.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Gui {
    connections: ConnectionManager,
    selected_anchor: String,
    remote: RemoteApp,
    telemetry: TelemetryApp,
    logger: LoggerApp,
}

impl Gui {
    /// Initialize before first frame is drawn.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        //let data_remote = Rc::new(RefCell::new(Data::default()));

        Self {
            connections: ConnectionManager::new(cc),
            logger: LoggerApp::default(),

            selected_anchor: String::default(),
            remote: RemoteApp::default(),
            telemetry: TelemetryApp::default(),
        }
    }

    /// Helper function to clean up draw code.
    /// Return apps as Vec of (app_name, app_anchor_name, &mut app as &mut dyn efram::App).
    fn app_vec(&mut self) -> Vec<(&str, &str, &mut dyn eframe::App)> {
        vec![
            (
                "Remote Connection",
                "remote",
                &mut self.remote as &mut dyn eframe::App,
            ),
            (
                "Telemetry",
                "telemetry",
                &mut self.telemetry as &mut dyn eframe::App,
            ),
        ]
    }

    /// Show selected application, defaults to RemoteApp.
    fn show_selected_app(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut found_anchor = false;
        let selected_anchor = self.selected_anchor.clone();

        for (_name, anchor, app) in self.app_vec() {
            if anchor == selected_anchor || ctx.memory().everything_is_visible() {
                app.update(ctx, frame);
                found_anchor = true;
            }
        }

        // If app cannot be found, default to RemoteApp
        // Will be drawn on next call of show_selected_app()
        if !found_anchor {
            self.selected_anchor = "remote".into();
        }
    }
}

impl eframe::App for Gui {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for any messages over open connections
        match self.connections.ws_remote.read() {
            Some(msg) => match msg {
                WsMessage::Binary(data) => match bincode::deserialize::<Data>(&data[..]) {
                    Ok(data) => self.remote.data = data,
                    Err(e) => event!(Level::ERROR, "{} {:?}", e, data),
                },
                _ => (),
            },
            None => (),
        }
        //self.connections.read();

        // Draw top menu ribbon
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.add_space(4.0);

            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);

                ui.separator();

                if ui
                    .selectable_label(self.connections.is_open(), "🖧  Connections")
                    .clicked()
                {
                    self.connections.toggle_is_open();
                }

                ui.separator();

                // Add all apps to top ribbon
                ui.horizontal(|ui| {
                    let mut selected_anchor = self.selected_anchor.clone();
                    for (name, anchor, _app) in self.app_vec() {
                        if ui
                            .selectable_label(selected_anchor == anchor, name)
                            .clicked()
                        {
                            selected_anchor = anchor.to_owned();
                            ui.output().open_url(format!("#{}", anchor));
                        }
                    }
                    self.selected_anchor = selected_anchor;
                });
            });

            ui.add_space(2.0);
        });

        // Draw logger
        self.logger.update(ctx, frame);

        // Draw connections manager
        self.connections.update(ctx, frame);

        // Draw selected app
        self.show_selected_app(ctx, frame);
    }
}
