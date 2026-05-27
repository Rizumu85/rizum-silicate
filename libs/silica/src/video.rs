use crate::error::SilicaError;
use std::io::{Cursor, Read};
use zip::ZipArchive;

const SEGMENT_PREFIX: &str = "video/segments/segment-";
const SEGMENT_SUFFIX: &str = ".mp4";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoSegment {
    pub index: u32,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoSegmentBytes {
    pub segment: ArchivedVideoSegment,
    pub bytes: Vec<u8>,
}

pub fn list_archived_video_segments(
    bytes: &[u8],
) -> Result<Vec<ArchivedVideoSegment>, SilicaError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut segments = Vec::new();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let Some(segment_index) = parse_segment_index(file.name()) else {
            continue;
        };

        segments.push(ArchivedVideoSegment {
            index: segment_index,
            path: file.name().to_owned(),
            size: file.size(),
        });
    }

    segments.sort_by(|left, right| {
        left.index
            .cmp(&right.index)
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(segments)
}

pub fn extract_archived_video_segments(
    bytes: &[u8],
) -> Result<Vec<ArchivedVideoSegmentBytes>, SilicaError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut segments = Vec::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let path = file.name().to_owned();
        let Some(segment_index) = parse_segment_index(&path) else {
            continue;
        };
        let size = file.size();
        let mut segment_bytes = Vec::with_capacity(size as usize);
        file.read_to_end(&mut segment_bytes)?;

        segments.push(ArchivedVideoSegmentBytes {
            segment: ArchivedVideoSegment {
                index: segment_index,
                path,
                size,
            },
            bytes: segment_bytes,
        });
    }

    segments.sort_by(|left, right| {
        left.segment
            .index
            .cmp(&right.segment.index)
            .then_with(|| left.segment.path.cmp(&right.segment.path))
    });

    Ok(segments)
}

fn parse_segment_index(path: &str) -> Option<u32> {
    let index = path
        .strip_prefix(SEGMENT_PREFIX)?
        .strip_suffix(SEGMENT_SUFFIX)?;

    if index.is_empty() {
        return None;
    }

    index.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn lists_video_segments_in_numeric_order() {
        let archive = zip_with_files([
            ("video/segments/segment-10.mp4", b"ten".as_slice()),
            ("video/segments/segment-2.mp4", b"two".as_slice()),
            ("video/segments/segment-1.mp4", b"one".as_slice()),
        ]);

        let segments = list_archived_video_segments(&archive).unwrap();

        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.index)
                .collect::<Vec<_>>(),
            vec![1, 2, 10]
        );
        assert_eq!(segments[0].path, "video/segments/segment-1.mp4");
        assert_eq!(segments[1].size, 3);
    }

    #[test]
    fn ignores_non_segment_files() {
        let archive = zip_with_files([
            ("video/segments/segment-1.mp4", b"one".as_slice()),
            ("video/segments/segment-preview.mp4", b"preview".as_slice()),
            ("video/segments/segment-2.txt", b"text".as_slice()),
            ("QuickLook/Preview.png", b"png".as_slice()),
        ]);

        let segments = list_archived_video_segments(&archive).unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].index, 1);
    }

    #[test]
    fn extracts_video_segment_bytes_in_numeric_order() {
        let archive = zip_with_files([
            ("video/segments/segment-10.mp4", b"ten".as_slice()),
            ("video/segments/segment-2.mp4", b"two".as_slice()),
            ("video/segments/segment-1.mp4", b"one".as_slice()),
        ]);

        let segments = extract_archived_video_segments(&archive).unwrap();

        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.segment.index)
                .collect::<Vec<_>>(),
            vec![1, 2, 10]
        );
        assert_eq!(segments[0].bytes, b"one");
        assert_eq!(segments[1].bytes, b"two");
        assert_eq!(segments[2].bytes, b"ten");
    }

    fn zip_with_files<const N: usize>(files: [(&str, &[u8]); N]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (path, bytes) in files {
            archive.start_file(path, options).unwrap();
            archive.write_all(bytes).unwrap();
        }

        archive.finish().unwrap().into_inner()
    }
}
