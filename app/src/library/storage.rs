use std::{fs, path::PathBuf};

use uuid::Uuid;

use super::{
    model::{WallpaperManifest, WallpaperOrigin},
    Wallpaper,
};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Library {
    root: PathBuf,
    built_in_roots: Vec<PathBuf>,
    wallpapers: Vec<Wallpaper>,
    invalid_items: usize,
}

impl Library {
    pub fn load_default() -> Result<Self, String> {
        Self::load_with_builtins(data_root(), built_in_roots())
    }

    pub fn empty_default() -> Self {
        Self {
            root: data_root(),
            built_in_roots: Vec::new(),
            wallpapers: Vec::new(),
            invalid_items: 0,
        }
    }

    fn load_with_builtins(root: PathBuf, built_in_roots: Vec<PathBuf>) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| format!("create library directory: {error}"))?;
        let root = root
            .canonicalize()
            .map_err(|error| format!("resolve library directory: {error}"))?;
        let configured_built_in_roots = built_in_roots.clone();
        let mut wallpapers = Vec::new();
        let mut invalid_items = 0;

        // System assets have precedence over user assets. A package-provided ID
        // cannot be shadowed by an imported directory with the same ID.
        let mut seen_ids = std::collections::HashSet::new();
        let mut visited_roots = std::collections::HashSet::new();
        for candidate_root in built_in_roots {
            let Ok(built_in_root) = candidate_root.canonicalize() else {
                continue;
            };
            if !visited_roots.insert(built_in_root.clone()) {
                continue;
            }
            let entries = match fs::read_dir(&built_in_root) {
                Ok(entries) => entries,
                Err(error) => {
                    eprintln!(
                        "gnomeengine: could not read built-in wallpaper directory {}: {error}",
                        built_in_root.display()
                    );
                    invalid_items += 1;
                    continue;
                }
            };
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        eprintln!("gnomeengine: could not inspect built-in wallpaper: {error}");
                        invalid_items += 1;
                        continue;
                    }
                };
                match load_built_in_item(&built_in_root, &entry.path()) {
                    Ok(Some(wallpaper)) => {
                        let id = wallpaper.manifest.id.clone();
                        if !seen_ids.insert(id.clone()) {
                            eprintln!("gnomeengine: skipped duplicate built-in wallpaper ID {id}");
                            invalid_items += 1;
                        } else {
                            wallpapers.push(wallpaper);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("gnomeengine: failed to load built-in wallpaper: {error}");
                        invalid_items += 1;
                    }
                }
            }
        }

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
                Ok(Some(wallpaper)) => {
                    let id = wallpaper.manifest.id.clone();
                    if !seen_ids.insert(id.clone()) {
                        eprintln!("gnomeengine: skipped user wallpaper with duplicate ID {id}");
                        invalid_items += 1;
                    } else {
                        wallpapers.push(wallpaper);
                    }
                }
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
            built_in_roots: configured_built_in_roots,
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

    pub fn reload(&self) -> Result<Self, String> {
        Self::load_with_builtins(self.root.clone(), self.built_in_roots.clone())
    }

    pub fn include_imported(&mut self, wallpaper: Wallpaper) -> Result<(), String> {
        if wallpaper.origin != WallpaperOrigin::User {
            return Err("only user wallpapers can be imported into the live library".to_owned());
        }
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
            if existing.origin.is_read_only() {
                return Err("wallpaper ID conflicts with a built-in wallpaper".to_owned());
            }
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
        if self
            .wallpapers
            .iter()
            .any(|wallpaper| wallpaper.manifest.id == id && wallpaper.origin.is_read_only())
        {
            return Err("built-in wallpapers cannot be removed".to_owned());
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

fn built_in_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    // cargo run/build binaries live under the repository's target directory.
    // Discover assets relative to the executable, never embedding a developer path.
    if let Ok(executable) = std::env::current_exe() {
        if let Some(repository) = executable.ancestors().find(|ancestor| {
            ancestor.join("Cargo.toml").is_file() && ancestor.join("assets/wallpapers").is_dir()
        }) {
            roots.push(repository.join("assets/wallpapers"));
        }
    }
    let system_data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .filter(|paths| !paths.is_empty())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    roots.extend(
        system_data_dirs
            .into_iter()
            .map(|directory| directory.join("gnomeengine/wallpapers")),
    );
    roots
}

pub(super) fn load_item_for_import(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<Wallpaper, String> {
    load_item(root, path)?.ok_or_else(|| "imported wallpaper is not a directory".to_owned())
}

fn load_item(root: &std::path::Path, path: &std::path::Path) -> Result<Option<Wallpaper>, String> {
    load_item_from_directory(root, path, WallpaperOrigin::User, true, false)
}

fn load_built_in_item(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<Option<Wallpaper>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(None);
    }
    let directory = path
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper directory: {error}"))?;
    if !directory.starts_with(root) {
        return Err("built-in wallpaper directory escapes its data root".to_owned());
    }
    let manifest_path = directory.join("manifest.json");
    // An incomplete source checkout (manifest present, video not yet supplied)
    // is intentionally ignored during development. Package validation handles
    // release readiness separately.
    let manifest_metadata = match fs::symlink_metadata(&manifest_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("inspect built-in manifest: {error}")),
    };
    if manifest_metadata.file_type().is_symlink()
        || !manifest_metadata.is_file()
        || manifest_metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err("built-in manifest is not a regular file or exceeds the size limit".to_owned());
    }
    load_item_from_directory(root, &directory, WallpaperOrigin::BuiltIn, false, true)
}

fn load_item_from_directory(
    root: &std::path::Path,
    path: &std::path::Path,
    origin: WallpaperOrigin,
    require_id_directory: bool,
    allow_incomplete: bool,
) -> Result<Option<Wallpaper>, String> {
    // Share the same validator and containment checks. Built-in directories are
    // human-readable (e.g. starter/) while user directories are ID-named.
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(None);
    }
    let directory = path
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper directory: {error}"))?;
    if !directory.starts_with(root) {
        return Err("wallpaper directory escapes its data root".to_owned());
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
    if require_id_directory
        && directory.file_name().and_then(|name| name.to_str()) != Some(manifest.id.as_str())
    {
        return Err("manifest ID does not match its library directory".to_owned());
    }
    let candidate = directory.join(&manifest.content.entry);
    let content_meta = match fs::symlink_metadata(&candidate) {
        Ok(metadata) => metadata,
        Err(error) if allow_incomplete && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(format!("wallpaper content is unavailable: {error}")),
    };
    if content_meta.file_type().is_symlink() || !content_meta.is_file() {
        return Err("wallpaper content must be a regular managed file".to_owned());
    }
    let content_path = candidate
        .canonicalize()
        .map_err(|error| format!("resolve wallpaper content: {error}"))?;
    if !content_path.starts_with(&directory) {
        return Err("wallpaper content escapes its managed directory".to_owned());
    }
    if allow_incomplete
        && fs::symlink_metadata(
            directory.join(manifest.thumbnail.as_deref().unwrap_or("thumbnail.png")),
        )
        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(None);
    }
    let thumbnail_path =
        resolve_thumbnail(&directory, &manifest, origin == WallpaperOrigin::BuiltIn)?;
    Ok(Some(Wallpaper {
        manifest,
        content_path,
        thumbnail_path,
        origin,
    }))
}

fn resolve_thumbnail(
    directory: &std::path::Path,
    manifest: &WallpaperManifest,
    require_existing: bool,
) -> Result<PathBuf, String> {
    let candidate = directory.join(manifest.thumbnail.as_deref().unwrap_or("thumbnail.png"));
    match fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err("wallpaper thumbnail must be a regular file".to_owned());
            }
            let canonical = candidate
                .canonicalize()
                .map_err(|error| format!("resolve wallpaper thumbnail: {error}"))?;
            if !canonical.starts_with(directory) {
                return Err("wallpaper thumbnail escapes its wallpaper directory".to_owned());
            }
            Ok(canonical)
        }
        Err(error) if !require_existing && error.kind() == std::io::ErrorKind::NotFound => {
            Ok(candidate)
        }
        Err(error) => Err(format!("wallpaper thumbnail is unavailable: {error}")),
    }
}

pub fn new_wallpaper_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::{load_item, load_item_for_import, new_wallpaper_id, Library};
    use crate::library::model::{
        ContentManifest, MediaMetadata, WallpaperManifest, WallpaperOrigin,
    };
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
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "content/wallpaper.mp4".to_owned(),
            },
            thumbnail: None,
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
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "content/wallpaper.mp4".to_owned(),
            },
            thumbnail: None,
            media: MediaMetadata::default(),
        };
        fs::write(
            item.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let mut library = Library::load_with_builtins(root, Vec::new()).unwrap();
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
        let mut library = Library::load_with_builtins(root.clone(), Vec::new()).unwrap();
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
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "content/wallpaper".to_owned(),
            },
            thumbnail: None,
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
        assert_eq!(library.wallpapers()[0].origin, WallpaperOrigin::User);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn discovers_built_in_from_named_directory_and_keeps_it_read_only() {
        let temp = std::env::temp_dir().join(format!("gnomeengine-builtin-{}", new_wallpaper_id()));
        let built_in_root = temp.join("system/gnomeengine/wallpapers");
        let user_root = temp.join("user/wallpapers");
        let item = built_in_root.join("starter");
        fs::create_dir_all(&item).unwrap();
        fs::create_dir_all(&user_root).unwrap();
        fs::write(item.join("wallpaper.mp4"), b"video fixture").unwrap();
        fs::write(item.join("thumbnail.webp"), b"image fixture").unwrap();
        let manifest = WallpaperManifest {
            schema_version: 1,
            id: "gnomeengine-starter-001".to_owned(),
            title: "Starter Wallpaper".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-03T00:00:00Z".to_owned(),
            author: Some("GnomeEngine".to_owned()),
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "wallpaper.mp4".to_owned(),
            },
            thumbnail: Some("thumbnail.webp".to_owned()),
            media: MediaMetadata::default(),
        };
        fs::write(
            item.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let mut library = Library::load_with_builtins(user_root, vec![built_in_root]).unwrap();

        assert_eq!(library.wallpapers().len(), 1);
        let wallpaper = &library.wallpapers()[0];
        assert_eq!(wallpaper.origin, WallpaperOrigin::BuiltIn);
        assert_eq!(wallpaper.content_path, item.join("wallpaper.mp4"));
        assert_eq!(wallpaper.thumbnail_path, item.join("thumbnail.webp"));
        assert!(library.remove("gnomeengine-starter-001").is_err());
        assert!(item.join("wallpaper.mp4").is_file());
        let reloaded = library.reload().unwrap();
        assert_eq!(reloaded.wallpapers().len(), 1);
        assert_eq!(reloaded.wallpapers()[0].origin, WallpaperOrigin::BuiltIn);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn built_in_assets_win_duplicate_user_ids() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-builtin-duplicate-{}",
            new_wallpaper_id()
        ));
        let built_in_root = temp.join("system");
        let user_root = temp.join("user");
        let built_in = built_in_root.join("starter");
        let id = "gnomeengine-starter-001";
        fs::create_dir_all(&built_in).unwrap();
        fs::create_dir_all(user_root.join(id)).unwrap();
        fs::write(built_in.join("wallpaper.mp4"), b"official").unwrap();
        fs::write(built_in.join("thumbnail.webp"), b"thumb").unwrap();
        fs::write(user_root.join(id).join("content"), b"user spoof").unwrap();
        let built_in_manifest = WallpaperManifest {
            schema_version: 1,
            id: id.to_owned(),
            title: "Official".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-03T00:00:00Z".to_owned(),
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "wallpaper.mp4".to_owned(),
            },
            thumbnail: Some("thumbnail.webp".to_owned()),
            media: MediaMetadata::default(),
        };
        fs::write(
            built_in.join("manifest.json"),
            serde_json::to_vec(&built_in_manifest).unwrap(),
        )
        .unwrap();
        let user_manifest = WallpaperManifest {
            content: ContentManifest {
                entry: "content".to_owned(),
            },
            thumbnail: None,
            ..built_in_manifest.clone()
        };
        fs::write(
            user_root.join(id).join("manifest.json"),
            serde_json::to_vec(&user_manifest).unwrap(),
        )
        .unwrap();

        let library = Library::load_with_builtins(user_root, vec![built_in_root]).unwrap();

        assert_eq!(library.wallpapers().len(), 1);
        assert_eq!(library.wallpapers()[0].origin, WallpaperOrigin::BuiltIn);
        assert_eq!(library.wallpapers()[0].manifest.title, "Official");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn ignores_incomplete_development_asset_without_affecting_user_library() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-builtin-incomplete-{}",
            new_wallpaper_id()
        ));
        let built_in_root = temp.join("assets");
        let user_root = temp.join("user");
        let starter = built_in_root.join("starter");
        fs::create_dir_all(&starter).unwrap();
        fs::create_dir_all(&user_root).unwrap();
        let manifest = WallpaperManifest {
            schema_version: 1,
            id: "gnomeengine-starter-001".to_owned(),
            title: "Starter Wallpaper".to_owned(),
            wallpaper_type: "video".to_owned(),
            created_at: "2026-10-03T00:00:00Z".to_owned(),
            author: None,
            copyright: None,
            license: None,
            content: ContentManifest {
                entry: "wallpaper.mp4".to_owned(),
            },
            thumbnail: Some("thumbnail.webp".to_owned()),
            media: MediaMetadata::default(),
        };
        fs::write(
            starter.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let library = Library::load_with_builtins(user_root, vec![built_in_root]).unwrap();
        assert!(library.wallpapers().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn invalid_built_in_manifest_is_skipped_without_failing_library_load() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-builtin-invalid-{}",
            new_wallpaper_id()
        ));
        let built_in_root = temp.join("assets");
        let user_root = temp.join("user");
        fs::create_dir_all(built_in_root.join("broken")).unwrap();
        fs::create_dir_all(&user_root).unwrap();
        fs::write(built_in_root.join("broken/manifest.json"), b"not json").unwrap();

        let library = Library::load_with_builtins(user_root, vec![built_in_root]).unwrap();

        assert!(library.wallpapers().is_empty());
        assert_eq!(library.invalid_items(), 1);
        fs::remove_dir_all(temp).unwrap();
    }
}
