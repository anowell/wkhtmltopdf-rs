use wkhtmltopdf::*;

fn main() {
	env_logger::init();
	let image_app = ImageApplication::new().expect("Failed to init image application");

	let mut out = image_app.builder()
		.format("png")
		.build_from_url("https://www.rust-lang.org/en-US/".parse().unwrap())
		.expect("failed to build image");
	out.save("image.png").expect("failed to save image.png");
}