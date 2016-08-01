# wkhtmltopdf-rs
High-level Rust bindings for wkhtmltopdf. This is a wrapper around the low-level binding provided by [libwkhtmltox-sys](https://github.com/anowell/libwkhtmltox-sys).

Resource  | Link    
----- | -----
Crate | [![Crates.io](https://img.shields.io/crates/v/rustc-serialize.svg?maxAge=2592000)](https://crates.io/crates/wkhtmltopdf)
Documentation | [Cargo docs](https://anowell.github.io/wkhtmltopdf-rs/wkhtmltopdf/)
Upstream | [wkhtmltopdf.org](http://wkhtmltopdf.org/)

-----

This crate aims to provide full configuration of wkhtmltopdf with safe, ergonomic Rust.
Wkhtmltopdf has several non-obvious limitations (mostly caused by Qt).
that make it very easy to cause undefined behavior with the C bindings.
Two such limitations that greatly impact the API are:

1. Wkhtmltopdf initialization can only occur once per process; deinitialization does make it safe to reuse
2. PDF generation must always occur on the thread that initialized wkhtmltopdf

This crate should make it impossible to break those rules in safe code. If you need parallel PDF generation,
you will need to spawn/fork processes to do so. Such an abstraction would be a welcome addition to this crate.

## Install

Install [wkhtmltopdf](http://wkhtmltopdf.org/downloads.html) 0.12.3 (libs and includes).

TODO: Add platform-relevant instructions to replace these manual install:
- `lib/*.so` files to /usr/lib
- `include/wkhtmltopdf` dir to `/usr/include/wkhtmltopdf`

## Usage

Basic usage looks like this:

```rust
  let html = r#"<html><body><div>foo</div></body></html>"#;
  let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");
  let mut pdfout = pdf_app.builder()
      .orientation(Orientation::Landscape)
      .margin(Size::Inches(2))
      .title("Awesome Foo")
      .build_from_html(&html)
      .expect("failed to build pdf");

  pdfout.save("foo.pdf").expect("failed to save foo.pdf");
  println!("generated PDF saved as: foo.pdf");
```

## Build

As long as the includes are installed (e.g. `pdf.h`), then it's all cargo:

```
cargo build
cargo test
```

Note: tests have to be combined into a single test case because we can only init `PdfApplication` once, and it is `!Send`/`!Sync`.
So the preference going forward will be to test with lots of good examples.

**Contributions welcome in the form of issue reports, feature requests, feedback, and/or pull request.**
