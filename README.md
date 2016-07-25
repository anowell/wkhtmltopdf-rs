# wkhtmltopdf-rs
High-level Rust bindings for wkhtmltopdf. This is a wrapper around the low-level binding provided by [libwkhtmltox-sys](https://github.com/anowell/libwkhtmltox-sys).

[Documentation](https://anowell.github.io/wkhtmltopdf-rs/wkhtmltopdf/)

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
  let mut pdfout = PdfBuilder::new()
      .orientation(Orientation::Landscape)
      .margin(Size::Inches(2))
      .title("Awesome Foo")
      .build_from_html(&html)
      .expect("failed to build pdf");

  pdfout.save("foo.pdf").expect("failed to save foo.pdf");
  println!("save generated PDF as: foo.pdf");
```

TODO:
- [ ] Error cleanup
- [ ] Support more settings, figure out why some flags don't work (like 'outlineDepth')
- [ ] Tests and better examples
- [ ] Other input sources: `Url`, `Path`, `impl Read`
- [ ] Consider extending for WkHtmlToImage

**Contributions welcome in the form of issue reports, feature requests, feed, and/or pull request.**
