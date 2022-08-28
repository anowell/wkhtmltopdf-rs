//! Generate images from HTML safely using [wkhtmltopdf](http://wkhtmltopdf.org/)
//!
//! Wkhtmltoimage uses QT Webkit to render HTML for image generation.
//! This crate depends on [low-level wkhtmltoimage bindings](https://crates.io/crates/wkhtmltox-sys),
//! to provide an ergonomic API for generating images from URLs, local HTML files, or HTML strings.
//! Installing wkhtmltoimage (currently 0.12.6) is a prerequisite to using this crate.
//!
//! ## Example
//! ```no_run
//! use wkhtmltopdf::*;
//!
//! let image_app = ImageApplication::new().expect("Failed to init image application");
//! let mut imageout = image_app.builder()
//!     .format(ImageFormat::Png)
//!     .build_from_path("input.html")
//!     .expect("failed to build image");
//!
//! imageout.save("foo.png").expect("failed to save foo.png");
//! ```
//!
//! Other examples can be seen in the documentation for
//! [`ImageBuilder`](struct.ImageBuilder.html) methods:
//!
//! - [`build_from_url`](struct.ImageBuilder.html#method.build_from_url)
//! - [`build_from_path`](struct.ImageBuilder.html#method.build_from_path)
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
/// This is effective a wrapper around `ImageGuard` that provides
/// a method for instantiating a builder
pub struct ImageApplication {
    _guard: ImageGuard,
}

impl ImageApplication {
    /// Initializes Wkhtmltoimage
    ///
    /// Wkhtmltoimage will remain initialized for this process until `ImageApplication` is dropped.
    /// Wkhtmltoimage may only be initialized once per process, and
    /// and all image generation must happen from the same thread that initialized wkhtmltoimage.
    ///
    /// Subsequent attempts to initialize wkhtmltoimage will return `Error:IllegalInit`.
    pub fn new() -> Result<ImageApplication> {
        image_init().map(|guard| ImageApplication { _guard: guard })
    }

    /// Instantiate an `ImageBuilder`
    ///
    /// This method borrows the `self` mutably to ensure only that one builder is active at a time which is a
    /// [basic limitation of wkhtmltoimage](https://github.com/wkhtmltoimage/wkhtmltoimage/issues/1711).
    /// Parallel execution is currently only possible by spawning multiple processes.
    pub fn builder(&self) -> ImageBuilder {
        ImageBuilder { gs: HashMap::new() }
    }
}

/// Image formats supported by wkhtmltoimage
pub enum ImageFormat {
    Jpg,
    Png,
    Bmp,
    Svg,
}

impl ImageFormat {
    /// Render the image format as a string to pass to wkhtmltoimage
    fn value(&self) -> &'static str {
        use ImageFormat::*;
        match self {
            Jpg => "jpg",
            Png => "png",
            Bmp => "bmp",
            Svg => "svg",
        }
    }
}

/// High-level builder for generating images (initialized from `ImageApplication`)
#[derive(Clone)]
pub struct ImageBuilder {
    gs: HashMap<&'static str, Cow<'static, str>>,
}

impl ImageBuilder {
    /// The with of the screen used to render in pixels, e.g "800"
    pub fn screen_width(&mut self, screen_width: u32) -> &mut ImageBuilder {
        self.gs
            .insert("screenWidth", screen_width.to_string().into());
        self
    }

    /* Pending https://github.com/wkhtmltopdf/wkhtmltopdf/issues/4714
    /// The with of the screen used to render in pixels, e.g "800"
    pub fn crop_left(&mut self, crop_left: u32) -> &mut ImageBuilder {
        self.gs.insert("crop.left", crop_left.to_string().into());
        self
    }

    /// The with of the screen used to render in pixels, e.g "800"
    pub fn crop_top(&mut self, crop_top: u32) -> &mut ImageBuilder {
        self.gs.insert("crop.top", crop_top.to_string().into());
        self
    }

    /// The with of the screen used to render in pixels, e.g "800"
    pub fn crop_width(&mut self, crop_width: u32) -> &mut ImageBuilder {
        self.gs.insert("crop.width", crop_width.to_string().into());
        self
    }

    /// The with of the screen used to render in pixels, e.g "800"
    pub fn crop_height(&mut self, crop_height: u32) -> &mut ImageBuilder {
        self.gs
            .insert("crop.height", crop_height.to_string().into());
        self
    }
    */

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

    /// The output format to use, valid formats are Jpg, Png, Bmp, and Svg
    pub fn format(&mut self, format: ImageFormat) -> &mut ImageBuilder {
        self.gs.insert("fmt", format.value().into());
        self
    }

    /// Set a global setting not explicitly supported by the ImageBuilder
    ///
    /// Valid settings can be found [here](https://wkhtmltopdf.org/libwkhtmltox/pagesettings.html#pageImageGlobal)
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

    /// Build an image using a URL as the source input
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::{ImageApplication, ImageFormat};
    /// let mut image_app = ImageApplication::new().expect("Failed to init image application");
    /// let mut imageout = image_app.builder()
    ///        .format(ImageFormat::Png)
    ///        .build_from_url(&"https://www.rust-lang.org/en-US/".parse().unwrap())
    ///        .expect("failed to build image");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltoimage
    pub fn build_from_url<'a, 'b>(&'a mut self, url: &Url) -> Result<ImageOutput<'b>> {
        let mut global = self.global_settings()?;
        unsafe {
            global.set("in", &*url.as_str())?;
        }
        let converter = global.create_converter(None);
        converter.convert()
    }

    /// Build an image using the provided HTML from a local file
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::{ImageApplication, ImageFormat};
    /// let mut image_app = ImageApplication::new().expect("Failed to init image application");
    /// let mut imageout = image_app.builder()
    ///        .format(ImageFormat::Png)
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
        let converter = global.create_converter(None);
        converter.convert()
    }

    /// Build an image using the provided HTML string
    ///
    /// ## Example
    /// ```no_run
    /// # use wkhtmltopdf::{ImageApplication, ImageFormat};
    /// let mut image_app = ImageApplication::new().expect("Failed to init image application");
    /// let mut imageout = image_app.builder()
    ///         .format(ImageFormat::Png)
    ///         .build_from_html("<h1>Hello World!</h1>")
    ///         .expect("failed to build image");
    /// ```
    ///
    /// This method should be safe if using only safe builder methods, or if usage
    /// of `unsafe` methods (e.g. adding custom settings) is properly handled by wkhtmltoimage
    pub fn build_from_html<'a, 'b, S: AsRef<str>>(
        &'a mut self,
        html: S,
    ) -> Result<ImageOutput<'b>> {
        let mut global = self.global_settings()?;
        unsafe {
            global.set("in", "-")?;
        }
        let converter = global.create_converter(Some(html.as_ref()));
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
    /// Save the image output to a local file
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
