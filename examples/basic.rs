extern crate wkhtmltopdf;
extern crate url;

use wkhtmltopdf::*;
use std::fs::File;

fn main() {
  let html = r#"
    <html><body>
      <h1>Rust can haz PDFs</h1>
      <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
    </body></html>
  "#;
  let settings = PdfSettings {
    orientation: Orientation::Landscape,
    margin: Margin::all(Size::Millimeters(12)),
    title: Some("PDFs for Rust".into()),
    .. Default::default()
  };
  let mut builder = PdfBuilder::from_html(html.to_string());
  let mut pdfout = builder.configure(settings)
    .build()
    .expect("failed to build pdf");

  // let mut pdfout = pdfout;
  let mut file = File::create("basic.pdf").expect("failed to create basic.pdf");
  let bytes = std::io::copy(&mut pdfout, &mut file).expect("failed to write to basic.pdf");
  println!("wrote {} bytes to file: basic.pdf", bytes);

  // let res = PdfBuilder::from_url(Url::parse("http://google.com").expect("invalid url")).build().expect("failed to build pdf");
  // let mut file = File::create("google.pdf").expect("failed to create foo.pdf");
  // println!("writing {} bytes to file: foo.pdf", res.len());
  // file.write_all(&res).expect("failed to write to foo.pdf");
}
