//! Generate PDFs from HTML safely using [wkhtmltopdf](http://wkhtmltopdf.org/)
//!
//! Wkhtmltopdf uses QT Webkit to render HTML for PDF generation.
//! This crate depends on [low-level wkhtmltopdf bindings](https://crates.io/crates/wkhtmltox-sys),
//! to provide an ergonomimc API for generating PDFs from URLs, local HTML files, or HTML strings.
//! Installing wkhtmltopdf (currently 0.12.3) is a prerequisite to using this crate.
//!
//! ## Example
//! ```no_run
//! use wkhtmltopdf::*;
//!
//! let html = r#"<html><body><div>foo</div></body></html>"#;
//! let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");
//! let mut pdfout = pdf_app.builder()
//!     .orientation(Orientation::Landscape)
//!     .margin(Size::Inches(2))
//!     .title("Awesome Foo")
//!     .build_from_html(&html)
//!     .expect("failed to build pdf");
//!
//! pdfout.save("foo.pdf").expect("failed to save foo.pdf");
//! ```
//!
//! Other examples can be seen in the documentation for
//! [`PdfBuilder`](struct.PdfBuilder.html) methods:
//!
//! - [`build_from_html`](struct.PdfBuilder.html#method.build_from_html)
//! - [`build_from_url`](struct.PdfBuilder.html#method.build_from_url)
//! - [`build_from_path`](struct.PdfBuilder.html#method.build_from_path)
//!
//! Addtionally, the [`lowlevel`](lowlevel/index.html) module provides safe abstractions
//!   that allow full configuration of wkhtmltopdf.
extern crate wkhtmltox_sys;
extern crate url;
extern crate thread_id;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

pub mod lowlevel;

mod error;
pub use error::*;

use std::path::Path;
use url::Url;
use std::io::{self, Read};
use std::collections::HashMap;
use std::borrow::Cow;
use std::fs::File;
use lowlevel::*;


/// Generated PDF output
pub struct PdfOutput<'a> {
    // slice of the data owned by the wkhtmltopdf_converter
    data: &'a [u8],
    // Don't drop the converter until data lifetime ends
    _converter: PdfConverter,
}

/// Physical size of the paper
#[derive(Debug)]
pub enum PageSize {
    A1, A2, A3, A4, A5, A6, A7, A8, A9,
    B0, B1, B2, B3, B4, B5, B6, B7, B8, B9, B10,
    C5E, Comm10E, DLE, Executive, Folio, Ledger, Legal, Letter, Tabloid,

    /// Custom paper size: (width, height)
    Custom(Size, Size)
}

impl PageSize {
    fn value(&self) -> Cow<'static, str> {
        // TODO: srsly, this should be a macro
        use PageSize::*;
        match *self {
            A1 => "A1", A2 => "A2", A3 => "A3", A4 => "A4", A5 => "A5",
            A6 => "A6", A7 => "A7", A8 => "A8", A9 => "A9",
            B0 => "B0", B1 => "B1", B2 => "B2", B3 => "B3", B4 => "B4", B5 => "B5",
            B6 => "B6", B7 => "B7", B8 => "B8", B9 => "B9", B10 => "B10",
            C5E => "C5E", Comm10E => "Comm10E", DLE => "DLE", Executive => "Executive", Folio => "Folio",
            Ledger => "Ledger", Legal => "Legal", Letter => "Letter", Tabloid => "Tabloid",
            Custom(_,_) => "Custom"
        }.into()
    }
}

/// Unit-aware sizes
#[derive(Debug, Clone)]
pub enum Size { Millimeters(u32), Inches(u32) }
impl Size {
    fn value(&self) -> String {
        match self {
            &Size::Millimeters(ref n) => format!("{}mm", n),
            &Size::Inches(ref n) => format!("{}in", n),
        }
    }
}

/// PDF Orientation
#[derive(Debug)]
pub enum Orientation { Landscape, Portrait }

/// PDF Margins
#[derive(Debug)]
pub struct Margin {
    pub top: Size,
    pub bottom: Size,
    pub left: Size,
    pub right: Size,
}

impl From<Size> for Margin {
    /// Performs the conversion using `size` for all margins
    fn from(size: Size) -> Margin {
        Margin{ top: size.clone(), bottom: size.clone(), left: size.clone(), right: size.clone() }
    }
}

impl From<(Size, Size)> for Margin {
    /// Performs the converstion to margins from an ordered tuple representing: (top & bottom, left & right)
    fn from(sizes: (Size, Size)) -> Margin {
        Margin{ top: sizes.0.clone(), bottom: sizes.0.clone(), left: sizes.1.clone(), right: sizes.1.clone() }
    }
}

impl From<(Size, Size, Size)> for Margin {
    /// Performs the converstion to margins from an ordered tuple representing: (top, left & right, bottom)
    fn from(sizes: (Size, Size, Size)) -> Margin {
        Margin{ top: sizes.0.clone(), bottom: sizes.2.clone(), left: sizes.1.clone(), right: sizes.1.clone() }
    }
}

impl From<(Size, Size, Size, Size)> for Margin {
    /// Performs the converstion to margins from an ordered tuple representing: (top, right, bottom, left)
    fn from(sizes: (Size, Size, Size, Size)) -> Margin {
        Margin{ top: sizes.0.clone(), bottom: sizes.2.clone(), left: sizes.3.clone(), right: sizes.1.clone() }
    }
}

/// Structure for initializing the underlying wkhtmltopdf
///
/// This is effective a wrapper around `PdfGuard` that provides
/// a method for instantiating one a builder
pub struct PdfApplication {
    _guard: PdfGuard
}

impl PdfApplication {
    /// Initializes Wkhtmltopdf
    ///
    /// Wkhtmltopdf will remain initialized for this process until `PdfApplication` is dropped.
    /// Wkhtmltopdf may only be initialized once per process, and
    /// and all PDF generation must happen from the same thread that initialized wkhtmltopdf.
    ///
    /// Subsequent attempts to initialize wkhtmltopdf will return `Error:IllegalInit`.
    pub fn new() -> Result<PdfApplication> {
        pdf_init().map( |guard|
            PdfApplication { _guard: guard }
        )
    }

    /// Instantiate a `PdfBuilder`
    ///
    /// This method borrows the `self` mutably to ensure only that one builder is active at a time which is a
    /// [basic limitation of wkhtmltopdf](https://github.com/wkhtmltopdf/wkhtmltopdf/issues/1711).
    /// Parallel execution is currently only possible by spawning multiple processes.
    pub fn builder(&mut self) -> PdfBuilder {
        PdfBuilder {
            gs: HashMap::new(),
            os: HashMap::new(),
        }
    }
}

/// High-level builder for generating PDFs (initialized from `PdfApplication`)
#[derive(Clone)]
pub struct PdfBuilder {
    gs: HashMap<&'static str, Cow<'static, str>>,
    os: HashMap<&'static str, Cow<'static, str>>,
}

impl PdfBuilder {
    /// The paper size of the output document (default A4)
    pub fn page_size(&mut self, page_size: PageSize) -> &mut PdfBuilder {
        match page_size {
            PageSize::Custom(ref w, ref h) => {
                self.gs.insert("size.width", w.value().into());
                self.gs.insert("size.height", h.value().into());
            },
            _ => {
                self.gs.insert("size.pageSize", page_size.value().into());
            }
        };
        self
    }

    /// Size of the page margins (default 10mm on all sides)
    ///
    /// Note: `Into<Margin>` is also implement for tuples of Margin elements
    ///   to provide CSS-like shorthand for setting each margin
    pub fn margin<M: Into<Margin>>(&mut self, margin: M) -> &mut PdfBuilder {
        let m = margin.into();
        self.gs.insert("margin.top", m.top.value().into());
        self.gs.insert("margin.bottom", m.bottom.value().into());
        self.gs.insert("margin.left", m.left.value().into());
        self.gs.insert("margin.right", m.right.value().into());
        self
    }

    /// The orientation of the output document (default portrait)
    pub fn orientation(&mut self, orientation: Orientation) -> &mut PdfBuilder {
        let value = match orientation {
            Orientation::Landscape => "Landscape",
            Orientation::Portrait => "Portrait",
        };
        self.gs.insert("orientation", value.into());
        self
    }

    /// What dpi should we use when printin (default 72)
    pub fn dpi(&mut self, dpi: u32) -> &mut PdfBuilder {
        self.gs.insert("dpi", dpi.to_string().into());
        self
    }

    /// JPEG image compression quality in percentage (default 94)
    pub fn image_quality(&mut self, image_quality: u32) -> &mut PdfBuilder {
        self.gs.insert("imageQuality", image_quality.to_string().into());
        self
    }

    /// Title of the output document (default none)
    pub fn title(&mut self, title: &str) -> &mut PdfBuilder {
        self.gs.insert("documentTitle", title.to_string().into());
        self
    }

    /// Enabled generating an outline (table of contents) in the sidebar with a specified depth (default 4)
    ///
    /// Note: despite being a documented in wkhtmltopdf, the outline depth is not currently configurable
    ///   due to [this upstream issue](https://github.com/wkhtmltopdf/wkhtmltopdf/issues/3055).
    ///   However, it can enable and disable the outline, and when the upstream issue is resolved,
    ///   this method will be updated to also set the outline depth.
    pub fn outline(&mut self, outline_depth: Option<u32>) -> &mut PdfBuilder {
        match outline_depth {
            Some(_depth) => {
                self.gs.insert("outline", "true".into());
                // Uncomment when upstream 3055 is resolved
                // self.gs.insert("outlineDepth", depth.to_string().into());
            },
            None => {
                self.gs.insert("outline", "false".into());
            }
        }
        self
    }

    /// Set a global setting not explicitly supported by the PdfBuilder
    ///
    /// Unsafe because values not supported by wkhtmltopdf can cause undefined behavior
    //    (e.g. segfault) in later calls.
    pub unsafe fn global_setting<S: Into<Cow<'static, str>>>(&mut self, name: &'static str, value: S) -> &mut PdfBuilder {
        self.gs.insert(name, value.into());
        self
    }

    /// Set an object setting not explicitly supported by the PdfBuilder
    ///
    /// Unsafe because values not supported by wkhtmltopdf can cause undefined behavior
    //    (e.g. segfault) in later calls.
    pub unsafe fn object_setting<S: Into<Cow<'static, str>>>(&mut self, name: &'static str, value: S) -> &mut PdfBuilder {
        self.os.insert(name, value.into());
        self
    }

    /// Build a PDF using a URL as the source input
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::PdfApplication;
    /// let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");
    /// let mut pdfout = pdf_app.builder()
    ///        .build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap())
    ///        .expect("failed to build pdf");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltopdf
    pub fn build_from_url<'a, 'b>(&'a mut self, url: Url) -> Result<PdfOutput<'b>> {
        let global = try!(self.global_settings());
        let object = try!(self.object_settings());
        let mut converter = global.create_converter();
        converter.add_page_object(object, url.as_str());
        converter.convert()
    }

    /// Build a PDF using the provided HTML from a local file
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::PdfApplication;
    /// let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");
    /// let mut pdfout = pdf_app.builder()
    ///        .build_from_path("/path/to/static/index.html")
    ///        .expect("failed to build pdf");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltopdf
    pub fn build_from_path<'a, 'b, P: AsRef<Path>>(&'a mut self, path: P) -> Result<PdfOutput<'b>> {
        let global = try!(self.global_settings());
        let object = try!(self.object_settings());
        let mut converter = global.create_converter();
        converter.add_page_object(object, &path.as_ref().to_string_lossy());
        converter.convert()
    }

    /// Build a PDF using the provided HTML source input
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::PdfApplication;
    /// let mut pdf_app = PdfApplication::new().expect("Failed to init PDF application");
    /// let html = r#"foo<b>bar</b>"#;
    /// let mut pdfout = pdf_app.builder()
    ///        .build_from_html(&html)
    ///        .expect("failed to build pdf");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltopdf
    pub fn build_from_html<'a, 'b, S: AsRef<str>>(&'a mut self, html: S) -> Result<PdfOutput<'b>> {
        let global = try!(self.global_settings());
        let object = try!(self.object_settings());
        let mut converter = global.create_converter();
        converter.add_html_object(object, html.as_ref());
        converter.convert()
    }

    /// Use the relevant settings to construct a low-level instance of `PdfGlobalSettings`
    pub fn global_settings(&self) -> Result<PdfGlobalSettings> {
        let mut global = try!(PdfGlobalSettings::new());
        for (ref name, ref val) in &self.gs {
            try!( unsafe { global.set(name, &val) } );
        }
        Ok(global)
    }

    /// Use the relevant settings to construct a low-level instance of `PdfObjectSettings`
    pub fn object_settings(&self) -> Result<PdfObjectSettings> {
        let mut object = PdfObjectSettings::new();
        for (ref name, ref val) in &self.os {
            try!( unsafe { object.set(name, &val) } );
        }
        Ok(object)
    }
}

impl <'a> PdfOutput<'a> {
    // Helper to save the PDF output to a local file
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> io::Result<File> {
        let mut file = try!(File::create(path));
        let _ = try!(io::copy(self, &mut file));
        Ok(file)
    }
}

impl <'a> Read for PdfOutput<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }
}

impl <'a> std::fmt::Debug for PdfOutput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    use super::*;

    #[test]
    fn one_test_to_rule_them_all() {
        // Has to be a single test because PdfApplication can only be initialized once and is !Sync/!Send
        let _ = env_logger::init();
        let mut pdf_app = PdfApplication::new().expect("Failed to init PDF Application");

        {
            // Test building PDF from HTML
            let res = pdf_app.builder().build_from_html("basic <b>from</b> html");
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        {
            // Test building PDF from URL
            let res = pdf_app.builder().build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap());
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }
    }
}

