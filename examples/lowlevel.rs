use env_logger;
use std::fs::File;
use wkhtmltopdf::Orientation;
use wkhtmltopdf::*;

fn main() {
    env_logger::init();
    let pdf_app = PdfApplication::new().expect("Failed to init PDF application");

    let html = r#"
        <html><body>
        <h1>Rust can haz PDFs</h1>
        <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
        <script>this.will.trigger.a.warning;</script>
        </body></html>
    "#;

    let mut settings = pdf_app.builder();
    settings
        .orientation(Orientation::Landscape)
        .margin(Size::Millimeters(12))
        .title("PDFs for Rust");

    unsafe {
        // Enables warning for JavaScript errors that may occur
        settings.object_setting("load.debugJavascript", "true");
    }

    // It is still safest to initialize global and object settings from the builder
    //   which provides a set of known-safe settings
    let gs = settings
        .global_settings()
        .expect("failed to create global settings");
    let os = settings
        .object_settings()
        .expect("failed to create object settings");

    // Instead of finalizing the builder with a `build_*` method,
    //   we can create the converter manually from the global settings
    let mut c = gs.create_converter();

    // Provides an event handling for JavaScript warnings, when debug is on
    c.set_warning_callback(Some(Box::new(|warn| {
        println!("warning: {}", warn);
    })));

    // Add an html object and convert
    c.add_html_object(os, &html);
    let mut pdfout = c.convert().expect("failed to convert");

    // let mut pdfout = pdfout;
    let mut file = File::create("basic.pdf").expect("failed to create basic.pdf");
    let bytes = std::io::copy(&mut pdfout, &mut file).expect("failed to write to basic.pdf");
    println!("wrote {} bytes to file: basic.pdf", bytes);
}
