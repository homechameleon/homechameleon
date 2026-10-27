//! Theme files stay under `<config>/themes` and `<config>/www/homechameleon`.
//! `configuration.yaml` is read to warn about the themes include, and never written.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use std::os::unix::fs::OpenOptionsExt;

pub const THEMES_INCLUDE: &str = "frontend:\n  themes: !include_dir_merge_named themes";
const THEMES_INCLUDE_MARKER: &str = "!include_dir_merge_named themes";
const CONFIG_FILE: &str = "configuration.yaml";

pub fn valid_slug(slug: &str) -> bool {
    let bytes = slug.as_bytes();
    !bytes.is_empty()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
        })
}

pub fn valid_background_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("bg-") else {
        return false;
    };
    let Some((hex, ext)) = rest.split_once('.') else {
        return false;
    };
    hex.len() == 8
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        && (ext == "webp" || ext == "jpg")
}

pub fn theme_yaml_path(config_dir: &Path, slug: &str) -> Result<PathBuf, String> {
    if !valid_slug(slug) {
        return Err("theme name is not a safe file name".into());
    }
    let path = config_dir.join("themes").join(format!("{slug}.yaml"));
    match classify(config_dir, &path)? {
        Kind::Theme => Ok(path),
        _ => Err("theme name is not a safe file name".into()),
    }
}

pub fn theme_dir_path(config_dir: &Path, slug: &str) -> Result<PathBuf, String> {
    if !valid_slug(slug) {
        return Err("theme name is not a safe file name".into());
    }
    let path = config_dir.join("www").join("homechameleon").join(slug);
    match classify(config_dir, &path)? {
        Kind::ThemeDir => Ok(path),
        _ => Err("theme name is not a safe file name".into()),
    }
}

pub fn background_path(config_dir: &Path, slug: &str, relative: &str) -> Result<PathBuf, String> {
    if !valid_slug(slug) {
        return Err("theme name is not a safe file name".into());
    }
    let Some((folder, name)) = relative.split_once('/') else {
        return Err("background file name is not allowed".into());
    };
    if folder != slug || name.contains('/') || !valid_background_name(name) {
        return Err("background file name is not allowed".into());
    }
    let path = theme_dir_path(config_dir, slug)?.join(name);
    match classify(config_dir, &path)? {
        Kind::Wallpaper => Ok(path),
        _ => Err("background file name is not allowed".into()),
    }
}

pub fn atomic_write(config_dir: &Path, path: &Path, data: &[u8]) -> Result<(), String> {
    match classify(config_dir, path)? {
        Kind::Theme | Kind::Wallpaper => {}
        Kind::ThemeDir => return Err("theme name is not a safe file name".into()),
    }
    refuse_symlinks(config_dir, path)?;
    let parent = path
        .parent()
        .ok_or_else(|| "theme name is not a safe file name".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    refuse_symlinks(config_dir, path)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_name = format!(".homechameleon-{}-{nanos}.tmp", std::process::id());
    let tmp = parent.join(tmp_name);
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .open(&tmp)
            .map_err(|err| err.to_string())?;
        file.write_all(data).map_err(|err| err.to_string())?;
        file.sync_all().map_err(|err| err.to_string())?;
        fs::rename(&tmp, path).map_err(|err| err.to_string())?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result
}

pub fn delete_theme(config_dir: &Path, slug: &str) -> Result<bool, String> {
    let yaml = theme_yaml_path(config_dir, slug)?;
    let folder = theme_dir_path(config_dir, slug)?;
    refuse_symlinks(config_dir, &yaml)?;
    refuse_symlinks(config_dir, &folder)?;
    let mut found = false;
    if yaml.is_file() {
        fs::remove_file(&yaml).map_err(|err| err.to_string())?;
        found = true;
    }
    if folder.is_dir() {
        fs::remove_dir_all(&folder).map_err(|err| err.to_string())?;
        found = true;
    }
    Ok(found)
}

pub fn www_directory_exists(config_dir: &Path) -> bool {
    config_dir.join("www").exists()
}

pub fn themes_include_warning(config_dir: &Path) -> Option<String> {
    let path = config_dir.join(CONFIG_FILE);
    let text = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes[..bytes.len().min(2_000_000)]).into_owned(),
        Err(_) => String::new(),
    };
    if text.contains(THEMES_INCLUDE_MARKER) {
        return None;
    }
    Some(format!(
        "Add these lines to configuration.yaml, then restart Home Assistant:\n\n{THEMES_INCLUDE}\n\nHomeChameleon does not edit that file."
    ))
}

enum Kind {
    Theme,
    Wallpaper,
    ThemeDir,
}

fn classify(config_dir: &Path, path: &Path) -> Result<Kind, String> {
    let config = normalize(config_dir);
    let full = normalize(path);
    if full == config.join(CONFIG_FILE) {
        return Err("HomeChameleon does not edit configuration.yaml".into());
    }
    let themes = config.join("themes");
    let wallpapers = config.join("www").join("homechameleon");
    let Some(name) = full.file_name().and_then(|part| part.to_str()) else {
        return Err("theme name is not a safe file name".into());
    };
    if full.parent() == Some(themes.as_path()) {
        if let Some(slug) = name.strip_suffix(".yaml") {
            if valid_slug(slug) {
                return Ok(Kind::Theme);
            }
        }
    }
    if let Some(parent) = full.parent() {
        if parent.parent() == Some(wallpapers.as_path()) {
            let slug = parent
                .file_name()
                .and_then(|part| part.to_str())
                .unwrap_or("");
            if valid_slug(slug) && valid_background_name(name) {
                return Ok(Kind::Wallpaper);
            }
        }
    }
    if full.parent() == Some(wallpapers.as_path()) && valid_slug(name) {
        return Ok(Kind::ThemeDir);
    }
    Err("theme name is not a safe file name".into())
}

fn refuse_symlinks(config_dir: &Path, path: &Path) -> Result<(), String> {
    let relative = normalize(path)
        .strip_prefix(normalize(config_dir))
        .map_err(|_| "theme name is not a safe file name".to_string())?
        .to_path_buf();
    let mut current = config_dir.to_path_buf();
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err("theme name is not a safe file name".into());
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("refusing to write outside the theme folders".into());
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.to_string()),
        }
    }
    Ok(())
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn temp_config() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "homechameleon-store-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(CONFIG_FILE), "default_config:\n").unwrap();
        path
    }

    #[test]
    fn writes_are_confined_to_theme_folders() {
        let root = temp_config();
        let original = fs::read(root.join(CONFIG_FILE)).unwrap();
        let yaml = theme_yaml_path(&root, "desk").unwrap();
        atomic_write(&root, &yaml, b"Desk:\n  modes: {}\n").unwrap();
        let wallpaper = background_path(&root, "desk", "desk/bg-0123abcd.webp").unwrap();
        atomic_write(&root, &wallpaper, b"webp").unwrap();
        assert_eq!(fs::read(yaml).unwrap(), b"Desk:\n  modes: {}\n");
        assert_eq!(fs::read(wallpaper).unwrap(), b"webp");
        assert_eq!(fs::read(root.join(CONFIG_FILE)).unwrap(), original);
        assert!(theme_yaml_path(&root, "../configuration").is_err());
        assert!(theme_yaml_path(&root, "a/b").is_err());
        assert!(background_path(&root, "desk", "other/bg-0123abcd.webp").is_err());
        assert!(background_path(&root, "desk", "desk/bg-0123ABCD.webp").is_err());
        assert!(background_path(&root, "desk", "desk/../../configuration.yaml").is_err());
        assert!(atomic_write(&root, &root.join(CONFIG_FILE), b"nope").is_err());
        assert_eq!(fs::read(root.join(CONFIG_FILE)).unwrap(), original);
        assert!(delete_theme(&root, "desk").unwrap());
        assert!(!root.join("themes").join("desk.yaml").exists());
        assert!(!root.join("www").join("homechameleon").join("desk").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn symlinks_are_not_write_targets() {
        let root = temp_config();
        let outside = temp_config();
        fs::remove_file(outside.join(CONFIG_FILE)).unwrap();
        fs::create_dir_all(root.join("themes")).unwrap();
        fs::remove_dir(root.join("themes")).unwrap();
        symlink(&outside, root.join("themes")).unwrap();
        let yaml = theme_yaml_path(&root, "desk").unwrap();
        assert!(atomic_write(&root, &yaml, b"nope").is_err());
        assert!(fs::read_dir(&outside).unwrap().next().is_none());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn missing_themes_include_is_a_warning() {
        let root = temp_config();
        let warning = themes_include_warning(&root).unwrap();
        assert!(warning.contains(THEMES_INCLUDE));
        assert!(warning.contains("does not edit that file"));
        fs::write(root.join(CONFIG_FILE), format!("{THEMES_INCLUDE}\n")).unwrap();
        assert!(themes_include_warning(&root).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
