pub mod error;
pub mod image;
pub mod pdf;
pub use error::*;
pub use image::*;
pub use pdf::*;
//pub use pdf::Orientation;

#[cfg(test)]
mod tests {
    use super::*;
    use ImageFormat::Png;

    #[test]
    fn one_test_to_rule_them_all() {
        // Has to be a single test because PdfApplication can only be initialized once and is !Sync/!Send
        let _ = env_logger::init();
        let pdf_app = PdfApplication::new().expect("Failed to init PDF Application");

        {
            // Test building PDF from HTML
            let res = pdf_app.builder().build_from_html("basic <b>from</b> html");
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        {
            // Test building PDF from URL
            let res = pdf_app
                .builder()
                .build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap());
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        let image_app = ImageApplication::new().expect("Failed to init image Application");

        {
            // Test building image from file
            let res = image_app
                .builder()
                .format(Png)
                .build_from_path("examples/input.html");
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        {
            // Test building image from URL
            let res = image_app
                .builder()
                .format(Png)
                .build_from_url(&"https://www.rust-lang.org/en-US/".parse().unwrap());
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        {
            // Test building image from HTML string
            let res = image_app
                .builder()
                .format(Png)
                .build_from_html("basic <b>from</b> html");
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }

        /*{ // Pending https://github.com/wkhtmltopdf/wkhtmltopdf/issues/4714
            // Test cropping options
            let res = image_app
                .builder()
                .format("png")
                .screen_width(1280)
                .crop_left(20)
                .crop_top(20)
                .crop_width(800)
                .crop_height(600)
                .build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap());
            assert!(res.is_ok(), "{}", res.unwrap_err());
        }*/
    }
}
