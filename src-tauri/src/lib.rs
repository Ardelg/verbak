/*
 * MACOS PERMISSIONS GUIDE:
 * Para que `enigo` pueda simular pulsaciones de teclas en macOS (Cmd+C, Cmd+V),
 * la aplicación compilada (.app) o la terminal ejecutando `cargo run` necesita
 * permisos de "Accesibilidad".
 *
 * Para habilitarlo:
 * 1. Abre "System Settings" (Configuración del Sistema) -> "Privacy & Security" (Privacidad y Seguridad).
 * 2. Selecciona "Accessibility" (Accesibilidad).
 * 3. Haz clic en el botón "+" y añade tu aplicación compilada (Verbak.app)
 *    o tu terminal (ej. iTerm, Terminal, Alacritty, VSCode).
 * 4. Activa el interruptor junto a la aplicación.
 */

mod ollama;
mod providers;
mod sanitize;

use copypasta::{ClipboardContext, ClipboardProvider};
use enigo::{Enigo, Keyboard, Settings, Key, Direction};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use serde_json::json;
use std::sync::Mutex;
use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::image::Image;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use providers::{ApiProfile, Provider, TranslateRequest};
use sanitize::SanitizeSettings;

lazy_static! {
    static ref ORIGINAL_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);
}

fn default_opacity() -> u8 { 95 }
fn default_true() -> bool { true }
fn default_target_lang() -> String { "ES".into() }
fn default_lang_a() -> String { "ES".into() }
fn default_lang_b() -> String { "EN-US".into() }
fn default_context() -> String {
    "Treat 'the agent' as a neutral AI system. Do not use gendered pronouns (he/she) for it. Use 'the agent', 'it', or passive phrasing as appropriate. The goal is literal, functional translation, level B2/C1. Do not improve the text.".into()
}
fn default_slack_context() -> String {
    "Mensaje de chat informal para compañeros de trabajo en Slack. Usa un tono amigable, natural y muy relajado. NO uses lenguaje corporativo formal.".into()
}

fn default_shortcut_a() -> String { "Ctrl+Alt+D".into() }
fn default_shortcut_b() -> String { "Ctrl+Alt+F".into() }
fn default_shortcut_slack() -> String { "Ctrl+Alt+S".into() }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    /// Clave heredada de la versión anterior: solo se usa para migrar a `profiles`.
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    profiles: Vec<ApiProfile>,
    #[serde(default)]
    active_profile: String,
    /// Si el perfil activo falla, se intenta con el resto de perfiles válidos.
    #[serde(default = "default_true")]
    auto_fallback: bool,
    /// Heredado: solo se usa para migrar a `lang_a`.
    #[serde(default = "default_target_lang")]
    target_lang: String,
    /// Par de idiomas entre los que alternan los atajos según lo detectado.
    /// Vacíos = sin configurar; `migrate` los rellena (respetando `target_lang`).
    #[serde(default)]
    lang_a: String,
    #[serde(default)]
    lang_b: String,
    #[serde(default = "default_context")]
    context: String,
    #[serde(default = "default_slack_context")]
    slack_context: String,
    #[serde(default = "default_opacity")]
    opacity: u8,
    #[serde(default)]
    sanitize: SanitizeSettings,
    #[serde(default = "default_shortcut_a")]
    shortcut_a: String,
    #[serde(default)]
    profile_a: Option<String>,
    #[serde(default = "default_shortcut_b")]
    shortcut_b: String,
    #[serde(default)]
    profile_b: Option<String>,
    #[serde(default = "default_shortcut_slack")]
    shortcut_slack: String,
    #[serde(default)]
    profile_slack: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: "".into(),
            profiles: Vec::new(),
            active_profile: "".into(),
            auto_fallback: true,
            target_lang: default_target_lang(),
            lang_a: default_lang_a(),
            lang_b: default_lang_b(),
            context: default_context(),
            slack_context: default_slack_context(),
            opacity: default_opacity(),
            sanitize: SanitizeSettings::default(),
            shortcut_a: default_shortcut_a(),
            profile_a: None,
            shortcut_b: default_shortcut_b(),
            profile_b: None,
            shortcut_slack: default_shortcut_slack(),
            profile_slack: None,
        }
    }
}

/// Crea el perfil DeepL a partir de la configuración antigua y asegura que
/// siempre haya un perfil activo válido.
fn migrate(mut settings: AppSettings) -> AppSettings {
    if settings.profiles.is_empty() {
        settings.profiles.push(ApiProfile {
            id: "deepl".into(),
            name: "DeepL".into(),
            provider: Provider::Deepl,
            api_key: settings.api_key.clone(),
            model: "".into(),
            base_url: "".into(),
            enabled: true,
        });
    }

    let active_exists = settings
        .profiles
        .iter()
        .any(|p| p.id == settings.active_profile);
    if !active_exists {
        settings.active_profile = settings
            .profiles
            .iter()
            .find(|p| p.is_usable())
            .or_else(|| settings.profiles.first())
            .map(|p| p.id.clone())
            .unwrap_or_default();
    }

    // Par de idiomas: si viene sin configurar, se hereda de `target_lang`.
    if settings.lang_a.trim().is_empty() {
        settings.lang_a = if settings.target_lang.trim().is_empty() {
            "ES".into()
        } else {
            settings.target_lang.clone()
        };
    }
    if settings.lang_b.trim().is_empty() {
        settings.lang_b = if providers::lang_family(&settings.lang_a) == "EN" {
            "ES".into()
        } else {
            "EN-US".into()
        };
    }
    if providers::lang_family(&settings.lang_a) == providers::lang_family(&settings.lang_b) {
        settings.lang_b = if providers::lang_family(&settings.lang_a) == "EN" {
            "ES".into()
        } else {
            "EN-US".into()
        };
    }
    settings
}

fn get_settings_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("settings.json")
}

fn load_settings(app: &AppHandle) -> AppSettings {
    let path = get_settings_path(app);
    let settings = fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
        .unwrap_or_default();
    migrate(settings)
}

fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path(app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> AppSettings {
    load_settings(&app)
}

fn register_shortcuts(app: &AppHandle, settings: &AppSettings) {
    let _ = app.global_shortcut().unregister_all();

    let handle_a = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut(settings.shortcut_a.as_str(), move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handle_flow_a(handle_a.clone());
        }
    }) {
        println!("Error al registrar {}: {:?}", settings.shortcut_a, e);
    }

    let handle_b = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut(settings.shortcut_b.as_str(), move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handle_flow_b(handle_b.clone());
        }
    }) {
        println!("Error al registrar {}: {:?}", settings.shortcut_b, e);
    }

    let handle_s = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut(settings.shortcut_slack.as_str(), move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handle_flow_slack(handle_s.clone());
        }
    }) {
        println!("Error al registrar {}: {:?}", settings.shortcut_slack, e);
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let migrated = migrate(settings);
    write_settings(&app, &migrated)?;
    register_shortcuts(&app, &migrated);
    Ok(())
}

/// Cambio rápido de API desde la ventana de revisión, sin tocar el resto.
#[tauri::command]
fn set_active_profile(app: AppHandle, id: String) -> Result<AppSettings, String> {
    let mut settings = load_settings(&app);
    if !settings.profiles.iter().any(|p| p.id == id) {
        return Err("Perfil no encontrado".into());
    }
    settings.active_profile = id;
    write_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn sanitize_preview(app: AppHandle, text: String) -> String {
    let settings = load_settings(&app);
    let mut rules = settings.sanitize.clone();
    // El botón "Limpiar" es una acción explícita: se aplica aunque el
    // automático esté apagado.
    rules.enabled = true;
    sanitize::sanitize(&text, &rules)
}

/// Prueba de conexión de un perfil, sin necesidad de guardarlo antes.
#[tauri::command]
async fn test_profile(profile: ApiProfile) -> Result<String, String> {
    let req = TranslateRequest {
        text: "Hi, this is a connection test.",
        target_lang: "ES",
        context: "",
        informal: false,
        plain_punctuation: true,
    };
    providers::translate(&profile, &req)
        .await
        .map_err(|e| e.to_string())
}

fn get_clipboard() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut ctx = ClipboardContext::new().unwrap();
    Ok(ctx.get_contents()?)
}

fn set_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(text.to_owned())?;
    Ok(())
}

fn simulate_copy() {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = enigo.key(Key::Meta, Direction::Press);
        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
        let _ = enigo.key(Key::Meta, Direction::Release);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = enigo.key(Key::Alt, Direction::Release);
        let _ = enigo.key(Key::Control, Direction::Release);
        let _ = enigo.key(Key::Shift, Direction::Release);
        std::thread::sleep(Duration::from_millis(20));

        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
    }
}

fn simulate_paste() {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = enigo.key(Key::Meta, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Meta, Direction::Release);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = enigo.key(Key::Alt, Direction::Release);
        let _ = enigo.key(Key::Control, Direction::Release);
        let _ = enigo.key(Key::Shift, Direction::Release);
        std::thread::sleep(Duration::from_millis(20));

        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
    }
}

#[derive(Debug, Serialize, Clone)]
struct TranslationResult {
    text: String,
    profile_name: String,
    provider: String,
    /// Avisos de los perfiles que fallaron antes de que uno funcionara.
    warnings: Vec<String>,
}

/// Idioma destino final: respeta el forzado y, si no, alterna entre el par
/// `lang_a` <-> `lang_b` según el idioma detectado en el texto.
/// Si el texto no está en ninguno de los dos (o no se puede detectar),
/// se traduce a `lang_a` (el idioma "propio" del usuario).
fn resolve_target(settings: &AppSettings, text: &str, force: Option<String>) -> String {
    if let Some(f) = force {
        return f;
    }
    let a = settings.lang_a.clone();
    let b = settings.lang_b.clone();
    let fam_a = providers::lang_family(&a);
    let fam_b = providers::lang_family(&b);

    // Restringir la detección a los dos idiomas del par: `whatlang` distingue
    // mucho mejor "¿es A o B?" que "¿qué idioma de los 69?" (clave para ES/PT).
    let allow: Vec<_> = [
        providers::family_to_whatlang(&fam_a),
        providers::family_to_whatlang(&fam_b),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut detected = if allow.len() == 2 {
        whatlang::Detector::with_allowlist(allow)
            .detect_lang(text)
            .map(providers::whatlang_family)
    } else {
        whatlang::detect(text).map(|info| providers::whatlang_family(info.lang()))
    };

    // El par ES/PT es el que más se le escapa a whatlang: desempate por léxico.
    let pair: [&str; 2] = [fam_a.as_str(), fam_b.as_str()];
    if pair.contains(&"ES") && pair.contains(&"PT") {
        if let Some(hint) = providers::es_pt_hint(text) {
            detected = Some(hint.to_string());
        }
    }

    match detected {
        Some(d) if d == fam_a => b,
        Some(d) if d == fam_b => a,
        _ => a,
    }
}

/// Orden de intento: perfil pedido (o el activo) y, si `auto_fallback` está
/// encendido, el resto de perfiles utilizables.
fn build_chain(settings: &AppSettings, preferred: Option<&str>) -> Vec<ApiProfile> {
    let wanted = preferred.unwrap_or(settings.active_profile.as_str());
    let mut chain: Vec<ApiProfile> = Vec::new();

    if let Some(p) = settings.profiles.iter().find(|p| p.id == wanted) {
        chain.push(p.clone());
    }
    if settings.auto_fallback {
        for p in settings.profiles.iter() {
            if p.id != wanted && p.is_usable() {
                chain.push(p.clone());
            }
        }
    }
    if chain.is_empty() {
        chain = settings.profiles.iter().filter(|p| p.is_usable()).cloned().collect();
    }
    chain
}

async fn translate_text(
    app: &AppHandle,
    text: &str,
    context_override: Option<String>,
    informal: bool,
    force_target_lang: Option<String>,
    preferred_profile: Option<String>,
) -> Result<TranslationResult, String> {
    let settings = load_settings(app);
    let target = resolve_target(&settings, text, force_target_lang);
    let context = context_override.unwrap_or_else(|| settings.context.clone());
    let chain = build_chain(&settings, preferred_profile.as_deref());

    if chain.is_empty() {
        return Err("No hay ninguna API configurada. Abre Ajustes y añade una.".into());
    }

    let mut warnings: Vec<String> = Vec::new();
    for profile in chain {
        if !profile.is_usable() {
            warnings.push(format!("{}: sin API Key", profile.name));
            continue;
        }
        let req = TranslateRequest {
            text,
            target_lang: &target,
            context: &context,
            informal,
            plain_punctuation: settings.sanitize.enabled,
        };
        match providers::translate(&profile, &req).await {
            Ok(raw) => {
                return Ok(TranslationResult {
                    text: sanitize::sanitize(&raw, &settings.sanitize),
                    profile_name: profile.name.clone(),
                    provider: profile.provider.label().to_string(),
                    warnings,
                });
            }
            Err(e) => {
                println!("Fallo en el perfil {}: {}", profile.name, e);
                warnings.push(format!("{}: {}", profile.name, e));
            }
        }
    }

    Err(warnings.join(" | "))
}

/// La pantalla de ajustes necesita más alto que la de revisión.
#[tauri::command]
fn set_view_size(app: AppHandle, settings_view: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let size = if settings_view {
            tauri::LogicalSize::new(660.0, 620.0)
        } else {
            tauri::LogicalSize::new(600.0, 450.0)
        };
        let _ = window.set_size(size);
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
async fn replace_text(app: AppHandle, new_text: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    set_clipboard(&new_text).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(100));
    simulate_paste();

    // Opcional: Restaurar portapapeles original
    std::thread::sleep(Duration::from_millis(500));
    if let Ok(guard) = ORIGINAL_CLIPBOARD.lock() {
        if let Some(orig) = &*guard {
            let _ = set_clipboard(orig);
        }
    }

    Ok(())
}

/// Retraducción desde la ventana de revisión: idioma y/o API concretos.
#[tauri::command]
async fn force_translate(
    app: AppHandle,
    text: String,
    // Ausente = detección automática ES <-> EN.
    target_lang: Option<String>,
    profile_id: Option<String>,
    informal: Option<bool>,
) -> Result<TranslationResult, String> {
    let target = target_lang.filter(|t| !t.is_empty());
    translate_text(
        &app,
        &text,
        None,
        informal.unwrap_or(false),
        target,
        profile_id,
    )
    .await
}

fn notify_error(app: &AppHandle, message: String) {
    println!("Error de traducción: {}", message);
    let _ = app.emit("translation-error", message);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Copia la selección actual y devuelve el texto, guardando el portapapeles previo.
fn capture_selection() -> Option<String> {
    let orig_clip = get_clipboard().unwrap_or_default();
    {
        let mut guard = ORIGINAL_CLIPBOARD.lock().unwrap();
        *guard = Some(orig_clip.clone());
    }

    simulate_copy();
    std::thread::sleep(Duration::from_millis(150)); // Esperar al SO

    let copied_text = get_clipboard().unwrap_or_default();
    if copied_text.trim().is_empty() {
        println!("Error: portapapeles vacío");
        return None;
    }
    Some(copied_text)
}

fn restore_clipboard() {
    std::thread::sleep(Duration::from_millis(500));
    let orig = ORIGINAL_CLIPBOARD.lock().unwrap().clone();
    if let Some(orig) = orig {
        let _ = set_clipboard(&orig);
    }
}

fn handle_flow_a(app: AppHandle) {
    println!("Flujo A iniciado");
    tauri::async_runtime::spawn(async move {
        let Some(copied_text) = capture_selection() else { return };
        let settings = load_settings(&app);
        
        println!("Traduciendo texto A...");
        match translate_text(&app, &copied_text, None, false, None, settings.profile_a).await {
            Ok(result) => {
                let _ = set_clipboard(&result.text);
                std::thread::sleep(Duration::from_millis(50));
                simulate_paste();
                restore_clipboard();
            }
            Err(e) => notify_error(&app, e),
        }
    });
}

fn handle_flow_b(app: AppHandle) {
    println!("Flujo B iniciado");
    tauri::async_runtime::spawn(async move {
        let Some(copied_text) = capture_selection() else { return };
        let settings = load_settings(&app);

        println!("Traduciendo texto B...");
        match translate_text(&app, &copied_text, None, false, None, settings.profile_b).await {
            Ok(result) => {
                let _ = app.emit("translation-ready", json!({
                    "original": copied_text,
                    "translated": result.text,
                    "profileName": result.profile_name,
                    "provider": result.provider,
                    "warnings": result.warnings,
                }));
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            Err(e) => notify_error(&app, e),
        }
    });
}

fn handle_flow_slack(app: AppHandle) {
    println!("Flujo Slack iniciado");
    tauri::async_runtime::spawn(async move {
        let Some(copied_text) = capture_selection() else { return };
        let settings = load_settings(&app);
        let slack_context = settings.slack_context.clone();

        println!("Traduciendo para Slack...");
        
        match translate_text(
            &app,
            &copied_text,
            Some(slack_context),
            true,
            None,
            settings.profile_slack,
        )
        .await
        {
            Ok(result) => {
                let _ = set_clipboard(&result.text);
                std::thread::sleep(Duration::from_millis(100));
                simulate_paste();
                restore_clipboard();
            }
            Err(e) => notify_error(&app, e),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// settings.json de la versión anterior: solo tenía estos campos.
    const LEGACY: &str = r#"{
        "api_key": "abc-123:fx",
        "target_lang": "ES",
        "context": "mi contexto",
        "slack_context": "mi slack",
        "opacity": 94
    }"#;

    #[test]
    fn legacy_settings_keep_key_and_become_a_deepl_profile() {
        let parsed: AppSettings = serde_json::from_str(LEGACY).expect("debe parsear");
        let migrated = migrate(parsed);

        assert_eq!(migrated.profiles.len(), 1);
        assert_eq!(migrated.profiles[0].provider, Provider::Deepl);
        assert_eq!(migrated.profiles[0].api_key, "abc-123:fx");
        assert_eq!(migrated.active_profile, migrated.profiles[0].id);
        assert_eq!(migrated.context, "mi contexto");
        assert_eq!(migrated.opacity, 94);
        assert!(migrated.sanitize.enabled);
        assert!(migrated.auto_fallback);
        // El par de idiomas se hereda de `target_lang`.
        assert_eq!(migrated.lang_a, "ES");
        assert_eq!(migrated.lang_b, "EN-US");
    }

    #[test]
    fn legacy_target_lang_seeds_the_pair() {
        let json = r#"{ "api_key": "", "target_lang": "FR" }"#;
        let m = migrate(serde_json::from_str::<AppSettings>(json).unwrap());
        assert_eq!(m.lang_a, "FR");
        assert_eq!(m.lang_b, "EN-US");
    }

    #[test]
    fn chain_falls_back_to_the_other_profiles() {
        let mut settings = migrate(serde_json::from_str::<AppSettings>(LEGACY).unwrap());
        settings.profiles.push(ApiProfile {
            id: "gem".into(),
            name: "Gemini".into(),
            provider: Provider::Gemini,
            api_key: "k".into(),
            model: "".into(),
            base_url: "".into(),
            enabled: true,
        });
        // Uno deshabilitado no debe entrar en la cadena.
        settings.profiles.push(ApiProfile {
            id: "off".into(),
            name: "Apagado".into(),
            provider: Provider::Openai,
            api_key: "k".into(),
            model: "".into(),
            base_url: "".into(),
            enabled: false,
        });

        let chain = build_chain(&settings, None);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, settings.active_profile);
        assert_eq!(chain[1].id, "gem");

        settings.auto_fallback = false;
        assert_eq!(build_chain(&settings, None).len(), 1);

        // Un perfil pedido explícitamente encabeza la cadena.
        settings.auto_fallback = true;
        let forced = build_chain(&settings, Some("gem"));
        assert_eq!(forced[0].id, "gem");
    }

    #[test]
    fn pair_flips_between_the_two_configured_languages() {
        let settings = AppSettings::default(); // ES <-> EN-US
        let es = "Necesito que revises el informe de ayer porque hay algunos números que no cuadran con lo que habíamos hablado.";
        let en = "I need you to review yesterday's report because some of the numbers do not match what we discussed.";
        assert_eq!(resolve_target(&settings, es, None), "EN-US");
        assert_eq!(resolve_target(&settings, en, None), "ES");
        // Forzar siempre gana.
        assert_eq!(resolve_target(&settings, "cualquier cosa", Some("FR".into())), "FR");
    }

    #[test]
    fn pair_works_for_non_english_languages() {
        let mut settings = AppSettings::default();
        settings.lang_a = "ES".into();
        settings.lang_b = "PT-BR".into();
        let es = "Necesito que revises el informe de ayer porque hay varios números que no cuadran.";
        let pt = "Preciso que você revise o relatório de ontem porque há vários números que não batem.";
        assert_eq!(resolve_target(&settings, es, None), "PT-BR");
        assert_eq!(resolve_target(&settings, pt, None), "ES");
        // Un idioma fuera del par cae en lang_a.
        let en = "I need you to review yesterday's report because several numbers do not add up.";
        assert_eq!(resolve_target(&settings, en, None), "ES");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .invoke_handler(tauri::generate_handler![
            replace_text,
            get_settings,
            save_settings,
            set_active_profile,
            sanitize_preview,
            test_profile,
            set_view_size,
            hide_window,
            force_translate,
            ollama::ollama_status,
            ollama::ollama_start,
            ollama::ollama_install,
            ollama::ollama_pull
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Setup Tray Icon
            let quit_i = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Configuración", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let icon_bytes = include_bytes!("../icons/icon.png");
            let icon = Image::from_bytes(icon_bytes).expect("Failed to load tray icon");

            let tray_builder = TrayIconBuilder::new().menu(&menu).icon(icon);

            let _tray = tray_builder
                .on_menu_event(move |app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    } else if event.id.as_ref() == "settings" {
                        let _ = app.emit("open-settings", ());
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let settings = load_settings(&app_handle);
            
            // Si no hay APIs configuradas, abre la ventana para el onboarding.
            if settings.profiles.is_empty() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            
            register_shortcuts(&app_handle, &settings);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
