use image::io::Reader as ImageReader;
use image::ImageFormat;
use image::imageops::FilterType;

fn main() {
    let img = ImageReader::open("c:/development/cat-lock/assets/app_icon.png")
        .unwrap()
        .decode()
        .unwrap();

    let resized = img.resize_exact(256, 256, FilterType::Lanczos3);
    let mut icon_file = std::fs::File::create("c:/development/cat-lock/assets/icon.ico").unwrap();
    resized.write_to(&mut icon_file, ImageFormat::Ico).unwrap();
}
