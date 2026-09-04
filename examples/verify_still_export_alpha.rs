use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use silicate::export::still::from_premultiplied_rgba;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut premultiplied = RgbaImage::new(3, 1);
    premultiplied.put_pixel(0, 0, Rgba([128, 0, 0, 128]));
    premultiplied.put_pixel(1, 0, Rgba([12, 34, 56, 255]));
    premultiplied.put_pixel(2, 0, Rgba([10, 10, 10, 0]));
    let image = from_premultiplied_rgba(premultiplied, silica::Orientation::NoRotation);
    expect_pixel(image.get_pixel(0, 0).0, [255, 0, 0, 128], "export image")?;
    expect_pixel(image.get_pixel(1, 0).0, [12, 34, 56, 255], "opaque pixel")?;
    expect_pixel(image.get_pixel(2, 0).0, [0, 0, 0, 0], "transparent pixel")?;

    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut encoded, ImageFormat::Png)?;
    let decoded =
        image::load_from_memory_with_format(encoded.get_ref(), ImageFormat::Png)?.into_rgba8();
    let decoded_pixel = decoded.get_pixel(0, 0).0;
    expect_pixel(decoded_pixel, [255, 0, 0, 128], "PNG round trip")?;

    println!("verification=still_export_alpha_v1");
    println!("png_rgba={decoded_pixel:?}");
    Ok(())
}

fn expect_pixel(
    actual: [u8; 4],
    expected: [u8; 4],
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if actual != expected {
        return Err(format!("{label} pixel was {actual:?}, expected {expected:?}").into());
    }
    Ok(())
}
