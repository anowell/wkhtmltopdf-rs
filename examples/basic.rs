extern crate wkhtmltopdf;
extern crate url;
extern crate env_logger;

use wkhtmltopdf::*;

fn main() {
    env_logger::init().unwrap();
    let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");

    let html = r#"
      <html><body>
        <h1>Rust can haz PDFs</h1>
        <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
      </body></html>
    "#;

    {
      let mut pdfout = pdf_app.builder()
        .orientation(Orientation::Landscape)
        .margin(Size::Millimeters(12))
        .title("PDFs for Rust")
        .build_from_html(&html)
        .expect("failed to build pdf");

      let _ = pdfout.save("basic.pdf").expect("failed to save basic.pdf");
      println!("PDF saved as basic.pdf");
    }

    {
      let mut pdfout = pdf_app.builder()
        .orientation(Orientation::Landscape)
        .margin(Size::Millimeters(12))
        .title("Rust Website")
        .build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap())
        .expect("failed to build pdf");

      let _ = pdfout.save("basic2.pdf").expect("failed to save basic.pdf");
      println!("PDF saved as basic2.pdf");
    }
}
