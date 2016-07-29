# wkhtmltopdf-rs
High-level Rust bindings for wkhtmltopdf. This is a wrapper around the low-level binding provided by [libwkhtmltox-sys](https://github.com/anowell/libwkhtmltox-sys).

[Documentation](https://anowell.github.io/wkhtmltopdf-rs/wkhtmltopdf/)

## Install

Install [wkhtmltopdf](http://wkhtmltopdf.org/downloads.html) 0.12.3 (libs and includes).

TODO: Add platform-relevant instructions to replace these manual install:
- `lib/*.so` files to /usr/lib
- `include/wkhtmltopdf` dir to `/usr/include/wkhtmltopdf`

## Usage

Basic usage looks like this:

```rust
  let html = r#"<html><body><div>foo</div></body></html>"#;
  let mut pdfout = PdfBuilder::new()
      .orientation(Orientation::Landscape)
      .margin(Size::Inches(2))
      .title("Awesome Foo")
      .build_from_html(&html)
      .expect("failed to build pdf");

  pdfout.save("foo.pdf").expect("failed to save foo.pdf");
  println!("generated PDF saved as: foo.pdf");
```

## TODO
- [ ] Support more settings, figure out why some flags don't work (like 'outlineDepth')
- [ ] Tests and better examples

**Contributions welcome in the form of issue reports, feature requests, feedback, and/or pull request.**
