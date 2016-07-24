# wkhtmltopdf-rs
High-level Rust bindings for wkhtmltopdf. This is a wrapper around the low-level binding provided by [libwkhtmltox-sys](https://github.com/anowell/libwkhtmltox-sys).

## Install

Install wkhtmltopdf 0.12.3 libs.

**Manually:**
- Download [wkhtmltopdf 0.12.3](http://wkhtmltopdf.org/downloads.html)
- Install libraries and includes

TODO: Add platform-relevant instructions

## Usage

This is a work-in-progress, but basic functionality should be working:

```rust
  let html = r#"<html><body><div>foo</div></body></html>"#.to_string();
  let mut pdf = PdfBuilder::from_html(html)
    .configure(PdfSettings {
      orientation: Orientation::Landscape,
      margin: Margin::all(Size::Inches(2)),
      title: Some("Awesome Foo".into()),
      .. Default::default()
    })
    .build()
    .expect("failed to build pdf");

  let mut file = File::create("foo.pdf").expect("failed to create foo.pdf");
  let bytes = std::io::copy(&mut pdf, &mut file).expect("failed to write to foo.pdf");
  println!("wrote {} bytes to file: foo.pdf", bytes);
```

TODO:
- [ ] Error cleanup
- [ ] Support more settings, figure out why some flags don't work (like 'outlineDepth')
- [ ] Tests and better examples
- [ ] Other input sources: `Url`, `Path`, `impl Read`
- [ ] Consider extending for WkHtmlToImage

**Contributions welcome in the form of issue reports, feature requests, feed, and/or pull request.**
