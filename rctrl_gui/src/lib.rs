use eframe;
use wasm_bindgen::prelude::wasm_bindgen;

mod connection;
mod gui;
mod logger;
mod remote;
mod telemetry;

/// Main loop of the application.
#[wasm_bindgen(start)]
pub fn main() -> Result<(), wasm_bindgen::JsValue> {
    // Set panic hook for addition debug information in web browser
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        "rctrl_gui_canvas",
        web_options,
        Box::new(|cc| Box::new(gui::Gui::new(cc))),
    )
    .expect("Failed to start rCTRL GUI");

    Ok(())
}
