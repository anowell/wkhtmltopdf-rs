

use std::{io, ffi, result};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        IoError(err: io::Error) {
            description("io error")
            display("I/O error: {}", err)
            cause(err)
            from()
        }

        NulError(err: ffi::NulError) {
            description(err.description())
            display("CString error: {}", err)
            cause(err)
            from()
        }

        ConversionFailed(msg: String) {
            description("conversion failed")
            display("Conversion failed: {}", msg)
        }

        GlobalSettingFailure(name: String, value: String) {
            description("global setting failure")
            display("Failed to update global setting '{}'='{}'", name, value)
        }

        ObjectSettingFailure(name: String, value: String) {
            description("object setting failure")
            display("Failed to update object setting '{}'='{}'", name, value)
        }
    }
}

pub type Result<T> = result::Result<T, Error>;