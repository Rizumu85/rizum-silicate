pub fn from_premultiplied_rgba(
    mut image: image::RgbaImage,
    orientation: silica::Orientation,
) -> image::RgbaImage {
    // The live WGPU target stays premultiplied for direct presentation. File codecs expect
    // straight alpha, so conversion belongs at this boundary instead of interactive frames.
    unpremultiply_rgba(&mut image);

    match orientation {
        silica::Orientation::NoRotation | silica::Orientation::Unknown => image,
        silica::Orientation::Clockwise90 => image::imageops::rotate90(&image),
        silica::Orientation::Clockwise180 => image::imageops::rotate180(&image),
        silica::Orientation::Clockwise270 => image::imageops::rotate270(&image),
    }
}

fn unpremultiply_rgba(image: &mut image::RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 {
            pixel.0 = [0, 0, 0, 0];
            continue;
        }
        if alpha == 255 {
            continue;
        }

        for channel in &mut pixel.0[..3] {
            let straight = (u32::from(*channel) * 255 + alpha / 2) / alpha;
            *channel = straight.min(255) as u8;
        }
    }
}
