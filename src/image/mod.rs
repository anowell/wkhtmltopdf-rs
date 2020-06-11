//! Generate images from HTML safely using [wkhtmltoimage](http://wkhtmltoimage.org/)
//!
//! Wkhtmltoimage uses QT Webkit to render HTML for image generation.
//! This crate depends on [low-level wkhtmltoimage bindings](https://crates.io/crates/wkhtmltox-sys),
//! to provide an ergonomic API for generating images from URLs, local HTML files, or HTML strings.
//! Installing wkhtmltoimage (currently 0.12.3) is a prerequisite to using this crate.
//!
//! ## Example
//! ```no_run
//! use wkhtmltopdf::*;
//!
//! let image_app = ImageApplication::new().expect("Failed to init image application");
//! let mut imageout = image_app.builder()
//!     .format("png")
//!     .build_from_path("input.html")
//!     .expect("failed to build image");
//!
//! imageout.save("foo.image").expect("failed to save foo.image");
//! ```
//!
//! Other examples can be seen in the documentation for
//! [`imageBuilder`](struct.imageBuilder.html) methods:
//!
//! - [`build_from_url`](struct.imageBuilder.html#method.build_from_url)
//! - [`build_from_path`](struct.imageBuilder.html#method.build_from_path)
//!
//! Addtionally, the [`lowlevel`](lowlevel/index.html) module provides safe abstractions
//!   that allow full configuration of wkhtmltoimage.

use crate::error::*;
pub mod lowlevel;
use log::warn;
use lowlevel::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use url::Url;

/// Generated image output
pub struct ImageOutput<'a> {
    // slice of the data owned by the wkhtmltoimage_converter
    data: &'a [u8],
    // Don't drop the converter until data lifetime ends
    _converter: ImageConverter,
}

/// Structure for initializing the underlying wkhtmltoimage
///
/// This is effective a wrapper around `imageGuard` that provides
/// a method for instantiating one a builder
pub struct ImageApplication {
    _guard: ImageGuard,
}

impl ImageApplication {
    /// Initializes Wkhtmltoimage
    ///
    /// Wkhtmltoimage will remain initialized for this process until `imageApplication` is dropped.
    /// Wkhtmltoimage may only be initialized once per process, and
    /// and all image generation must happen from the same thread that initialized wkhtmltoimage.
    ///
    /// Subsequent attempts to initialize wkhtmltoimage will return `Error:IllegalInit`.
    pub fn new() -> Result<ImageApplication> {
        image_init().map(|guard| ImageApplication { _guard: guard })
    }

    /// Instantiate a `imageBuilder`
    ///
    /// This method borrows the `self` mutably to ensure only that one builder is active at a time which is a
    /// [basic limitation of wkhtmltoimage](https://github.com/wkhtmltoimage/wkhtmltoimage/issues/1711).
    /// Parallel execution is currently only possible by spawning multiple processes.
    pub fn builder(&self) -> ImageBuilder {
        ImageBuilder {
            gs: HashMap::new(),
            os: HashMap::new(),
        }
    }
}

/// High-level builder for generating images (initialized from `imageApplication`)
#[derive(Clone)]
pub struct ImageBuilder {
    gs: HashMap<&'static str, Cow<'static, str>>,
    os: HashMap<&'static str, Cow<'static, str>>,
}

impl ImageBuilder {
    /// JPEG image compression quality in percentage (default 94). Only used
    /// when format is 'jpg'.
    pub fn image_quality(&mut self, image_quality: u32) -> &mut ImageBuilder {
        self.gs
            .insert("imageQuality", image_quality.to_string().into());
        self
    }

    /// When outputting a PNG or SVG, make the white background transparent.
    pub fn transparent(&mut self, transparent: bool) -> &mut ImageBuilder {
        self.gs
            .insert("transparent", transparent.to_string().into());
        self
    }

    /// The output format to use, must be either "", "jpg", "png", "bmp" or "svg"
    pub fn format(&mut self, format: &str) -> &mut ImageBuilder {
        if ["", "jpg", "png", "bmp", "svg"].contains(&format) {
            self.gs.insert("fmt", format.to_string().into());
        }
        self
    }

    /// Set a global setting not explicitly supported by the imageBuilder
    ///
    /// # Safety
    ///
    /// Unsafe because values not supported by wkhtmltoimage can cause undefined behavior
    //    (e.g. segfault) in later calls.
    pub unsafe fn global_setting<S: Into<Cow<'static, str>>>(
        &mut self,
        name: &'static str,
        value: S,
    ) -> &mut ImageBuilder {
        self.gs.insert(name, value.into());
        self
    }

    /// Set an object setting not explicitly supported by the imageBuilder
    ///
    /// # Safety
    ///
    /// Unsafe because values not supported by wkhtmltoimage can cause undefined behavior
    //    (e.g. segfault) in later calls.
    pub unsafe fn object_setting<S: Into<Cow<'static, str>>>(
        &mut self,
        name: &'static str,
        value: S,
    ) -> &mut ImageBuilder {
        self.os.insert(name, value.into());
        self
    }

    /// Build a image using a URL as the source input
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::ImageApplication;
    /// let mut image_app = ImageApplication::new().expect("Failed to init image application");
    /// let mut imageout = image_app.builder()
    ///        .format("png")
    ///        .build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap())
    ///        .expect("failed to build image");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltoimage
    pub fn build_from_url<'a, 'b>(&'a mut self, url: Url) -> Result<ImageOutput<'b>> {
        let mut global = self.global_settings()?;
        unsafe {
            global.set("in", url.as_str())?;
        }
        let converter = global.create_converter();
        converter.convert()
    }

    /// Build a image using the provided HTML from a local file
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::ImageApplication;
    /// let mut image_app = ImageApplication::new().expect("Failed to init image application");
    /// let mut imageout = image_app.builder()
    ///        .format("png")
    ///        .build_from_path("/path/to/static/index.html")
    ///        .expect("failed to build image");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltoimage
    pub fn build_from_path<'a, 'b, P: AsRef<Path>>(
        &'a mut self,
        path: P,
    ) -> Result<ImageOutput<'b>> {
        let path = path.as_ref();
        // Check that the file exists - otherwise wkhtmltopdf will silently fall back
        // to trying it as a URL:
        // https://github.com/wkhtmltopdf/wkhtmltopdf/blob/5fb6a6e479409c0a270e56d852a5a9e7b2b7651b/src/lib/multipageloader.cc#L690
        if !path.is_file() {
            warn!("the file {} does not exist", path.to_string_lossy());
            return Err(Error::GlobalSettingFailure(
                "in".to_string(),
                path.to_string_lossy().to_string(),
            ));
        }
        let mut global = self.global_settings()?;
        unsafe {
            global.set("in", &path.to_string_lossy())?;
        }
        let converter = global.create_converter();
        converter.convert()
    }

    /// Use the relevant settings to construct a low-level instance of `ImageGlobalSettings`
    pub fn global_settings(&self) -> Result<ImageGlobalSettings> {
        let mut global = ImageGlobalSettings::new()?;
        for (ref name, ref val) in &self.gs {
            unsafe { global.set(name, &val) }?;
        }
        Ok(global)
    }
}

impl<'a> ImageOutput<'a> {
    // Helper to save the image output to a local file
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> io::Result<File> {
        let mut file = File::create(path)?;
        let _ = io::copy(self, &mut file)?;
        Ok(file)
    }
}

impl<'a> Read for ImageOutput<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }
}

impl<'a> std::fmt::Debug for ImageOutput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.data.fmt(f)
    }
}
