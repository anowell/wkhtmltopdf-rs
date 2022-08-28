//! Low-level wkhtmltoimage without the raw pointers
//!
//! This module abstracts away the raw pointers of [wkhmtltox-sys](https://anowell.github.io/wkhtmltox-sys/wkhtmltox_sys/)
//! while providing ownership and drop semantics necessary to safely use wkhtmltox-sys.
//!
//! It is recommended to use the [`imageBuilder`](../struct.imageBuilder.html) build methods which manage all of these details,
//! however, some usage scenarios (e.g. adding multiple objects to your image) may require
//! using this lower-level module to achieve sufficient control.
use lazy_static::lazy_static;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int};
use std::sync::{mpsc, Arc, Mutex};
use std::{ptr, slice};
use wkhtmltox_sys::image::*;

use super::{Error, ImageOutput, Result};

enum WkhtmltoimageState {
    // Wkhtmltoimage has not yet been initialized
    New,
    // Wkhtmltoimage backend is available for image generation
    Ready,
    // Wkhtmltoimage backend is busy, so attempts to init a `ImageGlobalSettings` instance will return `Error::Blocked`
    Busy,
    // Once dropped, wkthmltoimage cannot be used again for the life of this process
    Dropped,
}

lazy_static! {
    // Globally count wkhtmltoimage handles so we can safely init/deinit the underlying wkhtmltoimage singleton
    static ref WKHTMLTOIMAGE_STATE: Mutex<WkhtmltoimageState> = Mutex::new(WkhtmltoimageState::New);
    static ref WKHTMLTOIMAGE_INIT_THREAD: usize = thread_id::get();

    // Globally track callbacks since wkhtmltoimage doesn't allow injecting any userdata
    // The HashMap key is the converter's raw pointer cast as usize, so we can have unique callbacks per converter
    static ref FINISHED_CALLBACKS: Mutex<HashMap<usize, Box<dyn FnMut(i32) + 'static + Send>>> = Mutex::new(HashMap::new());
    static ref ERROR_CALLBACKS: Mutex<HashMap<usize, Box<dyn FnMut(String) + 'static + Send>>> = Mutex::new(HashMap::new());
    // TODO: 3 more callback types
}

/// Handles initialization and deinitialization of wkhtmltoimage
///
/// This struct may only be initialized once per process which is a
/// which is [basic limitation of wkhtmltoimage](https://github.com/wkhtmltoimage/wkhtmltoimage/issues/1890).
///
/// When it goes out of scope, wkhtmltoimage will be deinitialized
/// and further image generation will not be possible.
pub struct ImageGuard {
    // Private to prevent struct construction
    // PhantomData<*const ()> to effectively impl !Send and !Sync on stable
    _private: PhantomData<*const ()>,
}

/// Safe wrapper for managing wkhtmltoimage global settings
pub struct ImageGlobalSettings {
    global_settings: *mut wkhtmltoimage_global_settings,
    // We only need to destroy global_settings if never consumed by wkhtmltoimage_create_converter
    needs_delete: bool,
}

/// Safe wrapper for working with the wkhtmltoimage converter
pub struct ImageConverter {
    converter: *mut wkhtmltoimage_converter,
    // imageGlobalSettings::drop also manages wkhtmktoimage_deinit, take ownership to delay drop
    _global: ImageGlobalSettings,
}

/// Initializes wkhtmltoimage
///
/// This function will only initialize wkhtmltoimage once per process which is a
///   [fundamental limitation of wkhtmltoimage](https://github.com/wkhtmltoimage/wkhtmltoimage/issues/1890).
///   Calling [`ImageApplication::new()`](../struct.ImageApplication.html)
///   has the same effect of initializing wkhtmltoimage.
///
/// Subsequent attempts to initialize wkhtmltoimage will return `Error:IllegalInit`
pub fn image_init() -> Result<ImageGuard> {
    let mut wk_state = WKHTMLTOIMAGE_STATE.lock().unwrap();
    match *wk_state {
        WkhtmltoimageState::New => {
            debug!("wkhtmltoimage_init graphics=0");
            let success = unsafe { wkhtmltoimage_init(0) == 1 };
            if success {
                *wk_state = WkhtmltoimageState::Ready;
                // first eval of the lazy static - effectively stores the thread id
                let _ = *WKHTMLTOIMAGE_INIT_THREAD;
            } else {
                error!("failed to initialize wkhtmltoimage");
            }
            Ok(ImageGuard {
                _private: PhantomData,
            })
        }
        _ => Err(Error::IllegalInit),
    }
}

impl ImageGlobalSettings {
    /// Instantiate ImageGlobalSettings
    ///
    /// This may only be called after `image_init` has successfully initialized wkhtmltoimage
    pub fn new() -> Result<ImageGlobalSettings> {
        if *WKHTMLTOIMAGE_INIT_THREAD != thread_id::get() {
            // A lot of QT functionality expects to run from the same thread that it was first initialized on
            return Err(Error::ThreadMismatch(
                *WKHTMLTOIMAGE_INIT_THREAD,
                thread_id::get(),
            ));
        }

        let mut wk_state = WKHTMLTOIMAGE_STATE.lock().unwrap();
        match *wk_state {
            WkhtmltoimageState::New => Err(Error::NotInitialized),
            WkhtmltoimageState::Dropped => Err(Error::NotInitialized),
            WkhtmltoimageState::Busy => Err(Error::Blocked),
            WkhtmltoimageState::Ready => {
                debug!("wkhtmltoimage_create_global_settings");
                let gs = unsafe { wkhtmltoimage_create_global_settings() };
                // TODO: is it possible to delay setting Busy until convert is called?
                *wk_state = WkhtmltoimageState::Busy;
                Ok(ImageGlobalSettings {
                    global_settings: gs,
                    needs_delete: true,
                })
            }
        }
    }

    /// Set a global setting for the wkhtmltoimage instance
    ///
    /// # Safety
    ///
    /// Unsafe as it may cause undefined behavior (generally segfault) if name or value are not valid
    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = CString::new(name).expect("setting name may not contain interior null bytes");
        let c_value =
            CString::new(value).expect("setting value may not contain interior null bytes");

        debug!("wkhtmltoimage_set_global_setting {}='{}'", name, value);
        match wkhtmltoimage_set_global_setting(
            self.global_settings,
            c_name.as_ptr(),
            c_value.as_ptr(),
        ) {
            0 => Err(Error::GlobalSettingFailure(name.into(), value.into())),
            1 => Ok(()),
            _ => unreachable!("wkhtmltoimage_set_global_setting returned invalid value"),
        }
    }

    /// calls wkhtmltoimage_create_converter which consumes global_settings
    ///   and thus we no longer need concern ourselves with deleting it
    pub fn create_converter(mut self) -> ImageConverter {
        debug!("wkhtmltoimage_create_converter");
        let converter = unsafe { wkhtmltoimage_create_converter(self.global_settings, &0) };
        self.needs_delete = false;

        ImageConverter {
            converter,
            _global: self,
        }
    }

    /// calls wkhtmltoimage_create_converter which consumes global_settings
    ///   and thus we no longer need concern ourselves with deleting it
    pub fn create_converter_with_html(mut self, html: &str) -> ImageConverter {
        debug!("wkhtmltoimage_create_converter");
        let c_html = CString::new(html).expect("html may not contain interior null bytes");

        let converter =
            unsafe { wkhtmltoimage_create_converter(self.global_settings, c_html.as_ptr()) };
        self.needs_delete = false;

        ImageConverter {
            converter,
            _global: self,
        }
    }
}

impl ImageConverter {
    /// Performs the HTML to image conversion
    ///
    /// This method does not do any additional allocations of the output,
    ///   so the `ImageConverter` will be owned by `ImageOutput` so that
    ///   it is not dropped until the `ImageOutput` is dropped.
    pub fn convert<'a>(self) -> Result<ImageOutput<'a>> {
        let rx = self.setup_callbacks();
        debug!("wkhtmltoimage_convert");
        let success = unsafe { wkhtmltoimage_convert(self.converter) == 1 };
        self.remove_callbacks();

        if success {
            let mut buf_ptr = ptr::null();
            debug!("wkhtmltoimage_get_output");
            unsafe {
                let bytes = wkhtmltoimage_get_output(self.converter, &mut buf_ptr) as usize;
                let image_slice = slice::from_raw_parts(buf_ptr, bytes);
                Ok(ImageOutput {
                    data: image_slice,
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
            debug!("wkhtmltoimage_set_finished_callback");
            wkhtmltoimage_set_finished_callback(self.converter, Some(finished_callback));
            debug!("wkhtmltoimage_set_error_callback");
            wkhtmltoimage_set_error_callback(self.converter, Some(error_callback));
            // wkhtmltoimage_set_progress_changed_callback(self.converter, Some(progress_changed));
            // wkhtmltoimage_set_phase_changed_callback(self.converter, Some(phase_changed));
            // wkhtmltoimage_set_warning_callback(self.converter, Some(warning_cb));
        }

        rx
    }
}

impl Drop for ImageConverter {
    fn drop(&mut self) {
        debug!("wkhtmltoimage_destroy_converter");
        unsafe { wkhtmltoimage_destroy_converter(self.converter) }
    }
}

// TODO: is it possible to revert to ready after convert finishes?
impl<'a> Drop for ImageOutput<'a> {
    fn drop(&mut self) {
        let mut wk_state = WKHTMLTOIMAGE_STATE.lock().unwrap();
        debug!("wkhtmltoimage ready again");
        *wk_state = WkhtmltoimageState::Ready;
    }
}

impl Drop for ImageGuard {
    fn drop(&mut self) {
        let mut wk_state = WKHTMLTOIMAGE_STATE.lock().unwrap();
        debug!("wkhtmltoimage_deinit");
        let success = unsafe { wkhtmltoimage_deinit() == 1 };
        *wk_state = WkhtmltoimageState::Dropped;
        if !success {
            warn!("Failed to deinitialize wkhtmltoimage")
        }
    }
}

unsafe extern "C" fn finished_callback(converter: *mut wkhtmltoimage_converter, val: c_int) {
    let id = converter as usize;
    {
        // call and remove this converter's FINISHED_CALLBACK
        let mut callbacks = FINISHED_CALLBACKS.lock().unwrap();
        if let Some(mut cb) = callbacks.remove(&id) {
            cb(val as i32);
        }
    }
}

unsafe extern "C" fn error_callback(
    converter: *mut wkhtmltoimage_converter,
    msg_ptr: *const c_char,
) {
    let cstr = CStr::from_ptr(msg_ptr);
    let mut callbacks = ERROR_CALLBACKS.lock().unwrap();
    let id = converter as usize;
    let msg = cstr.to_string_lossy().into_owned();
    match callbacks.get_mut(&id) {
        Some(cb) => cb(msg),
        None => println!("No callback for error: {}", msg),
    }
}

// unsafe extern fn warning_cb(_converter: *mut wkhtmltoimage_converter, msg_ptr: *const c_char) {
//     let msg = CStr::from_ptr(msg_ptr).to_string_lossy();
//     println!("Warning: {}", msg);
// }

// unsafe extern fn progress_changed(_converter: *mut wkhtmltoimage_converter, val: c_int) {
//     println!("{:3}", val);
// }

// unsafe extern fn phase_changed(converter: *mut wkhtmltoimage_converter) {
//     let phase = wkhtmltoimage_current_phase(converter);
//     let desc = wkhtmltoimage_phase_description(converter, phase);
// 	println!("Phase: {}", CStr::from_ptr(desc).to_string_lossy());
// }
