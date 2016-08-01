

use std::{io, result};

quick_error! {
    /// The error type for wkhtmltopdf generation
    #[derive(Debug)]
    pub enum Error {
        /// Indicates an I/O error that occurred during PDF generation
        IoError(err: io::Error) {
            description("io error")
            display("I/O error: {}", err)
            cause(err)
            from()
        }

        /// Indicates the wkhtmltopdf could be be initialized because it can only be initialized once per process (wkhtmltopdf limitation)
        IllegalInit {
            description("illegal initialization")
            display("Wkhtmltopdf may not be initialized more than once per process")
        }

        /// Indicates that wkhtmltopdf has not yet been initialized in this process
        NotInitialized {
            description("not initialized")
            display("Wkhtmltopdf is not currently initialized")
        }

        /// Indicates that wkhtmltopdf is blocked by another request within this process (wkhtmltopdf limitation)
        Blocked {
            description("wkhtmltopdf blocked")
            display("Wkhtmltopdf is currently blocked by another initialized instance")
        }

        /// Indicates that wkhtmltopdf was initialized on a different thread than this PDF generation atttempt (wkhtmltopdf limitation)
        ThreadMismatch(before: usize, after: usize) {
            description("thread mismatch")
            display("Wkhtmltopdf originally started on thread {:0x}, cannot recreate on thread {:0x}", before, after)
        }


        /// Indicates that wkhtmltopdf conversion failed - internal error message comes directly from wkhtmltopdf
        ConversionFailed(msg: String) {
            description("conversion failed")
            display("Conversion failed: {}", msg)
        }

        /// Indicates that wkhtmltopdf failed to set a particular global setting
        GlobalSettingFailure(name: String, value: String) {
            description("global setting failure")
            display("Failed to update global setting '{}'='{}'", name, value)
        }

        /// Indicates that wkhtmltopdf failed to set a particular object setting
        ObjectSettingFailure(name: String, value: String) {
            description("object setting failure")
            display("Failed to update object setting '{}'='{}'", name, value)
        }
    }
}

/// A specialized `Result` type for wkhtmltopdf generation
pub type Result<T> = result::Result<T, Error>;