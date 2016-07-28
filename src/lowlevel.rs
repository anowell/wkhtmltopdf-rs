//! Low-level wkhtmltopdf without the raw pointers
//!
//! This module abstracts away the raw pointers of [wkhmtltox-sys](https://anowell.github.io/wkhtmltox-sys/wkhtmltox_sys/)
//! while providing ownership and drop semantics necessary to safely use wkhtmltox-sys.
//!
//! It is recommended to use the `PdfBuilder` build methods which manage all of these details,
//! however, some usage scenarios (e.g. adding multiple objects to your PDF) may require
//! using this lower-level module to achieve sufficient control.
use wkhtmltox_sys::pdf::*;
use std::{ptr, slice};
use std::collections::HashMap;
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_int};
use std::sync::{Arc, Mutex, mpsc};
use super::{Result, PdfOutput};

lazy_static! {
    // Globally count wkhtmltopdf handles so we can safely init/deinit the underlying wkhtmltopdf singleton
    static ref WKHTMLTOPDF_GLOBAL_COUNT: Mutex<u32> = Mutex::new(0);

    // Globally track callbacks since wkhtmltopdf doesn't allow injecting any userdata
    // The HashMap key is the converter's raw pointer cast as usize, so we can have unique callbacks per converter
    static ref FINISHED_CALLBACKS: Mutex<HashMap<usize, Box<FnMut(i32) + 'static + Send>>> = Mutex::new(HashMap::new());
    static ref ERROR_CALLBACKS: Mutex<HashMap<usize, Box<FnMut(String) + 'static + Send>>> = Mutex::new(HashMap::new());
    // TODO: 3 more callback types
}

pub struct PdfGlobalSettings {
    global_settings: *mut wkhtmltopdf_global_settings,
    // We only need to destroy global_settings if never consumed by wkhtmltopdf_create_converter
    needs_delete: bool,
}

pub struct PdfObjectSettings {
    object_settings: *mut wkhtmltopdf_object_settings,
    // We only need to destroy object_settings if never consumed by wkhtmltopdf_add_object
    needs_delete: bool,
}

pub struct PdfConverter {
    converter: *mut wkhtmltopdf_converter,
    objects: Vec<PdfObjectSettings>,
    // PdfGlobalSettings::drop also manages wkhtmktopdf_deinit, take ownership to delay drop
    _global: PdfGlobalSettings,
}


impl PdfGlobalSettings {
    pub fn new() -> PdfGlobalSettings {
        // todo: what if safe_wkhtmltopdf_init failed?
        let mut global_count = WKHTMLTOPDF_GLOBAL_COUNT.lock().unwrap();
        if *global_count == 0 {
            debug!("wkhtmltopdf_init graphics=0");
            let success = unsafe {
                wkhtmltopdf_init(0) == 1
            };
            if success {
                *global_count += 1;
            }
        }

        unsafe {
            PdfGlobalSettings {
                global_settings: wkhtmltopdf_create_global_settings(),
                needs_delete: true,
            }
        }
    }

    // Unsafe as it may cause undefined behavior (generally segfault) if name or value are not valid
    pub unsafe fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let c_name = try!(CString::new(name)
            .map_err(|err| format!("encountered null byte in 'name'- {}", err)));
        let c_value = try!(CString::new(value)
            .map_err(|err| format!("encountered null byte in 'value'- {}", err)));

        debug!("wkhtmltopdf_set_global_setting {}='{}'", name, value);
        match wkhtmltopdf_set_global_setting(self.global_settings, c_name.as_ptr(), c_value.as_ptr()) {
            0 => Err(format!("failed to set '{}' to '{}'", name, value)),
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
            converter: converter,
            objects: Vec::new(),
            _global: self,
        }
    }
}


impl PdfConverter {
    pub fn add_page_object(&mut self, mut pdf_object: PdfObjectSettings) {
        let null: *const c_char = ptr::null();

        debug!("wkhtmltopdf_add_object data=NULL");
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, null);
        };
        pdf_object.needs_delete = false;
        self.objects.push(pdf_object);
    }

    pub fn add_html_object(&mut self, mut pdf_object: PdfObjectSettings, html: &str) {
        let c_html = CString::new(html).expect("null byte found");

        debug!("wkhtmltopdf_add_object data=&html");
        unsafe {
            wkhtmltopdf_add_object(self.converter, pdf_object.object_settings, c_html.as_ptr());
        };
        pdf_object.needs_delete = false;
        self.objects.push(pdf_object);
    }

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
                Ok(PdfOutput{ data: pdf_slice, _converter: self })
            }
        } else {
            match rx.recv().expect("sender disconnected") {
                Ok(_) => unreachable!(),
                Err(err) => {
                    Err(format!("wkhtmltopdf_convert failed: {}", err))
                }
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

        let tx_finished = tx.clone();
        let errors_finished = errors.clone();
        let on_finished = move |i| {
            let errors = errors_finished.lock().unwrap();

            let msg = match i {
                1 => Ok(()),
                _ => Err(errors.join(", ")),
            };
            let _ = tx_finished.send(msg);
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
        let c_name = try!(CString::new(name)
            .map_err(|err| format!("encountered null byte in 'name'- {}", err)));
        let c_value = try!(CString::new(value)
            .map_err(|err| format!("encountered null byte in 'value'- {}", err)));

        debug!("wkhtmltopdf_set_object_setting {}='{}'", name, value);
        match wkhtmltopdf_set_object_setting(self.object_settings, c_name.as_ptr(), c_value.as_ptr()) {
            0 => Err(format!("failed to set '{}' to '{}'", name, value)),
            1 => Ok(()),
            _ => unreachable!("wkhtmltopdf_set_object_setting returned invalid value"),
        }
    }
}


impl Drop for PdfGlobalSettings {
    fn drop(&mut self) {
        if self.needs_delete {
            debug!("wkhtmltopdf_destroy_global_settings");
            unsafe { wkhtmltopdf_destroy_global_settings(self.global_settings); }
        }

        let mut global_count = WKHTMLTOPDF_GLOBAL_COUNT.lock().unwrap();
        match *global_count {
            0 => unreachable!("unsound attempt to deinit wkhtmlpdf"),
            1 => {
                debug!("wkhtmltopdf_deinit");
                let success = unsafe { wkhtmltopdf_deinit() == 1 };
                if success {
                    *global_count = 0;
                } // TODO: what if failed to deinit?
            },
            _ => {
                *global_count -= 1;
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
            unsafe { wkhtmltopdf_destroy_object_settings(self.object_settings); }
        }
    }
}


unsafe extern fn finished_callback(converter: *mut wkhtmltopdf_converter, val: c_int) {
    let id = converter as usize;
    {
        // call and remove this converter's FINISHED_CALLBACK
        let mut callbacks = FINISHED_CALLBACKS.lock().unwrap();
        if let Some(mut cb) = callbacks.remove(&id) {
            cb(val as i32);
        }
    }
}

unsafe extern fn error_callback(converter: *mut wkhtmltopdf_converter, msg_ptr: *const c_char) {
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
