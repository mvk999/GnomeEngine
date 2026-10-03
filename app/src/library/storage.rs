use std::{fs, path::PathBuf};

use uuid::Uuid;

use super::{model::WallpaperManifest, Wallpaper};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Library {
    root: PathBuf,
    wallpapers: Vec<Wallpaper>,
    invalid_items: usize,
}

impl Library {
    pub fn load_default() -> Result<Self, String> {
        Self::load(data_root())
    }

    pub fn empty_default() -> Self {
        Self {
            root: data_root(),
            wallpapers: Vec::new(),
            invalid_items: 0,
        }
    }

    pub fn load(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| format!("create library directory: {error}"))?;
        let root = root
            .canonicalize()
            .map_err(|error| format!("resolve library directory: {error}"))?;
        let mut wallpapers = Vec::new();
        let mut invalid_items = 0;

        for entry in fs::read_dir(&root).map_err(|error| format!("read library: {error}"))? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    invalid_items += 1;
                    eprintln!("gnomeengine: skipped library entry: {error}");
                    continue;
                }
            };
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".import-"))
            {
                if fs::symlink_metadata(&path)
                    .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
                {
                    if let Err(error) = fs::remove_dir_all(&path) {
                        eprintln!(
                            "gnomeengine: could not remove stale import staging directory: {error}"
                        );
                        invalid_items += 1;
                    }
                }
                continue;
            }
            match load_item(&root, &path) {
                Ok(Some(wallpaper)) => wallpapers.push(wallpaper),
                Ok(None) => {}
                Err(error) => {
                    invalid_items += 1;
                    eprintln!("gnomeengine: skipped invalid wallpaper: {error}");
                }
            }
        }
        wallpapers.sort_by(|left, right| left.manifest.title.cmp(&right.manifest.title));
        Ok(Self {
            root,
            wallpapers,
            invalid_items,
        })
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub fn wallpapers(&self) -> &[Wallpaper] {
        &self.wallpapers
    }

    pub fn invalid_items(&self) -> usize {
        self.invalid_items
    }

    pub fn include_imported(&mut self, wallpaper: Wallpaper) -> Result<(), String> {
        let id = wallpaper.manifest.id.clone();
        if !super::model::valid_id(&id) {
            return Err("imported wallpaper has an invalid ID".to_owned());
        }
        let validated = load_item_for_import(&self.root, &self.root.join(&id))?;
        if validated.content_path != wallpaper.content_path {
            return Err("imported wallpaper path does not match its library entry".to_owned());
        }
        if let Some(existing) = self
            .wallpapers
            .iter_mut()
            .find(|existing| existing.manifest.id == id)
        {
            *existing = validated;
        } else {
            self.wallpapers.push(validated);
        }
        self.wallpapers
            .sort_by(|left, right| left.manifest.title.cmp(&right.manifest.title));
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if !super::model::valid_id(id) {
            return Err("invalid wallpaper ID".to_owned());
        }
        let item_path = self.root.join(id);
        let metadata = fs::symlink_metadata(&item_path)
            .map_err(|error| format!("wallpaper entry is unavailable: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("wallpaper entry is not a managed directory".to_owned());
        }
        let canonical = item_path
            .canonicalize()
            .map_err(|error| format!("resolve wallpaper directory: {error}"))?;
        if !canonical.starts_with(&self.root) || canonical.parent() != Some(self.root.as_path()) {
            return Err("wallpaper directory escapes the library root".to_owned());
        }
        let manifest_path = canonical.join("manifest.json");
        let manifest: WallpaperManifest = serde_json::from_slice(
            &fs::read(&manifest_path).map_err(|error| format!("read manifest: {error}"))?,
        )
        .map_err(|error| format!("parse manifest: {error}"))?;
        manifest.validate()?;
        if manifest.id != id {
            return Err("wallpaper ID does not match its library directory".to_owned());
        }
        fs::remove_dir_all(&canonical)
            .map_err(|error| format!("remove wallpaper from library: {error}"))?;
        self.wallpapers
            .retain(|wallpaper| wallpaper.manifest.id != id);
        Ok(())
    }
}

pub fn data_root() -> PathBuf {
    glib::user_data_dir().join("gnomeengine").join("wallpapers")
}

pub(super) fn load_item_for_import(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<Wallpaper, String> {
    load_item(root, path)?.ok_or_else(|| "imported wallpaper is not a directory".to_owned())
}

fn load_item(root: &std::path::Path, path: &std::path::Path) -> Result<Option<Wallpaper>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(None);
    }
    let directory = path
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper directory: {error}"))?;
    if !directory.starts_with(root) {
        return Err("wallpaper directory escapes the library root".to_owned());
    }
    let manifest_path = directory.join("manifest.json");
    let manifest_meta = fs::symlink_metadata(&manifest_path)
        .map_err(|error| format!("manifest is unavailable: {error}"))?;
    if manifest_meta.file_type().is_symlink()
        || !manifest_meta.is_file()
        || manifest_meta.len() > MAX_MANIFEST_BYTES
    {
        return Err("manifest is not a regular file or exceeds the size limit".to_owned());
    }
    let manifest: WallpaperManifest = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|error| format!("read manifest: {error}"))?,
    )
    .map_err(|error| format!("parse manifest: {error}"))?;
    manifest.validate()?;
    if directory.file_name().and_then(|name| name.to_str()) != Some(manifest.id.as_str()) {
        return Err("manifest ID does not match its library directory".to_owned());
    }
    let candidate = directory.join(&manifest.content.entry);
    let content_meta = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("wallpaper content is unavailable: {error}"))?;
    if content_meta.file_type().is_symlink() || !content_meta.is_file() {
        return Err("wallpaper content must be a regular managed file".to_owned());
    }
    let content_path = candidate
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper content: {error}"))?;
    if !content_path.starts_with(&directory) {
        return Err("wallpaper content escapes its managed directory".to_owned());
    }
    Ok(Some(Wallpaper {
        manifest,
        content_path,
        thumbnail_path: directory.join("thumbnail.png"),
    }))
}

pub fn new_wallpaper_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::{load_item, load_item_for_import, new_wallpaper_id, Library};
    use crate::library::model::{ContentManifest, MediaMetadata, WallpaperManifest};
    use std::{fs, path::Path};

    #[test]
    fn ids_are_unique_and_safe_for_directory_names() {
        let first = new_wallpaper_id();
        let second = new_wallpaper_id();
        assert_ne!(first, second);
        assert_eq!(first.len(), 36);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'));
    }

    #[test]
    fn missing_and_corrupt_items_are_rejected_without_panicking() {
        let temp = std::env::temp_dir().join(format!("gnomeengine-test-{}", new_wallpaper_id()));
        fs::create_dir_all(&temp).unwrap();
        assert!(load_item(&temp, &temp.join("missing")).is_err());
        fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn rejects_content_symlink_even_when_target_is_inside_directory() {
        let temp = std::env::temp_dir().join(format!("gnomeengine-test-{}", new_wallpaper_id()));
        let item = temp.join("c92c676a-1f60-4c21-9a84-f97e6b6e6fe0");
        fs::create_dir_all(item.join("content")).unwrap();
        fs::write(item.join("actual.mp4"), b"fixture").unwrap();
        let manifest = WallpaperManifest {
            schema_version: 1,
            id: "c92c676a-1f60-4c21-9a84-f97e6b6e6fe0".to_owned(),
            title: "Fixture".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-02T00:00:00Z".to_owned(),
            content: ContentManifest {
                entry: "content/wallpaper.mp4".to_owned(),
            },
            media: MediaMetadata::default(),
        };
        fs::write(
            item.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(item.join("actual.mp4"), item.join("content/wallpaper.mp4"))
            .unwrap();

        #[cfg(unix)]
        assert!(load_item(&temp, Path::new(&item)).is_err());
        fs::remove_dir_all(&temp).unwrap();
    }

    #[test]
    fn removal_deletes_only_the_managed_copy() {
        let temp = std::env::temp_dir().join(format!("gnomeengine-remove-{}", new_wallpaper_id()));
        let source = temp.join("original.mp4");
        fs::create_dir_all(&temp).unwrap();
        fs::write(&source, b"original").unwrap();
        let root = temp.join("library").join("wallpapers");
        let id = new_wallpaper_id();
        let item = root.join(&id);
        fs::create_dir_all(item.join("content")).unwrap();
        fs::write(item.join("content/wallpaper.mp4"), b"managed copy").unwrap();
        let manifest = WallpaperManifest {
            schema_version: 1,
            id: id.clone(),
            title: "Fixture".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-02T00:00:00Z".to_owned(),
            content: ContentManifest {
                entry: "content/wallpaper.mp4".to_owned(),
            },
            media: MediaMetadata::default(),
        };
        fs::write(
            item.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let mut library = Library::load(root).unwrap();
        library.remove(&id).unwrap();

        assert!(library.wallpapers().is_empty());
        assert!(source.is_file());
        assert!(!item.exists());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn newly_imported_item_is_added_to_the_live_library_without_rescan() {
        let temp = std::env::temp_dir().join(format!("gnomeengine-live-{}", new_wallpaper_id()));
        let root = temp.join("library").join("wallpapers");
        fs::create_dir_all(&root).unwrap();
        let mut library = Library::load(root.clone()).unwrap();
        assert!(library.wallpapers().is_empty());

        let id = new_wallpaper_id();
        let item = root.join(&id);
        fs::create_dir_all(item.join("content")).unwrap();
        fs::write(item.join("content/wallpaper"), b"managed video").unwrap();
        let manifest = WallpaperManifest {
            schema_version: 1,
            id: id.clone(),
            title: "Live import".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-02T00:00:00Z".to_owned(),
            content: ContentManifest {
                entry: "content/wallpaper".to_owned(),
            },
            media: MediaMetadata::default(),
        };
        fs::write(
            item.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let wallpaper = load_item_for_import(&root, &item).unwrap();

        library.include_imported(wallpaper).unwrap();

        assert_eq!(library.wallpapers().len(), 1);
        assert_eq!(library.wallpapers()[0].manifest.id, id);
        fs::remove_dir_all(temp).unwrap();
    }
}
