#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct RemoteApp {}

impl eframe::App for RemoteApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("REMOTE");
        });
    }
}
