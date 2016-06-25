extern crate wkhtmltopdf;
extern crate url;
use wkhtmltopdf::*;
use std::fs::File;
use std::io::Write;
use url::Url;

fn main() {
  let html = r#"
    <html><body>
      <div>foo</div>
      <img src="https://www.google.com.mx/images/branding/googlelogo/2x/googlelogo_color_272x92dp.png">
    </body></html>
  "#;
  let res = PdfBuilder::from_html(html.to_string())
    .configure(PdfSettings {
      orientation: Orientation::Landscape,
      margin: Margin::all(Size::Inches(2)),
      title: Some("Awesome Foo".into()),
      .. Default::default()
    })
    .build()
    .expect("failed to build pdf");
  let mut file = File::create("foo.pdf").expect("failed to create foo.pdf");
  println!("writing {} bytes to file: foo.pdf", res.len());
  file.write_all(&res).expect("failed to write to foo.pdf");

  // let res = PdfBuilder::from_url(Url::parse("http://google.com").expect("invalid url")).build().expect("failed to build pdf");
  // let mut file = File::create("google.pdf").expect("failed to create foo.pdf");
  // println!("writing {} bytes to file: foo.pdf", res.len());
  // file.write_all(&res).expect("failed to write to foo.pdf");
}
