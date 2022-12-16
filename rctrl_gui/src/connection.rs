use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use std::time::Duration;

pub trait Connection {
    /// Pop oldest message in queue, if there are any.
    fn read(&mut self) -> Option<WsMessage>;

    /// Draw details of connection for use within the ConnectionManager.
    fn draw_connection_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui);
}

/// Panel to manage connections to all data sources.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ConnectionManager {
    open: bool,
    pub ws_remote: WebSocketConnection,
    pub ws_telemetry: WebSocketConnection,
}

impl ConnectionManager {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ws_remote = WebSocketConnection::new("Remote", "ws://127.0.0.1:9090", &cc.egui_ctx);
        let ws_telemetry = WebSocketConnection::new("Remote", "", &cc.egui_ctx);

        Self {
            open: false,
            ws_remote: ws_remote,
            ws_telemetry: ws_telemetry,
        }
    }

    fn connection_vec(&mut self) -> Vec<&mut dyn Connection> {
        vec![
            (&mut self.ws_remote as &mut dyn Connection),
            (&mut self.ws_telemetry as &mut dyn Connection),
        ]
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle_is_open(&mut self) {
        self.open = !self.open;
    }

    //pub fn read(&mut self) {
    //    for connection in self.connection_vec() {
    //        connection.read();
    //    }
    //}

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.open {
            egui::SidePanel::left("connections")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("🖧  Connections");
                    });

                    for connection in self.connection_vec() {
                        connection.draw_connection_panel(ctx, ui);
                    }
                });
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct WebSocketConnection {
    name: String,
    url: String,

    #[serde(skip)]
    ws_sender: Option<WsSender>,
    #[serde(skip)]
    ws_receiver: Option<WsReceiver>,
    #[serde(skip)]
    last_rx: Option<f64>,
}

impl Connection for WebSocketConnection {
    fn read(&mut self) -> Option<WsMessage> {
        let ws_event = self
            .ws_receiver
            .as_ref()
            .and_then(|ws_receiver| ws_receiver.try_recv())?;

        self.last_rx = Some(js_sys::Date::now());

        match ws_event {
            WsEvent::Opened => {
                tracing::info!("WebSocket connection {} opened", &self.name);
                return None;
            }
            WsEvent::Message(msg) => Some(msg),
            WsEvent::Error(e) => {
                tracing::error!("WebSocket read error on {} connection: {}", &self.name, e);
                return None;
            }
            WsEvent::Closed => {
                tracing::info!("WebSocket connection {} closed", &self.name);
                return None;
            }
        }
    }

    fn draw_connection_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.add_space(20.0);

        ui.heading(&self.name);

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("WebSocket:");
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                match self.ws_sender.is_some() && self.ws_receiver.is_some() {
                    true => ui.add(egui::TextEdit::singleline(&mut self.url.as_str())),
                    false => ui.add(egui::TextEdit::singleline(&mut self.url)),
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Status:");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                match self.ws_sender.is_some() && self.ws_receiver.is_some() {
                    true => {
                        ui.add(
                            egui::Button::new("CONNECTED")
                                .sense(egui::Sense::hover())
                                .fill(egui::Color32::DARK_GREEN),
                        );
                    }
                    false => {
                        ui.add(
                            egui::Button::new("DISCONNECTED")
                                .sense(egui::Sense::hover())
                                .fill(egui::Color32::DARK_RED),
                        );
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label("Last Rx:");

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::TOP),
                |ui| match self.last_rx {
                    Some(last_rx) => {
                        let elapsed_time =
                            Duration::from_millis((js_sys::Date::now() - last_rx) as u64);
                        ui.label(format!("{:?} ago", elapsed_time))
                    }
                    None => ui.label("N/A"),
                },
            );
        });

        ui.vertical_centered_justified(|ui| {
            match self.ws_sender.is_some() && self.ws_receiver.is_some() {
                true => {
                    if ui.add(egui::Button::new("Disconnect")).clicked() {
                        self.disconnect();
                    }
                }
                false => {
                    if ui.add(egui::Button::new("Connect")).clicked() {
                        self.connect(ctx);
                    };
                }
            }
        });
    }
}

impl WebSocketConnection {
    pub fn new(name: &str, url: &str, ctx: &egui::Context) -> Self {
        let mut ws = Self {
            name: name.to_string(),
            url: url.to_string(),
            ws_sender: None,
            ws_receiver: None,
            last_rx: None,
        };

        ws.connect(ctx);

        return ws;
    }

    fn connect(&mut self, ctx: &egui::Context) {
        let ctx_c = ctx.clone();
        let wakeup = move || ctx_c.request_repaint();

        match ewebsock::connect_with_wakeup(&self.url, wakeup) {
            Ok((ws_sender, ws_receiver)) => {
                self.ws_sender = Some(ws_sender);
                self.ws_receiver = Some(ws_receiver);
            }
            Err(error) => {
                tracing::error!("{} failed to connect to {}: {}", self.name, "url", error);
            }
        }
    }

    /// Disconnect from WebSocket
    fn disconnect(&mut self) {
        self.ws_sender = None;
        self.ws_receiver = None;
    }
}
