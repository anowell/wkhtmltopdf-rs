extern crate libwkhtmltox_sys as libwkhtmltox;
extern crate url;

#[macro_use]
extern crate lazy_static;

use std::sync::Mutex;
use libwkhtmltox::*;
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_int, c_uchar};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use url::Url;
use std::fmt;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::borrow::Cow;

lazy_static! {
    // Globally count wkhtmltopdf handles so we can safely init/deinit the underlying wkhtmltopdf singleton
    static ref WKHTMLTOPDF_GLOBAL_COUNT: Mutex<u32> = Mutex::new(0);

    // Globally track callbacks since wkhtmltopdf doesn't allow injecting any userdata
    // The HashMap key is the converter's raw pointer cast as usize, so we can have unique callbacks per converter
    static ref FINISHED_CALLBACKS: Mutex<HashMap<usize, Box<FnMut(i32) + 'static + Send>>> = Mutex::new(HashMap::new());
    static ref ERROR_CALLBACKS: Mutex<HashMap<usize, Box<FnMut(String) + 'static + Send>>> = Mutex::new(HashMap::new());
    // TODO: 3 more callback types
}

pub type Result<T> = std::result::Result<T, String>; // TODO: better error type than String

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

#[derive(Clone)]
pub enum Size { Millimeters(u32), Inches(u32) }
impl Size {
    fn value(&self) -> Cow<'static, str> {
        match self {
            &Size::Millimeters(ref n) => format!("{}mm", n),
            &Size::Inches(ref n) => format!("{}in", n),
        }.into()
    }
}

pub enum Orientation { Landscape, Portrait }
impl Orientation {
    fn value(&self) -> Cow<'static, str> {
        match self {
            &Orientation::Landscape => "Landscape".into(),
            &Orientation::Portrait => "Portrait".into(),
        }
    }
}
pub struct Margin {
    pub top: Size,
    pub bottom: Size,
    pub left: Size,
    pub right: Size,
}
impl Margin {
    pub fn all(size: Size) -> Margin {
        Margin{ top: size.clone(), bottom: size.clone(), left: size.clone(), right: size.clone() }
    }
}

pub struct PdfSettings {
    /// The paper size of the output document (default A4)
    pub page_size: PageSize,
    /// The orientation of the output document (default portrait)
    pub orientation: Orientation,
    /// What dpi should we use when printin (default 72)
    pub dpi: u32,
    // /// The maximum depth of the outline (table of contents) to generate in the sidebar (default None)
    // pub outline_depth: Option<u32>,
    /// The title of the PDF document (default None)
    pub title: Option<String>,
    /// Size of the page margins (default 10mm on all sides)
    pub margin: Margin,
    /// JPEG image compression quality in percentage (default 94)
    pub image_quality: u32,
}

type Setting = (&'static str, Cow<'static, str>);

impl PdfSettings {
    fn global_settings<'a>(&'a self) -> HashMap<&'static str, Cow<'a, str>> {
        let mut settings = HashMap::new();

        settings.insert("margin.top", self.margin.top.value());
        settings.insert("margin.bottom", self.margin.bottom.value());
        settings.insert("margin.left", self.margin.left.value());
        settings.insert("margin.right", self.margin.right.value());
        settings.insert("dpi", self.dpi.to_string().into());
        settings.insert("orientation", self.orientation.value());
        settings.insert("imageQuality", self.image_quality.to_string().into());

        if let Some(ref title) = self.title {
            settings.insert("documentTitle", Cow::Borrowed(title));
        }
        // if let Some(depth) = self.outline_depth {
        //     settings.insert("outline", "true".into());
        //     // settings.insert("outlineDepth", depth.to_string().into());
        // }

        match self.page_size {
            PageSize::Custom(ref w, ref h) => {
                settings.insert("size.width", w.value());
                settings.insert("size.height", w.value());
            },
            _ => {
                settings.insert("size.pageSize", self.page_size.value());
            }
        };

        settings
    }
    fn object_settings(&self) -> Vec<(&'static str, String)> {
        vec![

        ]
    }
}

impl Default for PdfSettings {
    fn default() -> Self {
        PdfSettings {
            page_size: PageSize::A4,
            orientation: Orientation::Portrait,
            dpi: 72,
            // outline_depth: Some(4),
            title: None,
            image_quality: 94,
            margin: Margin::default(),
        }
    }
}

impl Default for Margin {
    fn default() -> Self {
        Margin {
            top: Size::Millimeters(10),
            left: Size::Millimeters(10),
            right: Size::Millimeters(10),
            bottom: Size::Millimeters(10),
        }
    }
}

enum Source {
    Url(Url),
    Path(PathBuf),
    Html(String)
}

pub struct PdfBuilder {
    src: Source,
    settings: PdfSettings
}

impl PdfBuilder {
    // initializers
    pub unsafe fn from_url<U: Into<Url>>(url: U) -> PdfBuilder {
        PdfBuilder{
            src: Source::Url(url.into()),
            settings: Default::default(),
        }
    }
    pub fn from_path<P: AsRef<Path>>(path: P) -> PdfBuilder {
        PdfBuilder{
            src: Source::Path(path.as_ref().to_owned()),
            settings: Default::default(),
        }
    }
    pub fn from_html<S: Into<String>>(html: S) -> PdfBuilder {
        PdfBuilder{
            src: Source::Html(html.into()),
            settings: Default::default(),
        }
    }

    pub fn configure(&mut self, settings: PdfSettings) -> &mut PdfBuilder {
        self.settings = settings;
        self
    }

    // Finalizers
    pub fn build(&mut self) -> Result<Vec<u8>> {
        let mut global = PdfGlobal::new();
        for (name, val) in self.settings.global_settings() {
            try!( unsafe { global.set(name, &val) } );
        }

        let mut converter = global.create_converter();
        let mut object = PdfObject::new();
        for (name, val) in self.settings.object_settings() {
            try!( unsafe { object.set(name, &val) } );
        }

        match self.src {
            Source::Url(ref url) => {
                try!( unsafe { object.set("page", url.as_str()) } );
                converter.add_object(object);
            },
            Source::Path(ref path) => {
                try!( unsafe { object.set("page", &path.to_string_lossy()) } );
                converter.add_object(object);
            },
            Source::Html(ref html) => {
                converter.add_html(object, html);
            }
        };

        unsafe { converter.convert() }
    }
    pub fn build_pdf(&mut self) -> Result<File> {
        // todo: call build and write it to a temp file
        unimplemented!();
    }
}

pub struct PdfGlobal {
    global_settings: *mut wkhtmltopdf_global_settings,
}

impl PdfGlobal {
    pub fn new() -> PdfGlobal {
        // todo: what if safe_wkhtmltopdf_init failed?
        let mut global_count = WKHTMLTOPDF_GLOBAL_COUNT.lock().unwrap();
        if *global_count == 0 {
            let success = unsafe {
                wkhtmltopdf_init(0) == 1
            };
            if success {
                *global_count += 1;
            }
        }

        unsafe {
            PdfGlobal {
                global_settings: wkhtmltopdf_create_global_settings()
            }
        }
    }

    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = try!(CString::new(name)
            .map_err(|err| format!("encountered null byte in 'name'- {}", err)));
        let c_value = try!(CString::new(value)
            .map_err(|err| format!("encountered null byte in 'value'- {}", err)));
        match unsafe { wkhtmltopdf_set_global_setting(self.global_settings, c_name.as_ptr(), c_value.as_ptr()) } {
            0 => Err(format!("failed to set '{}' to '{}'", name, value)),
            1 => Ok(()),
            _ => unreachable!("wkhtmltopdf_set_global_setting returned invalid value"),
        }
    }

    // Consume self because create_converter consumes global_settings
    pub fn create_converter(self) -> PdfConverter {
        PdfConverter {
            converter: unsafe { wkhtmltopdf_create_converter(self.global_settings) },
            _global: self,
        }
    }
}

impl Drop for PdfGlobal {
    fn drop(&mut self) {
        let mut global_count = WKHTMLTOPDF_GLOBAL_COUNT.lock().unwrap();
        unsafe { wkhtmltopdf_destroy_global_settings(self.global_settings); }
        match *global_count {
            0 => unreachable!("unsound attempt to deinit wkhtmlpdf"),
            1 => {
               let success = unsafe { wkhtmltopdf_deinit() == 1 };
               if success {
                   *global_count = 0;
               } // TODO: if failed to deinit?
            },
            _ => {
                *global_count -= 1;
            }
        }
    }
}

pub struct PdfConverter {
    converter: *mut wkhtmltopdf_converter,
    _global: PdfGlobal, // just holding it to control the drop sequence
}

impl PdfConverter {
    pub fn add_object(&mut self, pdf_object: PdfObject) {
        let null: *const c_char = std::ptr::null();
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, null);
        };
    }

    pub fn add_html(&mut self, pdf_object: PdfObject, html: &str) {
        let c_html = CString::new(html).expect("null byte found");
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, c_html.as_ptr());
        };
    }

    // pub fn monitor(&self) {
        // unsafe {
        //     wkhtmltopdf_set_progress_changed_callback(converter, Some(int_callback));
        //     wkhtmltopdf_set_warning_callback(converter, Some(str_callback));
        //     wkhtmltopdf_set_phase_changed_callback(converter, Some(void_callback));
        // };
    // }

    // blocks until complete (or error)

    pub unsafe fn convert(self) -> Result<Vec<u8>> {
        let rx = self.setup_callbacks();

        let success = wkhtmltopdf_convert(self.converter) == 1;

        if success {
            match rx.recv().expect("sender disconnected") {
                Ok(_) => {
                    // TODO: for some strange reason, this sleep prevents segfault in debug mode
                    //   the sleep has to happen before mem::uninitialized
                    //   otherwise wkhtmltopdf_get_output segfaults.
                    //   release mode is less predictable.
                    // Very probably this code is sound yet,
                    //   but it's also possible wkhtmltopdf has a data race
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    let buf_ptr: *mut *const c_uchar = std::mem::uninitialized();
                    let bytes = wkhtmltopdf_get_output(self.converter, buf_ptr) as usize;
                    let buf_slice = std::slice::from_raw_parts(*buf_ptr as *const c_uchar, bytes);
                    Ok(buf_slice.to_vec())
                },
                Err(err) => {
                    Err(format!("wkhtmltopdf_convert failed: {}", err))
                }
            }
        } else {
            Err("wkhtmltopdf_convert".to_string())
        }
    }

    fn setup_callbacks(&self) -> Receiver<Result<()>> {
        unsafe {
            wkhtmltopdf_set_finished_callback(self.converter, Some(finished_callback));
            wkhtmltopdf_set_error_callback(self.converter, Some(error_callback));
        }

        let (tx, rx) = mpsc::channel();
        let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let tx_finished = tx.clone();
        let errors_finished = errors.clone();
        let on_finished = move |i| {
            let mut errors = errors_finished.lock().unwrap();

            let msg = match i {
                1 => Ok(()),
                _ => Err(format!("Finished with errors: {}", errors.join(", "))),
            };
            let _ = tx_finished.send(msg);
        };

        let tx_error = tx.clone();
        let on_error = move |err| {
            let mut errors = errors.lock().unwrap();
            errors.push(err);
        };

        // Insert into our lazy static callbacks
        {
            let id = self.converter as usize;
            let mut finished_callbacks = FINISHED_CALLBACKS.lock().unwrap();
            finished_callbacks.insert(id, Box::new(on_finished));
            let mut error_callbacks = ERROR_CALLBACKS.lock().unwrap();
            error_callbacks.insert(id, Box::new(on_error));
        }

        rx
    }

}

impl Drop for PdfConverter {
    fn drop(&mut self) {
        unsafe { wkhtmltopdf_destroy_converter(self.converter) }
    }
}

pub struct PdfObject {
    object_settings: *mut wkhtmltopdf_object_settings
}

impl PdfObject {
    pub fn new() -> PdfObject {
        PdfObject {
            object_settings: unsafe { wkhtmltopdf_create_object_settings() }
        }
    }

    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = try!(CString::new(name)
            .map_err(|err| format!("encountered null byte in 'name'- {}", err)));
        let c_value = try!(CString::new(value)
            .map_err(|err| format!("encountered null byte in 'value'- {}", err)));
        match unsafe { wkhtmltopdf_set_object_setting(self.object_settings, c_name.as_ptr(), c_value.as_ptr()) } {
            0 => Err(format!("failed to set '{}' to '{}'", name, value)),
            1 => Ok(()),
            _ => unreachable!("wkhtmltopdf_set_object_setting returned invalid value"),
        }
    }
}

// unsafe extern fn void_callback(_converter: *mut wkhtmltopdf_converter) {
//     println!("void callback fired");
// }

unsafe extern fn finished_callback(converter: *mut wkhtmltopdf_converter, val: c_int) {
    let id = converter as usize;
    {
        // call and remove this converter's FINISHED_CALLBACK
        let mut callbacks = FINISHED_CALLBACKS.lock().unwrap();
        if let Some(mut cb) = callbacks.remove(&id) {
            cb(val as i32);
        }
    }
    {
        // remove this converter's ERROR_CALLBACK
        let mut callbacks = ERROR_CALLBACKS.lock().unwrap();
        let _ = callbacks.remove(&id);
    }
}

unsafe extern fn error_callback(converter: *mut wkhtmltopdf_converter, ptr: *const c_char) {
    let cstr = CStr::from_ptr(ptr);
    let mut callbacks = ERROR_CALLBACKS.lock().unwrap();
    let id = converter as usize;
    let msg = cstr.to_string_lossy().into_owned();
    match callbacks.get_mut(&id) {
        Some(cb) => cb(msg),
        None => println!("No callback for error: {}", msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let res = PdfBuilder::from_html("foo").build();
        assert_eq!(res.is_ok(), true);
        assert_eq!(res.unwrap().len(), 3124);
    }
}

