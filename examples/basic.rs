extern crate wkhtmltopdf;
extern crate url;

use wkhtmltopdf::*;

fn main() {
    let html = r#"
      <html><body>
        <h1>Rust can haz PDFs</h1>
        <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
      </body></html>
    "#;

    let mut pdfout = PdfBuilder::new()
    	.orientation(Orientation::Landscape)
      .margin(Size::Millimeters(12))
      .title("PDFs for Rust")
      .build_from_html(&html)
      .expect("failed to build pdf");

    let _ = pdfout.save("basic.pdf").expect("failed to save basic.pdf");
    println!("PDF saved as basic.pdf");
}
