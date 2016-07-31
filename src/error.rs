

use std::{io, result};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        IoError(err: io::Error) {
            description("io error")
            display("I/O error: {}", err)
            cause(err)
            from()
        }

        AlreadyInitialized {
            description("wkhtmltopdf already initialized")
            display("Wkhtmltopdf has already been initialized")
        }

        InitThreadMismatch(before: usize, after: usize) {
            description("initialization thread mismatch")
            display("Wkhtmltopdf QApplication originally started on thread {:0x}, cannot recreate on thread {:0x}", before, after)
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