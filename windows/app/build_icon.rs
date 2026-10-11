// Repackage the macOS artwork without decoding or redrawing it.
pub fn icon() -> Vec<u8> {
    let source = include_bytes!("../../Resources/AppIcon.icns");
    let mut images = std::collections::BTreeMap::new();
    let mut offset = 8;
    while offset < source.len() {
        let length =
            u32::from_be_bytes(source[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let image = &source[offset + 8..offset + length];
        if image.starts_with(b"\x89PNG\r\n\x1a\n") {
            let size = u32::from_be_bytes(image[16..20].try_into().unwrap());
            if size <= 256 {
                images.insert(size, image);
            }
        }
        offset += length;
    }
    assert!(
        !images.is_empty(),
        "macOS icon must contain Windows-sized PNGs"
    );
    let mut result = vec![0, 0, 1, 0];
    result.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (&size, image) in &images {
        result.extend([size as u8, size as u8, 0, 0]);
        result.extend(1u16.to_le_bytes());
        result.extend(32u16.to_le_bytes());
        result.extend((image.len() as u32).to_le_bytes());
        result.extend((offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in images.values() {
        result.extend_from_slice(image);
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn ico_preserves_macos_png_artwork() {
        let ico = super::icon();
        let source = include_bytes!("../../Resources/AppIcon.icns");
        assert_eq!(&ico[..4], &[0, 0, 1, 0]);
        let count = u16::from_le_bytes(ico[4..6].try_into().unwrap()) as usize;
        assert!(count >= 4);
        for entry in ico[6..6 + count * 16].chunks_exact(16) {
            let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            let png = &ico[offset..offset + length];
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
            assert!(source.windows(png.len()).any(|bytes| bytes == png));
            let size = u32::from_be_bytes(png[16..20].try_into().unwrap());
            assert_eq!(entry[0], size as u8);
            assert_eq!(entry[1], size as u8);
        }
    }
}
