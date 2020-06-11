//! Low-level wkhtmltopdf without the raw pointers
//!
//! This module abstracts away the raw pointers of [wkhmtltox-sys](https://anowell.github.io/wkhtmltox-sys/wkhtmltox_sys/)
//! while providing ownership and drop semantics necessary to safely use wkhtmltox-sys.
//!
//! It is recommended to use the [`PdfBuilder`](../struct.PdfBuilder.html) build methods which manage all of these details,
//! however, some usage scenarios (e.g. adding multiple objects to your PDF) may require
//! using this lower-level module to achieve sufficient control.
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int};
use std::sync::{mpsc, Arc, Mutex};
use std::{ptr, slice};
use thread_id;
use wkhtmltox_sys::pdf::*;

use super::{Error, PdfOutput, Result};

enum WkhtmltopdfState {
    // Wkhtmltopdf has not yet been initialized
    New,
    // Wkhtmltopdf backend is available for PDF generation
    Ready,
    // Wkhtmltopdf backend is busy, so attempts to init a `PdfGlobalSettings` instance will return `Error::Blocked`
    Busy,
    // Once dropped, wkthmltopdf cannot be used again for the life of this process
    Dropped,
}

lazy_static! {
    // Globally count wkhtmltopdf handles so we can safely init/deinit the underlying wkhtmltopdf singleton
    static ref WKHTMLTOPDF_STATE: Mutex<WkhtmltopdfState> = Mutex::new(WkhtmltopdfState::New);
    static ref WKHTMLTOPDF_INIT_THREAD: usize = thread_id::get();

    // Globally track callbacks since wkhtmltopdf doesn't allow injecting any userdata
    // The HashMap key is the converter's raw pointer cast as usize, so we can have unique callbacks per converter
    static ref FINISHED_CALLBACKS: Mutex<HashMap<usize, Box<dyn FnMut(i32) + 'static + Send>>> = Mutex::new(HashMap::new());
    static ref ERROR_CALLBACKS: Mutex<HashMap<usize, Box<dyn FnMut(String) + 'static + Send>>> = Mutex::new(HashMap::new());
    // TODO: 3 more callback types
}

/// Handles initialization and deinitialization of wkhtmltopdf
///
/// This struct may only be initialized once per process which is a
/// which is [basic limitation of wkhtmltopdf](https://github.com/wkhtmltopdf/wkhtmltopdf/issues/1890).
///
/// When it goes out of scope, wkhtmltopdf will be deinitialized
/// and further PDF generation will not be possible.
pub struct PdfGuard {
    // Private to prevent struct construction
    // PhantomData<*const ()> to effectively impl !Send and !Sync on stable
    _private: PhantomData<*const ()>,
}

/// Safe wrapper for managing wkhtmltopdf global settings
pub struct PdfGlobalSettings {
    global_settings: *mut wkhtmltopdf_global_settings,
    // We only need to destroy global_settings if never consumed by wkhtmltopdf_create_converter
    needs_delete: bool,
}

/// Safe wrapper for managing wkhtmltopdf object settings
pub struct PdfObjectSettings {
    object_settings: *mut wkhtmltopdf_object_settings,
    // We only need to destroy object_settings if never consumed by wkhtmltopdf_add_object
    needs_delete: bool,
}

/// Safe wrapper for working with the wkhtmltopdf converter
pub struct PdfConverter {
    converter: *mut wkhtmltopdf_converter,
    // PdfGlobalSettings::drop also manages wkhtmktopdf_deinit, take ownership to delay drop
    _global: PdfGlobalSettings,
}

/// Initializes wkhtmltopdf
///
/// This function will only initialize wkhtmltopdf once per process which is a
///   [fundamental limitation of wkhtmltopdf](https://github.com/wkhtmltopdf/wkhtmltopdf/issues/1890).
///   Calling [`PdfApplication::new()`](../struct.PdfApplication.html)
///   has the same effect of initializing wkhtmltopdf.
///
/// Subsequent attempts to initialize wkhtmltopdf will return `Error:IllegalInit`
pub fn pdf_init() -> Result<PdfGuard> {
    let mut wk_state = WKHTMLTOPDF_STATE.lock().unwrap();
    match *wk_state {
        WkhtmltopdfState::New => {
            debug!("wkhtmltopdf_init graphics=0");
            let success = unsafe { wkhtmltopdf_init(0) == 1 };
            if success {
                *wk_state = WkhtmltopdfState::Ready;
                // first eval of the lazy static - effectively stores the thread id
                let _ = *WKHTMLTOPDF_INIT_THREAD;
            } else {
                error!("failed to initialize wkhtmltopdf");
            }
            Ok(PdfGuard {
                _private: PhantomData,
            })
        }
        _ => Err(Error::IllegalInit),
    }
}

impl PdfGlobalSettings {
    /// Instantiate PdfGlobalSettings
    ///
    /// This may only be called after `pdf_init` has successfully initialized wkhtmltopdf
    pub fn new() -> Result<PdfGlobalSettings> {
        if *WKHTMLTOPDF_INIT_THREAD != thread_id::get() {
            // A lot of QT functionality expects to run from the same thread that it was first initialized on
            return Err(Error::ThreadMismatch(
                *WKHTMLTOPDF_INIT_THREAD,
                thread_id::get(),
            ));
        }

        let mut wk_state = WKHTMLTOPDF_STATE.lock().unwrap();
        match *wk_state {
            WkhtmltopdfState::New => Err(Error::NotInitialized),
            WkhtmltopdfState::Dropped => Err(Error::NotInitialized),
            WkhtmltopdfState::Busy => Err(Error::Blocked),
            WkhtmltopdfState::Ready => {
                debug!("wkhtmltopdf_create_global_settings");
                let gs = unsafe { wkhtmltopdf_create_global_settings() };
                // TODO: is it possible to delay setting Busy until convert is called?
                *wk_state = WkhtmltopdfState::Busy;
                Ok(PdfGlobalSettings {
                    global_settings: gs,
                    needs_delete: true,
                })
            }
        }
    }

    // Unsafe as it may cause undefined behavior (generally segfault) if name or value are not valid
    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = CString::new(name).expect("setting name may not contain interior null bytes");
        let c_value =
            CString::new(value).expect("setting value may not contain interior null bytes");

        debug!("wkhtmltopdf_set_global_setting {}='{}'", name, value);
        match wkhtmltopdf_set_global_setting(
            self.global_settings,
            c_name.as_ptr(),
            c_value.as_ptr(),
        ) {
            0 => Err(Error::GlobalSettingFailure(name.into(), value.into())),
            1 => Ok(()),
            _ => unreachable!("wkhtmltopdf_set_global_setting returned invalid value"),
        }
    }

    pub fn create_converter(mut self) -> PdfConverter {
        // call wkhtmltopdf_create_convert which consumes global_settings
        //   and thus we no longer need concern ourselves with deleting it
        debug!("wkhtmltopdf_create_converter");
        let converter = unsafe { wkhtmltopdf_create_converter(self.global_settings) };
        self.needs_delete = false;

        PdfConverter {
            converter,
            _global: self,
        }
    }
}

impl PdfConverter {
    /// Adds a page object to the PDF by URL or local path to the page
    ///
    /// This method will set/override the `page` object setting.
    pub fn add_page_object(&mut self, mut pdf_object: PdfObjectSettings, page: &str) {
        unsafe {
            pdf_object
                .set("page", page)
                .expect("Failed to set 'page' setting");
        }

        debug!("wkhtmltopdf_add_object data=NULL");
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, ptr::null());
        };
        pdf_object.needs_delete = false;
    }

    /// Adds a page object to the PDF using provided HTML data
    ///
    /// In general, this will result in ignoring the 'page' setting if added to this `pdf_object`.
    ///   The exception is when `html` is an empty string, but `app_page_object` should be
    ///   the preferred way to set the `page` setting.
    pub fn add_html_object(&mut self, mut pdf_object: PdfObjectSettings, html: &str) {
        let c_html = CString::new(html).expect("null byte found");

        debug!("wkhtmltopdf_add_object data=&html");
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, c_html.as_ptr());
        };
        pdf_object.needs_delete = false;
    }

    /// Performs the HTML to PDF conversion
    ///
    /// This method does not do any additional allocations of the output,
    ///   so the `PdfConverter` will be owned by `PdfOutput` so that
    ///   it is not dropped until the `PdfOutput` is dropped.
    pub fn convert<'a>(self) -> Result<PdfOutput<'a>> {
        let rx = self.setup_callbacks();
        debug!("wkhtmltopdf_convert");
        let success = unsafe { wkhtmltopdf_convert(self.converter) == 1 };
        self.remove_callbacks();

        if success {
            let mut buf_ptr = ptr::null();
            debug!("wkhtmltopdf_get_output");
            unsafe {
                let bytes = wkhtmltopdf_get_output(self.converter, &mut buf_ptr) as usize;
                let pdf_slice = slice::from_raw_parts(buf_ptr, bytes);
                Ok(PdfOutput {
                    data: pdf_slice,
                    _converter: self,
                })
            }
        } else {
            match rx.recv().expect("sender disconnected") {
                Ok(_) => unreachable!("failed without errors"),
                Err(err) => Err(err),
            }
        }
    }

    fn remove_callbacks(&self) {
        let id = self.converter as usize;

        let _ = ERROR_CALLBACKS.lock().unwrap().remove(&id);
        let _ = FINISHED_CALLBACKS.lock().unwrap().remove(&id);
    }

    fn setup_callbacks(&self) -> mpsc::Receiver<Result<()>> {
        let (tx, rx) = mpsc::channel();
        let errors = Arc::new(Mutex::new(Vec::new()));

        let tx_finished = tx;
        let errors_finished = errors.clone();
        let on_finished = move |i| {
            let errors = errors_finished.lock().unwrap();

            let res = match i {
                1 => Ok(()),
                _ => Err(Error::ConversionFailed(errors.join(", "))),
            };
            let _ = tx_finished.send(res);
        };

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

        unsafe {
            debug!("wkhtmltopdf_set_finished_callback");
            wkhtmltopdf_set_finished_callback(self.converter, Some(finished_callback));
            debug!("wkhtmltopdf_set_error_callback");
            wkhtmltopdf_set_error_callback(self.converter, Some(error_callback));
            // wkhtmltopdf_set_progress_changed_callback(self.converter, Some(progress_changed));
            // wkhtmltopdf_set_phase_changed_callback(self.converter, Some(phase_changed));
            // wkhtmltopdf_set_warning_callback(self.converter, Some(warning_cb));
        }

        rx
    }
}

impl PdfObjectSettings {
    pub fn new() -> PdfObjectSettings {
        debug!("wkhtmltopdf_create_object_settings");
        PdfObjectSettings {
            object_settings: unsafe { wkhtmltopdf_create_object_settings() },
            needs_delete: true,
        }
    }

    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = CString::new(name).expect("setting name may not contain interior null bytes");
        let c_value =
            CString::new(value).expect("setting value may not contain interior null bytes");

        debug!("wkhtmltopdf_set_object_setting {}='{}'", name, value);
        match wkhtmltopdf_set_object_setting(
            self.object_settings,
            c_name.as_ptr(),
            c_value.as_ptr(),
        ) {
            0 => Err(Error::ObjectSettingFailure(name.into(), value.into())),
            1 => Ok(()),
            _ => unreachable!("wkhtmltopdf_set_object_setting returned invalid value"),
        }
    }
}

impl Drop for PdfGlobalSettings {
    fn drop(&mut self) {
        if self.needs_delete {
            debug!("wkhtmltopdf_destroy_global_settings");
            unsafe {
                wkhtmltopdf_destroy_global_settings(self.global_settings);
            }
        }
    }
}

impl Drop for PdfConverter {
    fn drop(&mut self) {
        debug!("wkhtmltopdf_destroy_converter");
        unsafe { wkhtmltopdf_destroy_converter(self.converter) }
    }
}

impl Drop for PdfObjectSettings {
    fn drop(&mut self) {
        if self.needs_delete {
            debug!("wkhtmltopdf_destroy_object_settings");
            unsafe {
                wkhtmltopdf_destroy_object_settings(self.object_settings);
            }
        }
    }
}

// TODO: is it possible to revert to ready after convert finishes?
impl<'a> Drop for PdfOutput<'a> {
    fn drop(&mut self) {
        let mut wk_state = WKHTMLTOPDF_STATE.lock().unwrap();
        debug!("wkhtmltopdf ready again");
        *wk_state = WkhtmltopdfState::Ready;
    }
}

impl Drop for PdfGuard {
    fn drop(&mut self) {
        let mut wk_state = WKHTMLTOPDF_STATE.lock().unwrap();
        debug!("wkhtmltopdf_deinit");
        let success = unsafe { wkhtmltopdf_deinit() == 1 };
        *wk_state = WkhtmltopdfState::Dropped;
        if !success {
            warn!("Failed to deinitialize wkhtmltopdf")
        }
    }
}

unsafe extern "C" fn finished_callback(converter: *mut wkhtmltopdf_converter, val: c_int) {
    let id = converter as usize;
    {
        // call and remove this converter's FINISHED_CALLBACK
        let mut callbacks = FINISHED_CALLBACKS.lock().unwrap();
        if let Some(mut cb) = callbacks.remove(&id) {
            cb(val as i32);
        }
    }
}

unsafe extern "C" fn error_callback(converter: *mut wkhtmltopdf_converter, msg_ptr: *const c_char) {
    let cstr = CStr::from_ptr(msg_ptr);
    let mut callbacks = ERROR_CALLBACKS.lock().unwrap();
    let id = converter as usize;
    let msg = cstr.to_string_lossy().into_owned();
    match callbacks.get_mut(&id) {
        Some(cb) => cb(msg),
        None => println!("No callback for error: {}", msg),
    }
}

// unsafe extern fn warning_cb(_converter: *mut wkhtmltopdf_converter, msg_ptr: *const c_char) {
//     let msg = CStr::from_ptr(msg_ptr).to_string_lossy();
//     println!("Warning: {}", msg);
// }

// unsafe extern fn progress_changed(_converter: *mut wkhtmltopdf_converter, val: c_int) {
//     println!("{:3}", val);
// }

// unsafe extern fn phase_changed(converter: *mut wkhtmltopdf_converter) {
//     let phase = wkhtmltopdf_current_phase(converter);
//     let desc = wkhtmltopdf_phase_description(converter, phase);
// 	println!("Phase: {}", CStr::from_ptr(desc).to_string_lossy());
// }
