use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub codec: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperManifest {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub wallpaper_type: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub content: ContentManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub media: MediaMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContentManifest {
    pub entry: String,
}

#[derive(Clone, Debug)]
pub struct Wallpaper {
    pub manifest: WallpaperManifest,
    pub content_path: PathBuf,
    pub thumbnail_path: PathBuf,
    pub origin: WallpaperOrigin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WallpaperOrigin {
    BuiltIn,
    User,
}

impl WallpaperOrigin {
    pub fn is_read_only(self) -> bool {
        self == Self::BuiltIn
    }
}

impl WallpaperManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "unsupported wallpaper manifest schema version {}",
                self.schema_version
            ));
        }
        if !valid_id(&self.id) {
            return Err("wallpaper manifest has an invalid ID".to_owned());
        }
        if self.wallpaper_type != "video" {
            return Err("unsupported wallpaper type".to_owned());
        }
        if self.title.trim().is_empty() || self.title.len() > 256 {
            return Err("wallpaper title must contain 1 to 256 bytes".to_owned());
        }
        validate_relative_entry(Path::new(&self.content.entry))?;
        if let Some(thumbnail) = &self.thumbnail {
            validate_relative_entry(Path::new(thumbnail))?;
        }
        Ok(())
    }
}

pub fn valid_id(id: &str) -> bool {
    (8..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub fn validate_relative_entry(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("wallpaper entry must be a non-empty relative path".to_owned());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("wallpaper entry escapes its managed directory".to_owned());
    }
    Ok(())
}

pub fn title_from_filename(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Untitled wallpaper")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        title_from_filename, valid_id, validate_relative_entry, ContentManifest, MediaMetadata,
        WallpaperManifest,
    };
    use std::path::Path;

    fn valid_manifest() -> WallpaperManifest {
        WallpaperManifest {
            schema_version: 1,
            id: "c92c676a-1f60-4c21-9a84-f97e6b6e6fe0".to_owned(),
            title: "Aurora".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-02T00:00:00Z".to_owned(),
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "content/wallpaper.mp4".to_owned(),
            },
            thumbnail: None,
            media: MediaMetadata::default(),
        }
    }

    #[test]
    fn accepts_version_one_video_manifest() {
        assert!(valid_manifest().validate().is_ok());
    }

    #[test]
    fn rejects_unknown_schema_versions_and_types() {
        let mut manifest = valid_manifest();
        manifest.schema_version = 2;
        assert!(manifest.validate().unwrap_err().contains("schema version"));

        manifest = valid_manifest();
        manifest.wallpaper_type = "web".to_owned();
        assert!(manifest.validate().unwrap_err().contains("type"));
    }

    #[test]
    fn rejects_invalid_identifiers_and_parent_paths() {
        assert!(!valid_id("../../video"));
        assert!(validate_relative_entry(Path::new("../escape.mp4")).is_err());
        assert!(validate_relative_entry(Path::new("/tmp/video.mp4")).is_err());
        assert!(validate_relative_entry(Path::new("content/video.mp4")).is_ok());
        let mut manifest = valid_manifest();
        manifest.thumbnail = Some("../outside.webp".to_owned());
        assert!(manifest.validate().unwrap_err().contains("escapes"));
    }

    #[test]
    fn legacy_schema_one_manifest_without_optional_metadata_remains_valid() {
        let json = r#"{
            "schemaVersion":1,
            "id":"c92c676a-1f60-4c21-9a84-f97e6b6e6fe0",
            "title":"Existing import",
            "type":"video",
            "createdAt":"2026-10-02T00:00:00Z",
            "content":{"entry":"content/wallpaper.mp4"},
            "media":{}
        }"#;
        let manifest: WallpaperManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.validate().is_ok());
        assert_eq!(manifest.thumbnail, None);
        assert_eq!(manifest.author, None);
    }

    #[test]
    fn derives_title_from_filename_without_extension() {
        assert_eq!(
            title_from_filename(Path::new("tokyo-night.mp4")),
            "tokyo-night"
        );
    }
}
