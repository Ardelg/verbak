/*
 * Ayudantes para el motor local gratuito (Ollama).
 *
 * Objetivo: que alguien sin conocimientos pueda dejarlo funcionando desde la
 * ventana de ajustes, sin abrir una terminal.
 *   - `ollama_status`  : ¿está instalado? ¿corriendo? ¿qué modelos tiene?
 *   - `ollama_start`   : levanta el servidor si está instalado pero apagado.
 *   - `ollama_install` : descarga el instalador oficial y lo corre en silencio.
 *   - `ollama_pull`    : descarga un modelo por la API local, con progreso.
 *
 * El progreso se emite como eventos Tauri: `ollama-install-progress` y
 * `ollama-pull-progress`, con forma { phase, pct, note }.
 */

use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const OLLAMA_HOST: &str = "http://localhost:11434";
const INSTALLER_URL: &str = "https://ollama.com/download/OllamaSetup.exe";
pub const DEFAULT_MODEL: &str = "gemma3:4b";

#[derive(Debug, Serialize, Clone)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct Progress {
    phase: String,
    pct: u8,
    note: String,
}

fn emit(app: &AppHandle, event: &str, phase: &str, pct: u8, note: &str) {
    let _ = app.emit(
        event,
        Progress {
            phase: phase.into(),
            pct,
            note: note.into(),
        },
    );
}

fn quick_client(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .build()
        .unwrap_or_default()
}

/// Ruta típica de la instalación por usuario de Ollama en Windows.
fn ollama_dir() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let dir = PathBuf::from(local).join("Programs").join("Ollama");
    if dir.exists() {
        Some(dir)
    } else {
        None
    }
}

fn ollama_on_path() -> Option<PathBuf> {
    let out = std::process::Command::new("where")
        .arg("ollama")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|l| PathBuf::from(l.trim()))
        .filter(|p| p.exists())
}

fn installed_on_disk() -> bool {
    ollama_dir().is_some() || ollama_on_path().is_some()
}

async fn fetch_models() -> Option<Vec<String>> {
    let res = quick_client(6)
        .get(format!("{OLLAMA_HOST}/api/tags"))
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    let v: Value = res.json().await.ok()?;
    Some(
        v.get("models")?
            .as_array()?
            .iter()
            .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect(),
    )
}

#[tauri::command]
pub async fn ollama_status() -> OllamaStatus {
    match fetch_models().await {
        Some(models) => OllamaStatus {
            installed: true,
            running: true,
            models,
        },
        None => OllamaStatus {
            installed: installed_on_disk(),
            running: false,
            models: vec![],
        },
    }
}

#[tauri::command]
pub async fn ollama_start() -> Result<(), String> {
    let target = ollama_dir()
        .map(|d| d.join("ollama app.exe"))
        .filter(|p| p.exists())
        .or_else(|| ollama_dir().map(|d| d.join("ollama.exe")).filter(|p| p.exists()))
        .or_else(ollama_on_path)
        .ok_or_else(|| "Ollama no está instalado en este equipo.".to_string())?;

    let mut cmd = std::process::Command::new(&target);
    if target.file_name().and_then(|f| f.to_str()) == Some("ollama.exe") {
        cmd.arg("serve");
    }
    cmd.spawn()
        .map_err(|e| format!("No se pudo iniciar Ollama: {e}"))?;

    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(600)).await;
        if fetch_models().await.is_some() {
            return Ok(());
        }
    }
    Err("Ollama no respondió después de iniciarse. Intenta abrirlo desde el menú Inicio.".into())
}

#[tauri::command]
pub async fn ollama_install(app: AppHandle) -> Result<(), String> {
    let ev = "ollama-install-progress";
    emit(&app, ev, "download", 0, "Descargando el instalador de Ollama (~1 GB)...");

    let dest = std::env::temp_dir().join("VerbakOllamaSetup.exe");
    let client = quick_client(1800);
    let mut res = client
        .get(INSTALLER_URL)
        .send()
        .await
        .map_err(|e| format!("No se pudo descargar el instalador: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("La descarga falló (HTTP {}).", res.status().as_u16()));
    }

    let total = res.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(&dest)
        .map_err(|e| format!("No se pudo crear el archivo temporal: {e}"))?;
    while let Some(chunk) = res
        .chunk()
        .await
        .map_err(|e| format!("Se cortó la descarga: {e}"))?
    {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 {
            ((downloaded.saturating_mul(90)) / total) as u8
        } else {
            45
        };
        emit(
            &app,
            ev,
            "download",
            pct,
            &format!("Descargando... {} MB", downloaded / 1_048_576),
        );
    }
    drop(file);

    emit(&app, ev, "install", 92, "Instalando Ollama (sin ventanas)...");
    let status = tokio::process::Command::new(&dest)
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
        .status()
        .await
        .map_err(|e| format!("No se pudo ejecutar el instalador: {e}"))?;
    if !status.success() {
        return Err(
            "El instalador de Ollama terminó con error. Instálalo manualmente desde ollama.com.".into(),
        );
    }

    emit(&app, ev, "starting", 95, "Esperando a que Ollama arranque...");
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(750)).await;
        if fetch_models().await.is_some() {
            emit(&app, ev, "done", 100, "Ollama quedó instalado y funcionando.");
            return Ok(());
        }
    }

    // A veces hay que darle el primer empujón.
    let _ = ollama_start().await;
    if fetch_models().await.is_some() {
        emit(&app, ev, "done", 100, "Ollama quedó instalado y funcionando.");
        Ok(())
    } else {
        Err("Ollama se instaló pero no respondió. Reinicia el equipo o ábrelo desde el menú Inicio.".into())
    }
}

#[tauri::command]
pub async fn ollama_pull(app: AppHandle, model: Option<String>) -> Result<(), String> {
    let ev = "ollama-pull-progress";
    let model = model
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    emit(&app, ev, "pull", 0, &format!("Preparando la descarga de {model}..."));

    let client = quick_client(3600);
    let mut res = client
        .post(format!("{OLLAMA_HOST}/api/pull"))
        .json(&json!({ "name": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("¿Ollama está corriendo? {e}"))?;

    if !res.status().is_success() {
        let code = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        return Err(format!(
            "Ollama {code}: {}",
            body.chars().take(200).collect::<String>()
        ));
    }

    let mut buf = String::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(nl) = buf.find('\n') {
            let line: String = buf.drain(..=nl).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Ollama: {err}"));
            }
            let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
            let completed = v.get("completed").and_then(|c| c.as_u64());
            let total = v.get("total").and_then(|t| t.as_u64());
            let pct = match (completed, total) {
                (Some(c), Some(t)) if t > 0 => ((c.saturating_mul(100)) / t) as u8,
                _ => 0,
            };
            emit(&app, ev, "pull", pct, status);
            if status == "success" {
                emit(&app, ev, "done", 100, "El modelo quedó listo.");
                return Ok(());
            }
        }
    }

    emit(&app, ev, "done", 100, "Descarga finalizada.");
    Ok(())
}
