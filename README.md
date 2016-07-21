# wkhtmltopdf-rs
High-level Rust bindings for wkhtmltopdf. This is a wrapper around the low-level binding provided by [libwkhtmltox-sys](https://github.com/anowell/libwkhtmltox-sys).

## Install

Install wkhtmltopdf 0.12.3 libs.

Arch: `pacman -S wkhtmltopdf`

TODO: add install for more common platforms.

Manually:
- Download [wkhtmltopdf 0.12.3](http://wkhtmltopdf.org/downloads.html)
- Install libraries (libwkhtmltox.so.0.12.3 and symlinks) into system libraries or use `LD_LIBRARY_PATH` when running your app

## WIP

This is a work-in-progress. It is kinda, sorta able to generate PDFs from HTML*. It currently works like this:

```rust
  let html = r#"<html><body><div>foo</div></body></html>"#.to_string();
  let res = PdfBuilder::from_html(html)
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
```

&ast; most of the time, in debug mode, at least on my laptop (see first TODO item)



TODO:
- [ ] Resolve soundess issues - `wkhtmltopdf_get_output` segfaults, but hacked around with weird sleep in debug mode 
- [ ] Error cleanup
- [ ] Support more settings, figure out why some flags don't work (like 'outlineDepth')
- [ ] Tests
- [ ] Other input sources: `Url`, `Path`, `impl Read`
- [ ] Better examples
- [ ] Consider extending for WkHtml

**Contributions welcome in the form of issue reports, feature requests, feed, and/or pull request.**
