#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TelemetryApp {}

impl eframe::App for TelemetryApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("TELEMETRY");
        });
    }
}
