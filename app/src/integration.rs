use gio::prelude::SettingsExtManual;

const EXTENSION_UUID: &str = "gnomeengine@mvk999.github.io";

/// Describes why desktop rendering is unavailable. Extension files alone do
/// not prove that the active Shell loaded or enabled the extension.
pub fn guidance(ready: bool) -> &'static str {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let installed = extension_files_installed();
    let enabled = installed
        && enabled_extension_uuids()
            .map(|extensions| extensions.iter().any(|uuid| uuid == EXTENSION_UUID))
            .unwrap_or(false);
    guidance_for(&session, installed, enabled, ready)
}

pub fn extension_can_be_enabled() -> bool {
    extension_files_installed()
        && enabled_extension_uuids()
            .map(|enabled| !enabled.iter().any(|uuid| uuid == EXTENSION_UUID))
            .unwrap_or(false)
}

/// Enable the packaged extension only after an explicit user action in the
/// application. Package installation itself never mutates this user setting.
pub fn enable_for_current_user() -> Result<(), String> {
    let settings = shell_extension_settings()?;
    let enabled = settings
        .strv("enabled-extensions")
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let updated = with_extension_uuid(enabled.clone(), EXTENSION_UUID);
    if updated != enabled {
        settings
            .set_strv("enabled-extensions", updated)
            .map_err(|_| "O GNOME não permitiu habilitar a extensão.".to_owned())?;
    }
    Ok(())
}

fn enabled_extension_uuids() -> Result<Vec<String>, String> {
    Ok(shell_extension_settings()?
        .strv("enabled-extensions")
        .iter()
        .map(ToString::to_string)
        .collect())
}

fn shell_extension_settings() -> Result<gio::Settings, String> {
    let source = gio::SettingsSchemaSource::default()
        .ok_or_else(|| "As preferências do GNOME não estão disponíveis nesta sessão.".to_owned())?;
    let schema = source
        .lookup("org.gnome.shell", true)
        .ok_or_else(|| "Não foi possível localizar as preferências do GNOME Shell.".to_owned())?;
    if !schema.has_key("enabled-extensions") {
        return Err("Esta versão do GNOME não expõe a lista de extensões habilitadas.".to_owned());
    }

    Ok(gio::Settings::new_full(
        &schema,
        None::<&gio::SettingsBackend>,
        None,
    ))
}

fn with_extension_uuid(mut enabled: Vec<String>, uuid: &str) -> Vec<String> {
    if !enabled.iter().any(|existing| existing == uuid) {
        enabled.push(uuid.to_owned());
    }
    enabled
}

fn guidance_for(
    session: &str,
    extension_installed: bool,
    extension_enabled: bool,
    ready: bool,
) -> &'static str {
    if ready {
        return "Ativa — pronta para classificar a superfície de vídeo";
    }

    match session {
        "x11" if !extension_installed => {
            "Sessão X11 detectada. A extensão GnomeEngine não está instalada."
        }
        "wayland" if !extension_installed => "Extensão GnomeEngine não está instalada.",
        _ if extension_installed && !extension_enabled => {
            "Extensão instalada, mas desativada. Use Ativar integração para habilitá-la."
        }
        "x11" => "Sessão X11 detectada. O GNOME ainda não confirmou a integração.",
        "wayland" => "Extensão encontrada, mas ainda não confirmada pelo GNOME.",
        _ => "Não foi possível confirmar uma sessão GNOME compatível",
    }
}

fn extension_files_installed() -> bool {
    let extension_path = std::path::Path::new("gnome-shell")
        .join("extensions")
        .join(EXTENSION_UUID)
        .join("metadata.json");
    let user_data_dir = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(glib::user_data_dir);

    [
        std::path::PathBuf::from("/usr/share").join(&extension_path),
        std::path::PathBuf::from("/usr/local/share").join(&extension_path),
        user_data_dir.join(&extension_path),
    ]
    .iter()
    .any(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{guidance_for, with_extension_uuid};

    #[test]
    fn enabling_extension_preserves_other_extensions_and_is_idempotent() {
        let existing = vec!["other@example.org".to_owned()];
        let enabled = with_extension_uuid(existing, "gnomeengine@mvk999.github.io");
        assert_eq!(enabled.len(), 2);
        assert_eq!(
            with_extension_uuid(enabled.clone(), "gnomeengine@mvk999.github.io"),
            enabled
        );
    }

    #[test]
    fn identifies_x11_without_claiming_it_is_unsupported() {
        let guidance = guidance_for("x11", false, false, false);
        assert!(guidance.contains("X11"));
        assert!(!guidance.contains("Indisponível"));
    }

    #[test]
    fn distinguishes_an_extracted_package_from_installed_extension_files() {
        assert!(guidance_for("wayland", false, false, false).contains("não está instalada"));
    }

    #[test]
    fn does_not_claim_extension_is_enabled_just_because_files_exist() {
        assert!(guidance_for("wayland", true, true, false).contains("não confirmada"));
    }

    #[test]
    fn explains_that_an_installed_extension_is_disabled() {
        assert!(guidance_for("wayland", true, false, false).contains("desativada"));
    }

    #[test]
    fn reports_ready_only_after_shell_handshake() {
        assert!(guidance_for("x11", false, false, true).starts_with("Ativa"));
    }
}
