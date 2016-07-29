extern crate wkhtmltopdf;
extern crate url;
extern crate env_logger;

use wkhtmltopdf::*;
use std::fs::File;

fn main() {
    env_logger::init().unwrap();
    let html = r#"
        <html><body>
        <h1>Rust can haz PDFs</h1>
        <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
        </body></html>
    "#;

    let mut settings = PdfBuilder::new();
    settings.orientation(Orientation::Landscape)
        .margin(Size::Millimeters(12))
        .title("PDFs for Rust");

    // It is still safest to initialize global and object settings from the builder
    //   which provides a set of known-safe settings
    let gs = settings.global_settings().expect("failed to create global settings");
    let os = settings.object_settings().expect("failed to create object settings");

    // Instead of finalizing the builder with a `build_*` method,
    //   we can create the converter manually from the global settings
    let mut c = gs.create_converter();

    // Add an html object and convert
    c.add_html_object(os, &html);
    let mut pdfout = c.convert().expect("failed to convert");

    // let mut pdfout = pdfout;
    let mut file = File::create("basic.pdf").expect("failed to create basic.pdf");
    let bytes = std::io::copy(&mut pdfout, &mut file).expect("failed to write to basic.pdf");
    println!("wrote {} bytes to file: basic.pdf", bytes);
}
