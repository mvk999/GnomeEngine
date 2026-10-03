use std::{fs, path::Path, path::PathBuf};

use gstreamer as gst;
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_video::VideoInfo;

use super::{
    model::{title_from_filename, ContentManifest, MediaMetadata, WallpaperManifest},
    storage::{load_item_for_import, new_wallpaper_id},
    Wallpaper,
};

const MAX_DISCOVERY_SECONDS: u64 = 12;
const SAMPLE_TIMEOUT_SECONDS: u64 = 18;
const MAX_THUMBNAIL_EDGE: u32 = 512;

struct StagingDirectory(PathBuf);

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

pub fn discover_video(path: &Path) -> Result<MediaMetadata, String> {
    gst::init().map_err(|error| format!("initialize GStreamer: {error}"))?;
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("read selected file: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("selected item must be a regular local file".to_owned());
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("resolve selected video: {error}"))?;
    let uri = gst::glib::filename_to_uri(&path, None)
        .map_err(|error| format!("create local media URI: {error}"))?;
    let discoverer =
        gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(MAX_DISCOVERY_SECONDS))
            .map_err(|error| format!("create media discoverer: {error}"))?;
    let info = discoverer
        .discover_uri(&uri)
        .map_err(|error| format!("could not inspect media: {error}"))?;
    if info.result() != gstreamer_pbutils::DiscovererResult::Ok {
        return Err(format!(
            "GStreamer could not decode this media ({:?})",
            info.result()
        ));
    }
    let video = info
        .video_streams()
        .into_iter()
        .next()
        .ok_or_else(|| "selected file does not contain a video stream".to_owned())?;
    let fps = video.framerate();
    let fps = (fps.denom() > 0).then(|| fps.numer() as f64 / fps.denom() as f64);
    let codec = video
        .caps()
        .map(|caps| gstreamer_pbutils::pb_utils_get_codec_description(caps.as_ref()).to_string());

    Ok(MediaMetadata {
        width: (video.width() > 0).then_some(video.width()),
        height: (video.height() > 0).then_some(video.height()),
        fps: fps.filter(|value| value.is_finite() && *value > 0.0),
        duration_seconds: info
            .duration()
            .map(|duration| duration.nseconds() as f64 / 1_000_000_000.0)
            .filter(|value| value.is_finite() && *value > 0.0),
        codec,
    })
}

pub fn import_video(source: &Path, library_root: &Path) -> Result<Wallpaper, String> {
    let media = discover_video(source)?;
    let source_metadata =
        fs::symlink_metadata(source).map_err(|error| format!("read selected file: {error}"))?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        return Err("selected item must be a regular local file".to_owned());
    }
    let source = source
        .canonicalize()
        .map_err(|error| format!("resolve selected video: {error}"))?;
    fs::create_dir_all(library_root)
        .map_err(|error| format!("create wallpaper library: {error}"))?;
    let root = library_root
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper library: {error}"))?;

    let id = new_wallpaper_id();
    let stage_path = root.join(format!(".import-{id}"));
    fs::create_dir(&stage_path)
        .map_err(|error| format!("create import staging directory: {error}"))?;
    let _staging = StagingDirectory(stage_path.clone());
    let content_dir = stage_path.join("content");
    fs::create_dir(&content_dir).map_err(|error| format!("create content directory: {error}"))?;
    let content_path = content_dir.join("wallpaper");
    fs::copy(&source, &content_path)
        .map_err(|error| format!("copy wallpaper into library: {error}"))?;

    match generate_thumbnail(&content_path, &media, &stage_path.join("thumbnail.png")) {
        Ok(()) => {}
        Err(error) => eprintln!("gnomeengine: thumbnail unavailable for import {id}: {error}"),
    }

    let created_at = glib::DateTime::now_utc()
        .and_then(|now| now.format_iso8601())
        .map_err(|error| format!("create import timestamp: {error}"))?
        .to_string();
    let manifest = WallpaperManifest {
        schema_version: super::model::MANIFEST_SCHEMA_VERSION,
        id: id.clone(),
        title: title_from_filename(&source),
        wallpaper_type: "video".to_owned(),
        created_at,
        author: None,
        copyright: None,
        license: None,
        content: ContentManifest {
            entry: "content/wallpaper".to_owned(),
        },
        thumbnail: None,
        media,
    };
    manifest.validate()?;
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("serialize wallpaper manifest: {error}"))?;
    fs::write(stage_path.join("manifest.json"), manifest_bytes)
        .map_err(|error| format!("write wallpaper manifest: {error}"))?;

    let final_path = root.join(&id);
    fs::rename(&stage_path, &final_path)
        .map_err(|error| format!("commit imported wallpaper: {error}"))?;
    load_item_for_import(&root, &final_path)
}

fn generate_thumbnail(
    source: &Path,
    media: &MediaMetadata,
    destination: &Path,
) -> Result<(), String> {
    let source = source
        .canonicalize()
        .map_err(|error| format!("resolve staged media: {error}"))?;
    let uri = gst::glib::filename_to_uri(&source, None)
        .map_err(|error| format!("create staged media URI: {error}"))?;
    let (width, height) = thumbnail_dimensions(media);
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "RGBA")
        .field("width", width as i32)
        .field("height", height as i32)
        .build();
    let sink = AppSink::builder()
        .caps(&caps)
        .sync(false)
        .max_buffers(1)
        .drop(true)
        .build();
    let pipeline = gst::ElementFactory::make("playbin")
        .build()
        .map_err(|error| format!("create thumbnail pipeline: {error}"))?;
    let audio_sink = gst::ElementFactory::make("fakesink")
        .build()
        .map_err(|error| format!("create muted thumbnail sink: {error}"))?;
    pipeline.set_property("uri", uri);
    pipeline.set_property("video-sink", &sink);
    pipeline.set_property("audio-sink", audio_sink);

    let operation = (|| {
        pipeline
            .set_state(gst::State::Paused)
            .map_err(|error| format!("preroll thumbnail pipeline: {error}"))?;
        let _ = pipeline.state(Some(gst::ClockTime::from_seconds(8)));
        if let Some(duration) = media.duration_seconds {
            let position = (duration * 0.15 * 1_000_000_000.0) as u64;
            let _ = pipeline.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_nseconds(position),
            );
        }
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|error| format!("start thumbnail decode: {error}"))?;
        let sample = sink
            .try_pull_sample(gst::ClockTime::from_seconds(SAMPLE_TIMEOUT_SECONDS))
            .ok_or_else(|| "timed out decoding a thumbnail frame".to_owned())?;
        let sample_caps = sample
            .caps()
            .ok_or_else(|| "thumbnail frame has no caps".to_owned())?;
        let info = VideoInfo::from_caps(sample_caps)
            .map_err(|error| format!("read thumbnail frame layout: {error}"))?;
        let buffer = sample
            .buffer()
            .ok_or_else(|| "thumbnail frame has no pixel buffer".to_owned())?;
        let map = buffer
            .map_readable()
            .map_err(|_| "could not read thumbnail pixels".to_owned())?;
        let row_bytes = info.width() as usize * 4;
        let stride = *info
            .stride()
            .first()
            .ok_or_else(|| "thumbnail frame has no row stride".to_owned())?
            as usize;
        if stride < row_bytes {
            return Err("thumbnail frame row stride is too short".to_owned());
        }
        let raw = map.as_slice();
        let mut packed = Vec::with_capacity(row_bytes * info.height() as usize);
        for row in 0..info.height() as usize {
            let start = row * stride;
            let end = start + row_bytes;
            let row_data = raw
                .get(start..end)
                .ok_or_else(|| "thumbnail frame pixel data is truncated".to_owned())?;
            packed.extend_from_slice(row_data);
        }

        let file = fs::File::create(destination)
            .map_err(|error| format!("create thumbnail image: {error}"))?;
        let mut encoder = png::Encoder::new(file, info.width(), info.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&packed))
            .map_err(|error| format!("encode thumbnail: {error}"))?;
        Ok(())
    })();
    let _ = pipeline.set_state(gst::State::Null);
    operation
}

fn thumbnail_dimensions(media: &MediaMetadata) -> (u32, u32) {
    let (Some(width), Some(height)) = (media.width, media.height) else {
        return (MAX_THUMBNAIL_EDGE, MAX_THUMBNAIL_EDGE);
    };
    if width == 0 || height == 0 {
        return (MAX_THUMBNAIL_EDGE, MAX_THUMBNAIL_EDGE);
    }
    let scale = (MAX_THUMBNAIL_EDGE as f64 / width as f64)
        .min(MAX_THUMBNAIL_EDGE as f64 / height as f64)
        .min(1.0);
    (
        ((width as f64 * scale).round() as u32).max(1),
        ((height as f64 * scale).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::thumbnail_dimensions;
    use super::MediaMetadata;

    #[test]
    fn thumbnail_dimensions_preserve_aspect_and_bound_memory() {
        assert_eq!(
            thumbnail_dimensions(&MediaMetadata {
                width: Some(1920),
                height: Some(1080),
                ..MediaMetadata::default()
            }),
            (512, 288)
        );
        assert_eq!(
            thumbnail_dimensions(&MediaMetadata {
                width: Some(320),
                height: Some(240),
                ..MediaMetadata::default()
            }),
            (320, 240)
        );
    }
}
