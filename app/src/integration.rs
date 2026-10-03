const EXTENSION_UUID: &str = "gnomeengine@mvk999.github.io";

/// Describes why desktop rendering is unavailable. Extension files alone do
/// not prove that the active Shell loaded or enabled the extension.
pub fn guidance(ready: bool) -> &'static str {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    guidance_for(&session, extension_files_installed(), ready)
}

fn guidance_for(session: &str, extension_installed: bool, ready: bool) -> &'static str {
    if ready {
        return "Ativa — pronta para classificar a superfície de vídeo";
    }

    match session {
        "x11" => "Indisponível nesta sessão X11 — o suporte atual exige GNOME 50+ no Wayland",
        "wayland" if !extension_installed => {
            "Extensão não instalada. Extrair o .deb não instala a integração no GNOME"
        }
        "wayland" => {
            "Extensão encontrada, mas não confirmada pelo GNOME. Ative-a e use GNOME 50+ no Wayland"
        }
        _ => "Não foi possível confirmar uma sessão GNOME Wayland compatível",
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
    use super::guidance_for;

    #[test]
    fn explains_that_the_current_x11_session_is_unsupported() {
        assert!(guidance_for("x11", false, false).contains("X11"));
    }

    #[test]
    fn distinguishes_an_extracted_package_from_installed_extension_files() {
        assert!(guidance_for("wayland", false, false).contains("não instalada"));
    }

    #[test]
    fn does_not_claim_extension_is_enabled_just_because_files_exist() {
        assert!(guidance_for("wayland", true, false).contains("não confirmada"));
    }

    #[test]
    fn reports_ready_only_after_shell_handshake() {
        assert!(guidance_for("x11", false, true).starts_with("Ativa"));
    }
}
