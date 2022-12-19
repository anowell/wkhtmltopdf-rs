use wkhtmltopdf::*;

fn main() {
    env_logger::init();
    let mut image_app = ImageApplication::new().expect("Failed to init image application");

    {
        let mut out = image_app
            .builder()
            .format(ImageFormat::Png)
            .build_from_url(&"https://www.rust-lang.org/en-US/".parse().unwrap())
            .expect("failed to build image");
        out.save("image1.png").expect("failed to save image1.png");
    }

    {
        let html = r#"
        <html><body>
          <h1>Rust can haz images</h1>
          <img src="https://www.rust-lang.org/logos/rust-logo-512x512.png">
        </body></html>
      "#;

        let mut out = image_app
            .builder()
            .format(ImageFormat::Png)
            .build_from_html(html)
            .expect("failed to build image");
        out.save("image2.png").expect("failed to save image2.png");
    }
}
