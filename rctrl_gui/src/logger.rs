#[derive(serde::Deserialize, serde::Serialize)]
pub struct LoggerApp {
    logs: Vec<String>,
    display_error: bool,
    display_debug: bool,
}

impl Default for LoggerApp {
    fn default() -> Self {
        Self {
            logs: Vec::<String>::new(),
            display_error: true,
            display_debug: false,
        }
    }
}

impl LoggerApp {
    pub fn log(&mut self, msg: String) {
        self.logs.push(msg);
    }
}

impl eframe::App for LoggerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(200.0)
            .show(&ctx, |ui| {
                ui.add_space(4.0);

                egui::menu::bar(ui, |ui| {
                    ui.label("Filter Severity: ");
                    if ui
                        .selectable_label(self.display_error, egui::RichText::new("⚠  20"))
                        .clicked()
                    {
                        self.display_error = !self.display_error;
                    }

                    if ui
                        .selectable_label(self.display_debug, egui::RichText::new("ℹ  103"))
                        .clicked()
                    {
                        self.display_debug = !self.display_debug;
                    }
                });

                ui.separator();

                let text_style = egui::TextStyle::Body;
                let row_height = ui.text_style_height(&text_style);
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .always_show_scroll(true)
                    .stick_to_bottom(true)
                    .show_rows(ui, row_height, self.logs.len(), |ui, row_range| {
                        for row in row_range {
                            let text = format!("This is row {}", row + 1);
                            ui.label(text);
                        }
                    });
            });
    }
}
