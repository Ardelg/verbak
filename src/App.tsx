import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import { openUrl } from "@tauri-apps/plugin-opener";

type ProviderId = "deepl" | "gemini" | "openai" | "anthropic" | "google";

interface ApiProfile {
  id: string;
  name: string;
  provider: ProviderId;
  api_key: string;
  model: string;
  base_url: string;
  enabled: boolean;
}

interface CustomRule {
  from: string;
  to: string;
}

interface SanitizeSettings {
  enabled: boolean;
  dashes: boolean;
  semicolons: boolean;
  semicolon_replacement: string;
  quotes: boolean;
  ellipsis: boolean;
  spaces: boolean;
  bullets: boolean;
  preserve_code: boolean;
  custom: CustomRule[];
}

interface AppSettings {
  api_key: string;
  profiles: ApiProfile[];
  active_profile: string;
  auto_fallback: boolean;
  target_lang: string;
  lang_a: string;
  lang_b: string;
  context: string;
  slack_context: string;
  opacity: number;
  sanitize: SanitizeSettings;
  shortcut_a: string;
  profile_a: string | null;
  shortcut_b: string;
  profile_b: string | null;
  shortcut_slack: string;
  profile_slack: string | null;
}

interface TranslationResult {
  text: string;
  profile_name: string;
  provider: string;
  warnings: string[];
}

const PROVIDERS: Record<
  ProviderId,
  {
    label: string;
    keyHint: string;
    baseHint: string;
    models: string[];
    keyUrl?: string;
    keyCta?: string;
    note?: string;
    free?: "full" | "keyless";
  }
> = {
  google: {
    label: "Google Traducción",
    keyHint: "no necesita clave",
    baseHint: "endpoint público de Google Translate",
    models: [],
    free: "keyless",
    note:
      "Gratis y sin configuración, funciona en cualquier PC. Traduce frase a frase: no aplica el contexto ni el tono. Para eso usa Gemini, Ollama o DeepL.",
  },
  gemini: {
    label: "Google Gemini",
    keyHint: "AIza...",
    baseHint: "https://generativelanguage.googleapis.com/v1beta",
    models: ["gemini-3.1-flash-lite", "gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"],
    keyUrl: "https://aistudio.google.com/apikey",
    keyCta: "Obtener clave gratuita ↗",
    free: "full",
    note: "Clave gratuita sin tarjeta. Entiende el contexto y el tono. Recomendado.",
  },
  deepl: {
    label: "DeepL",
    keyHint: "0af58aec-...:fx",
    baseHint: "Automático según la clave (Free / Pro)",
    models: [],
    keyUrl: "https://www.deepl.com/your-account/keys",
    keyCta: "Obtener clave ↗",
  },
  openai: {
    label: "OpenAI y compatibles",
    keyHint: "sk-...",
    baseHint: "https://api.openai.com/v1",
    models: [
      "gpt-4.1-mini",
      "gpt-4.1",
      "gpt-4o-mini",
      "gemma3:4b",
      "qwen2.5",
      "llama-3.3-70b-versatile",
      "deepseek-chat",
      "mistral-large-latest",
      "llama3.2",
    ],
    keyUrl: "https://platform.openai.com/api-keys",
    keyCta: "Obtener clave ↗",
  },
  anthropic: {
    label: "Anthropic (Claude)",
    keyHint: "sk-ant-...",
    baseHint: "https://api.anthropic.com/v1",
    models: ["claude-sonnet-5", "claude-opus-5", "claude-haiku-4-5-20251001"],
    keyUrl: "https://console.anthropic.com/settings/keys",
    keyCta: "Obtener clave ↗",
  },
};

/** Altas rápidas: proveedores que hablan el mismo protocolo pero otro endpoint. */
const PRESETS: { key: string; name: string; profile: Omit<ApiProfile, "id"> }[] = [
  {
    key: "google",
    name: "Google Traducción · gratis, sin clave",
    profile: { name: "Google", provider: "google", api_key: "", model: "", base_url: "", enabled: true },
  },
  {
    key: "gemini",
    name: "Google Gemini · gratis con clave, entiende contexto",
    profile: { name: "Gemini", provider: "gemini", api_key: "", model: "gemini-3.1-flash-lite", base_url: "", enabled: true },
  },
  {
    key: "ollama",
    name: "Ollama · gratis, local y offline, entiende contexto",
    profile: {
      name: "Ollama",
      provider: "openai",
      api_key: "",
      model: "gemma3:4b",
      base_url: "http://localhost:11434/v1",
      enabled: true,
    },
  },
  {
    key: "deepl",
    name: "DeepL",
    profile: { name: "DeepL", provider: "deepl", api_key: "", model: "", base_url: "", enabled: true },
  },
  {
    key: "openai",
    name: "OpenAI",
    profile: { name: "OpenAI", provider: "openai", api_key: "", model: "gpt-4.1-mini", base_url: "", enabled: true },
  },
  {
    key: "anthropic",
    name: "Anthropic (Claude)",
    profile: { name: "Claude", provider: "anthropic", api_key: "", model: "claude-sonnet-5", base_url: "", enabled: true },
  },
  {
    key: "groq",
    name: "Groq",
    profile: {
      name: "Groq",
      provider: "openai",
      api_key: "",
      model: "llama-3.3-70b-versatile",
      base_url: "https://api.groq.com/openai/v1",
      enabled: true,
    },
  },
  {
    key: "openrouter",
    name: "OpenRouter",
    profile: {
      name: "OpenRouter",
      provider: "openai",
      api_key: "",
      model: "google/gemini-2.5-flash",
      base_url: "https://openrouter.ai/api/v1",
      enabled: true,
    },
  },
  {
    key: "deepseek",
    name: "DeepSeek",
    profile: {
      name: "DeepSeek",
      provider: "openai",
      api_key: "",
      model: "deepseek-chat",
      base_url: "https://api.deepseek.com/v1",
      enabled: true,
    },
  },
  {
    key: "mistral",
    name: "Mistral",
    profile: {
      name: "Mistral",
      provider: "openai",
      api_key: "",
      model: "mistral-large-latest",
      base_url: "https://api.mistral.ai/v1",
      enabled: true,
    },
  },
];

const EMPTY_SANITIZE: SanitizeSettings = {
  enabled: true,
  dashes: true,
  semicolons: true,
  semicolon_replacement: ",",
  quotes: true,
  ellipsis: true,
  spaces: true,
  bullets: true,
  preserve_code: true,
  custom: [],
};

type SettingsTab = "apis" | "translation" | "style" | "general";

/** Idiomas ofrecidos para el par A ↔ B. El código es estilo DeepL. */
const LANGS: { code: string; name: string; short: string }[] = [
  { code: "ES", name: "Español", short: "ES" },
  { code: "EN-US", name: "Inglés (US)", short: "EN" },
  { code: "EN-GB", name: "Inglés (UK)", short: "EN-UK" },
  { code: "PT-BR", name: "Portugués (Brasil)", short: "PT-BR" },
  { code: "PT-PT", name: "Portugués (Portugal)", short: "PT-PT" },
  { code: "FR", name: "Francés", short: "FR" },
  { code: "DE", name: "Alemán", short: "DE" },
  { code: "IT", name: "Italiano", short: "IT" },
  { code: "NL", name: "Neerlandés", short: "NL" },
  { code: "PL", name: "Polaco", short: "PL" },
  { code: "RU", name: "Ruso", short: "RU" },
  { code: "JA", name: "Japonés", short: "JA" },
  { code: "ZH", name: "Chino", short: "ZH" },
  { code: "KO", name: "Coreano", short: "KO" },
];
const langName = (code: string) => LANGS.find((l) => l.code === code)?.name ?? code;
const langShort = (code: string) => LANGS.find((l) => l.code === code)?.short ?? code;
const langFamily = (code: string) => code.toUpperCase().split(/[-_]/)[0];

function VerbakMark({ className = "" }: { className?: string }) {
  return (
    <svg viewBox="0 0 512 512" className={className} aria-hidden="true">
      <defs>
        <linearGradient id="vk-mark" x1="64" y1="48" x2="448" y2="464" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#6366F1" />
          <stop offset="0.55" stopColor="#4F46E5" />
          <stop offset="1" stopColor="#4338CA" />
        </linearGradient>
      </defs>
      <rect x="16" y="16" width="480" height="480" rx="120" fill="url(#vk-mark)" />
      <path
        d="M150 150 L256 356 L362 150"
        stroke="#ffffff"
        strokeWidth="66"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <rect x="366" y="104" width="52" height="52" rx="17" fill="#ffffff" fillOpacity="0.92" />
    </svg>
  );
}

interface OllamaState {
  installed: boolean;
  running: boolean;
  models: string[];
}
interface OllamaProgress {
  phase: string;
  pct: number;
  note: string;
}

/** Deja Ollama listo sin abrir una terminal: detectar, instalar, bajar el modelo. */
function OllamaSetup({ model }: { model: string }) {
  const wanted = (model || "gemma3:4b").trim();
  const [state, setState] = useState<OllamaState | null>(null);
  const [busy, setBusy] = useState<"" | "install" | "pull" | "start">("");
  const [prog, setProg] = useState<OllamaProgress>({ phase: "", pct: 0, note: "" });
  const [err, setErr] = useState("");

  const refresh = () => invoke<OllamaState>("ollama_status").then(setState).catch(() => {});

  useEffect(() => {
    refresh();
    const a = listen<OllamaProgress>("ollama-install-progress", (e) => setProg(e.payload));
    const b = listen<OllamaProgress>("ollama-pull-progress", (e) => setProg(e.payload));
    return () => {
      a.then((f) => f());
      b.then((f) => f());
    };
  }, []);

  const run = async (kind: "install" | "pull" | "start", cmd: string, args?: Record<string, unknown>) => {
    setErr("");
    setBusy(kind);
    setProg({ phase: "", pct: 0, note: "Empezando..." });
    try {
      await invoke(cmd, args);
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy("");
    }
  };

  const baseModel = (m: string) => m.split(":")[0];
  const hasModel = !!state?.models.some((m) => m === wanted || baseModel(m) === baseModel(wanted));

  const bar = (
    <div className="mt-1">
      <div className="h-1.5 w-full rounded-full bg-slate-200 overflow-hidden">
        <div
          className="h-full bg-indigo-600 transition-[width] duration-300"
          style={{ width: `${Math.min(Math.max(prog.pct, 3), 100)}%` }}
        />
      </div>
      <p className="text-[10.5px] text-slate-500 mt-1">
        {prog.note} {prog.pct ? `· ${prog.pct}%` : ""}
      </p>
    </div>
  );

  return (
    <div className="rounded-xl border border-slate-200 bg-slate-50/70 p-2.5 flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[11px] font-medium text-slate-600">
          {state === null
            ? "Comprobando Ollama..."
            : !state.installed
            ? "Ollama no está instalado"
            : !state.running
            ? "Ollama instalado, apagado"
            : hasModel
            ? `Listo · Ollama + ${wanted}`
            : `Ollama funcionando · falta el modelo ${wanted}`}
        </span>
        <button
          type="button"
          onClick={refresh}
          className="text-[10.5px] text-slate-400 hover:text-slate-700 focus:outline-none"
        >
          Verificar
        </button>
      </div>

      {state && !state.installed && (
        <>
          <button
            type="button"
            disabled={!!busy}
            onClick={() => run("install", "ollama_install")}
            className="self-start px-3 py-1.5 text-[11px] font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg disabled:opacity-50 focus:outline-none"
          >
            {busy === "install" ? "Instalando..." : "Instalar Ollama automáticamente (~1 GB)"}
          </button>
          <p className="text-[10px] text-slate-400">
            Descarga grande (~1 GB + ~3 GB el modelo). En una PC modesta o con internet lento,
            conviene usar Gemini con clave gratuita.
          </p>
        </>
      )}

      {state?.installed && !state.running && (
        <button
          type="button"
          disabled={!!busy}
          onClick={() => run("start", "ollama_start")}
          className="self-start px-3 py-1.5 text-[11px] font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg disabled:opacity-50 focus:outline-none"
        >
          {busy === "start" ? "Iniciando..." : "Iniciar Ollama"}
        </button>
      )}

      {state?.running && !hasModel && (
        <button
          type="button"
          disabled={!!busy}
          onClick={() => run("pull", "ollama_pull", { model: wanted })}
          className="self-start px-3 py-1.5 text-[11px] font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg disabled:opacity-50 focus:outline-none"
        >
          {busy === "pull" ? "Descargando modelo..." : `Descargar modelo ${wanted} (~3 GB)`}
        </button>
      )}

      {state?.running && hasModel && !busy && (
        <p className="text-[11px] text-green-600 font-medium">Todo listo para traducir gratis y offline.</p>
      )}

      {busy && bar}
      {err && <p className="text-[10.5px] text-red-600 select-text">{err}</p>}
    </div>
  );
}

function App() {
  const [originalText, setOriginalText] = useState("");
  const [translatedText, setTranslatedText] = useState("");
  const [view, setView] = useState<"translate" | "settings" | "onboarding">("translate");
  const [tab, setTab] = useState<SettingsTab>("apis");

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [usedProfile, setUsedProfile] = useState("");
  const [lastLang, setLastLang] = useState<string>("");

  const [autostart, setAutostart] = useState(false);
  const [testStatus, setTestStatus] = useState<Record<string, string>>({});

  const [settings, setSettings] = useState<AppSettings>({
    api_key: "",
    profiles: [],
    active_profile: "",
    auto_fallback: true,
    target_lang: "ES",
    lang_a: "ES",
    lang_b: "EN-US",
    context: "",
    slack_context: "",
    opacity: 100,
    sanitize: EMPTY_SANITIZE,
    shortcut_a: "Ctrl+Alt+D",
    profile_a: null,
    shortcut_b: "Ctrl+Alt+F",
    profile_b: null,
    shortcut_slack: "Ctrl+Alt+S",
    profile_slack: null,
  });
  const [saveStatus, setSaveStatus] = useState("");


  const profileIsUsable = (p: ApiProfile) =>
    p.enabled &&
    (p.api_key.trim() !== "" || p.provider === "google" || p.base_url.includes("11434") || p.base_url.includes("localhost:1234"));

  useEffect(() => {
    invoke<AppSettings>("get_settings").then((s) => {
      setSettings(s);
      if (!s.profiles.some(profileIsUsable)) setView("onboarding");
    });
    isEnabled().then(setAutostart);

    const unlistenReady = listen<{
      original: string;
      translated: string;
      profileName: string;
      provider: string;
      warnings: string[];
    }>("translation-ready", (event) => {
      setOriginalText(event.payload.original);
      setTranslatedText(event.payload.translated);
      setUsedProfile(`${event.payload.profileName} (${event.payload.provider})`);
      setError(event.payload.warnings?.length ? event.payload.warnings.join(" | ") : "");
      setLastLang("");
      setView("translate");
      invoke<AppSettings>("get_settings").then(setSettings);
    });

    const unlistenError = listen<string>("translation-error", (event) => {
      setError(event.payload);
      setView("translate");
    });

    const unlistenSettings = listen("open-settings", async () => {
      const currentSettings = await invoke<AppSettings>("get_settings");
      setSettings(currentSettings);
      setAutostart(await isEnabled());
      setView("settings");
      setSaveStatus("");
    });

    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        await invoke("hide_window");
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlistenReady.then((f) => f());
      unlistenError.then((f) => f());
      unlistenSettings.then((f) => f());
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  useEffect(() => {
    invoke("set_view_size", { settingsView: view !== "translate" }).catch(() => {});
  }, [view]);

  const chooseFree = async (kind: "gemini" | "google" | "ollama" | "deepl") => {
    const preset = PRESETS.find((p) => p.key === kind);
    if (!preset) return;
    const id = `${kind}-${Date.now().toString(36)}`;
    const next: AppSettings = {
      ...settings,
      profiles: [...settings.profiles, { ...preset.profile, id }],
      active_profile: id,
    };
    setSettings(next);
    try {
      await invoke("save_settings", { settings: next });
      const fresh = await invoke<AppSettings>("get_settings");
      setSettings(fresh);
    } catch (e) {
      setError(String(e));
    }
    if (kind === "google") {
      setView("translate");
    } else {
      setView("settings");
      setTab("apis");
      if (kind === "gemini") openExternal(PROVIDERS.gemini.keyUrl);
      if (kind === "deepl") openExternal(PROVIDERS.deepl.keyUrl);
    }
  };

  const handleReplace = async () => {
    await invoke("replace_text", { newText: translatedText });
  };

  const handleCancel = async () => {
    await invoke("hide_window");
  };

  const openExternal = (url?: string) => {
    if (url) openUrl(url).catch(() => {});
  };

  const retranslate = async (targetLang?: string, profileId?: string) => {
    if (!originalText || busy) return;
    setBusy(true);
    setError("");
    try {
      const result = await invoke<TranslationResult>("force_translate", {
        text: originalText,
        targetLang: targetLang ?? lastLang ?? null,
        profileId: profileId ?? null,
      });
      setTranslatedText(result.text);
      setUsedProfile(`${result.profile_name} (${result.provider})`);
      if (result.warnings.length) setError(result.warnings.join(" | "));
      if (targetLang !== undefined) setLastLang(targetLang);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleSwitchProfile = async (id: string) => {
    try {
      const updated = await invoke<AppSettings>("set_active_profile", { id });
      setSettings(updated);
      await retranslate(lastLang || undefined, id);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleClean = async () => {
    const cleaned = await invoke<string>("sanitize_preview", { text: translatedText });
    setTranslatedText(cleaned);
  };

  const handleSaveSettings = async () => {
    try {
      await invoke("save_settings", { settings });
      const fresh = await invoke<AppSettings>("get_settings");
      setSettings(fresh);
      setSaveStatus("Guardado con éxito ✔");
      setTimeout(() => setSaveStatus(""), 3000);
    } catch (e) {
      setSaveStatus(`Error al guardar: ${e}`);
    }
  };

  /* ---------------- Perfiles de API ---------------- */

  const updateProfile = (id: string, patch: Partial<ApiProfile>) => {
    setSettings((s) => ({
      ...s,
      profiles: s.profiles.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    }));
  };

  const addProfile = (presetKey: string) => {
    const preset = PRESETS.find((p) => p.key === presetKey);
    if (!preset) return;
    const id = `${preset.key}-${Date.now().toString(36)}`;
    const taken = settings.profiles.filter((p) => p.name.startsWith(preset.profile.name)).length;
    const name = taken ? `${preset.profile.name} ${taken + 1}` : preset.profile.name;
    setSettings((s) => ({
      ...s,
      profiles: [...s.profiles, { ...preset.profile, name, id }],
      active_profile: s.active_profile || id,
    }));
  };

  const removeProfile = (id: string) => {
    setSettings((s) => {
      const profiles = s.profiles.filter((p) => p.id !== id);
      return {
        ...s,
        profiles,
        active_profile:
          s.active_profile === id ? profiles[0]?.id ?? "" : s.active_profile,
      };
    });
  };

  const handleTestProfile = async (profile: ApiProfile) => {
    setTestStatus((s) => ({ ...s, [profile.id]: "Probando..." }));
    try {
      const result = await invoke<string>("test_profile", { profile });
      setTestStatus((s) => ({ ...s, [profile.id]: `OK → "${result}"` }));
    } catch (e) {
      setTestStatus((s) => ({ ...s, [profile.id]: `Error: ${e}` }));
    }
  };

  const setSanitize = (patch: Partial<SanitizeSettings>) =>
    setSettings((s) => ({ ...s, sanitize: { ...s.sanitize, ...patch } }));

  /* ---------------- Estilos base ---------------- */

  const inputClass =
    "bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-sm text-slate-800 placeholder:text-slate-400 focus:outline-none focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/15 transition-colors select-text";
  const checkboxClass =
    "w-4 h-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500/30 focus:ring-2 accent-indigo-600";
  const primaryBtn =
    "px-5 py-2 text-sm font-semibold text-white bg-indigo-600 hover:bg-indigo-700 active:bg-indigo-800 rounded-xl shadow-sm shadow-indigo-600/20 transition-all focus:outline-none focus:ring-2 focus:ring-indigo-500/40 disabled:opacity-50";
  const secondaryBtn =
    "px-4 py-2 text-sm font-medium text-slate-600 bg-white hover:bg-slate-50 hover:text-slate-900 border border-slate-200 rounded-xl transition-all focus:outline-none focus:ring-2 focus:ring-slate-300/60 disabled:opacity-50";

  /* ---------------- Render ---------------- */

  return (
    <div
      className="flex flex-col w-full h-screen text-slate-700 p-4 font-sans select-none rounded-[20px] border border-slate-200/80 shadow-[0_1px_3px_rgba(15,23,42,0.06),0_24px_56px_-16px_rgba(15,23,42,0.28)] transition-colors duration-200 overflow-hidden"
      style={{
        backgroundColor: `rgba(255, 255, 255, ${Math.max(settings.opacity, 35) / 100})`,
        backdropFilter: "blur(16px)",
        WebkitBackdropFilter: "blur(16px)",
      }}
    >
      <div className="flex justify-between items-center mb-3 cursor-move" data-tauri-drag-region>
        <div className="flex items-center gap-2.5 pointer-events-none">
          <VerbakMark className="w-7 h-7 rounded-lg shadow-sm shadow-indigo-600/25" />
          <div className="flex items-baseline gap-2">
            <span className="text-[15px] font-bold tracking-tight text-slate-900">
              Verba<span className="text-indigo-600">k</span>
            </span>
            <span className="text-[11px] font-medium text-slate-400 uppercase tracking-widest">
              {view === "translate" ? "Revisión" : view === "onboarding" ? "Bienvenido" : "Ajustes"}
            </span>
          </div>
          {view === "settings" && (
            <div className="relative group pointer-events-auto ml-1">
              <span className="text-slate-400 hover:text-indigo-500 cursor-help text-xs bg-slate-100 rounded-full w-5 h-5 flex items-center justify-center border border-slate-200">
                ?
              </span>
              <div className="absolute left-0 top-6 w-72 bg-white border border-slate-200 rounded-xl shadow-xl p-3 text-xs text-slate-600 opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-50">
                <p className="font-semibold text-slate-900 mb-2 border-b border-slate-100 pb-1">Atajos de teclado</p>
                <ul className="space-y-2 mb-3">
                  <li><strong className="text-indigo-600">{settings.shortcut_a}</strong><br />Reemplazo automático (usa este contexto).</li>
                  <li><strong className="text-indigo-600">{settings.shortcut_b}</strong><br />Revisión manual (abre esta ventana).</li>
                  <li><strong className="text-indigo-600">{settings.shortcut_slack}</strong><br />Modo Slack (reemplazo automático informal).</li>
                </ul>
                <div className="border-t border-slate-100 pt-2 text-center text-[10px] text-slate-400">
                  Creado por <a href="#" onClick={(e) => { e.preventDefault(); openExternal("https://github.com/Ardelg"); }} className="text-indigo-600 hover:underline">Ariel Delgue</a>
                </div>
              </div>
            </div>
          )}
        </div>
        <button
          onClick={handleCancel}
          className="text-slate-400 hover:text-red-500 hover:bg-red-50 rounded-lg w-7 h-7 flex items-center justify-center transition-colors focus:outline-none"
          aria-label="Cerrar"
        >
          ✕
        </button>
      </div>

      {error && (
        <div className="mb-2 px-3 py-2 rounded-xl bg-red-50 border border-red-200 text-[11px] text-red-700 select-text max-h-16 overflow-y-auto custom-scrollbar">
          {error}
        </div>
      )}

      {view === "onboarding" ? (
        <div className="flex flex-col flex-grow gap-3 overflow-y-auto custom-scrollbar pr-1">
          <div>
            <h3 className="text-lg font-bold text-slate-900 tracking-tight">¿Cómo quieres usar Verbak?</h3>
            <p className="text-xs text-slate-500 mt-0.5">
              Elige un motor de traducción. Puedes cambiarlo o agregar más después, en Ajustes.
            </p>
          </div>

          {([
            {
              key: "gemini" as const,
              badge: "Recomendado",
              badgeClass: "bg-indigo-100 text-indigo-700",
              title: "Google Gemini · gratis con clave",
              body: "Mejor calidad, entiende el contexto y el tono. Funciona en cualquier PC. Hay que crear una clave gratuita (sin tarjeta); se abre la página.",
              cta: "Usar Gemini y abrir la página de la clave",
            },
            {
              key: "google" as const,
              badge: "Cero configuración",
              badgeClass: "bg-slate-100 text-slate-600",
              title: "Google Traducción · sin clave",
              body: "Empieza a funcionar ahora mismo, sin registro. Traduce frase a frase: no aplica el contexto ni el tono.",
              cta: "Usar Google sin clave y empezar",
            },
            {
              key: "ollama" as const,
              badge: "Local y privado",
              badgeClass: "bg-slate-100 text-slate-600",
              title: "Ollama · gratis, en tu equipo",
              body: "Todo offline, nada sale de tu PC. Entiende el contexto. Descarga grande (~4 GB) y necesita ~8 GB de RAM. Verbak lo instala solo.",
              cta: "Configurar Ollama",
            },
            {
              key: "deepl" as const,
              badge: "Clásico",
              badgeClass: "bg-slate-100 text-slate-600",
              title: "DeepL",
              body: "Muy buena calidad y respeta el contexto. Nota: la API gratuita de DeepL cerró para cuentas nuevas en 2026; puede pedir tarjeta o plan de pago.",
              cta: "Usar DeepL y abrir la página de la clave",
            },
          ]).map((opt) => (
            <button
              key={opt.key}
              onClick={() => chooseFree(opt.key)}
              className="text-left rounded-2xl border border-slate-200 bg-white hover:border-indigo-300 hover:bg-indigo-50/40 transition-colors p-3 focus:outline-none focus:ring-2 focus:ring-indigo-500/30"
            >
              <div className="flex items-center gap-2 mb-1">
                <span className={`text-[10px] font-semibold uppercase tracking-wide px-1.5 py-0.5 rounded ${opt.badgeClass}`}>
                  {opt.badge}
                </span>
                <span className="text-sm font-semibold text-slate-900">{opt.title}</span>
              </div>
              <p className="text-[11.5px] text-slate-500 leading-snug">{opt.body}</p>
              <span className="inline-block mt-2 text-[11px] font-medium text-indigo-600">{opt.cta} →</span>
            </button>
          ))}

          <button
            onClick={() => setView("settings")}
            className="self-center text-[11px] text-slate-400 hover:text-slate-700 mt-1 focus:outline-none"
          >
            Prefiero configurarlo a mano
          </button>
        </div>
      ) : view === "translate" ? (
        <>
          <div className="flex flex-col flex-grow">
            <textarea
              value={translatedText}
              onChange={(e) => setTranslatedText(e.target.value)}
              className="flex-1 resize-none bg-white border border-slate-200 rounded-2xl p-4 text-base text-slate-900 focus:outline-none focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/15 custom-scrollbar select-text leading-relaxed shadow-inner shadow-slate-100 transition-colors"
              autoFocus
              placeholder="Traducción..."
            />
          </div>

          <div className="flex items-center gap-2 mt-3 text-[11px] text-slate-500">
            <span className="uppercase tracking-wider font-medium text-slate-400">API</span>
            <select
              value={settings.active_profile}
              onChange={(e) => handleSwitchProfile(e.target.value)}
              disabled={busy}
              className="bg-slate-50 border border-slate-200 rounded-lg px-2 py-1 text-[11px] text-slate-700 focus:outline-none focus:border-indigo-500 disabled:opacity-50"
            >
              {settings.profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} · {PROVIDERS[p.provider]?.label ?? p.provider}
                </option>
              ))}
            </select>
            {busy && <span className="text-indigo-600 font-medium">Traduciendo...</span>}
            {!busy && usedProfile && <span className="text-slate-400">usada: {usedProfile}</span>}
          </div>

          <div className="flex justify-between gap-3 mt-3 items-center w-full">
            <div className="flex gap-2">
              <button
                onClick={() => retranslate(settings.lang_a)}
                disabled={busy}
                title={`Traducir a ${langName(settings.lang_a)}`}
                className="px-4 py-2 text-sm font-medium text-indigo-700 bg-indigo-50 hover:bg-indigo-100 border border-indigo-100 rounded-xl transition-all focus:outline-none disabled:opacity-50"
              >
                {langShort(settings.lang_a)}
              </button>
              <button
                onClick={() => retranslate(settings.lang_b)}
                disabled={busy}
                title={`Traducir a ${langName(settings.lang_b)}`}
                className="px-4 py-2 text-sm font-medium text-indigo-700 bg-indigo-50 hover:bg-indigo-100 border border-indigo-100 rounded-xl transition-all focus:outline-none disabled:opacity-50"
              >
                {langShort(settings.lang_b)}
              </button>
              <button
                onClick={handleClean}
                title="Reemplaza guiones largos, punto y coma y comillas tipográficas"
                className={secondaryBtn}
              >
                Limpiar
              </button>
            </div>

            <div className="flex gap-2 items-center">
              <button
                onClick={() => setView("settings")}
                className="px-3 py-2 text-sm font-medium text-slate-500 hover:text-slate-900 transition-colors focus:outline-none"
              >
                ⚙ Ajustes
              </button>
              <button onClick={handleCancel} className={secondaryBtn}>
                Cancelar
              </button>
              <button onClick={handleReplace} className={primaryBtn}>
                Reemplazar
              </button>
            </div>
          </div>
        </>
      ) : (
        <>
          <div className="flex gap-1.5 mb-3">
            {([
              ["apis", "APIs"],
              ["translation", "Traducción"],
              ["style", "Estilo de salida"],
              ["general", "General"],
            ] as [SettingsTab, string][]).map(([key, label]) => (
              <button
                key={key}
                onClick={() => setTab(key)}
                className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors focus:outline-none ${
                  tab === key
                    ? "bg-indigo-50 border-indigo-200 text-indigo-700"
                    : "bg-white border-slate-200 text-slate-500 hover:text-slate-800 hover:border-slate-300"
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          <div className="flex flex-col flex-grow gap-4 overflow-y-auto custom-scrollbar pr-2 -mr-2">
            {tab === "apis" && (
              <>
                <p className="text-[11px] text-slate-500 ml-0.5">
                  El perfil marcado es el que usan los atajos. Si falla, se prueban los demás
                  automáticamente. Las claves se guardan solo en este equipo.
                </p>

                {settings.profiles.map((profile) => {
                  const meta = PROVIDERS[profile.provider];
                  const status = testStatus[profile.id];
                  const isActive = settings.active_profile === profile.id;
                  const isOllama = profile.base_url.includes("11434") || profile.base_url.includes("localhost:1234");
                  return (
                    <div
                      key={profile.id}
                      className={`flex flex-col gap-2 p-3 rounded-2xl border transition-colors ${
                        isActive
                          ? "border-indigo-300 bg-indigo-50/50 ring-1 ring-indigo-200"
                          : "border-slate-200 bg-white"
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        <input
                          type="radio"
                          name="active-profile"
                          checked={isActive}
                          onChange={() => setSettings((s) => ({ ...s, active_profile: profile.id }))}
                          className={checkboxClass}
                          title="Usar esta API por defecto"
                        />
                        <input
                          value={profile.name}
                          onChange={(e) => updateProfile(profile.id, { name: e.target.value })}
                          className={`${inputClass} flex-1 py-1`}
                          placeholder="Nombre"
                        />
                        <select
                          value={profile.provider}
                          onChange={(e) =>
                            updateProfile(profile.id, { provider: e.target.value as ProviderId })
                          }
                          className={`${inputClass} py-1`}
                        >
                          {Object.entries(PROVIDERS).map(([id, p]) => (
                            <option key={id} value={id}>
                              {p.label}
                            </option>
                          ))}
                        </select>
                        <label className="flex items-center gap-1 text-[11px] text-slate-500 cursor-pointer">
                          <input
                            type="checkbox"
                            checked={profile.enabled}
                            onChange={(e) => updateProfile(profile.id, { enabled: e.target.checked })}
                            className={checkboxClass}
                          />
                          activa
                        </label>
                      </div>

                      {meta?.free === "keyless" ? (
                        <p className="text-[11px] text-amber-700 bg-amber-50 border border-amber-100 rounded-lg px-2.5 py-1.5">
                          {meta.note}
                        </p>
                      ) : (
                        <>
                          <div className="flex items-center gap-2">
                            <input
                              type="password"
                              value={profile.api_key}
                              onChange={(e) => updateProfile(profile.id, { api_key: e.target.value })}
                              placeholder={`API Key · ${meta?.keyHint ?? ""}`}
                              className={`${inputClass} flex-1`}
                            />
                            {meta?.keyUrl && (
                              <button
                                type="button"
                                onClick={() => openExternal(meta.keyUrl)}
                                className="whitespace-nowrap text-[11px] font-medium text-indigo-600 hover:text-indigo-700 hover:underline focus:outline-none"
                              >
                                {meta.keyCta ?? "Obtener clave ↗"}
                              </button>
                            )}
                          </div>
                          {meta?.note && meta.free === "full" && (
                            <p className="text-[10.5px] text-slate-400 ml-0.5 -mt-1">{meta.note}</p>
                          )}
                        </>
                      )}

                      {profile.provider !== "deepl" && profile.provider !== "google" && (
                        <input
                          list={`models-${profile.id}`}
                          value={profile.model}
                          onChange={(e) => updateProfile(profile.id, { model: e.target.value })}
                          placeholder={`Modelo · por defecto ${meta?.models[0] ?? ""}`}
                          className={`${inputClass} w-full`}
                        />
                      )}
                      <datalist id={`models-${profile.id}`}>
                        {(meta?.models ?? []).map((m) => (
                          <option key={m} value={m} />
                        ))}
                      </datalist>

                      {profile.provider !== "google" && (
                        <input
                          value={profile.base_url}
                          onChange={(e) => updateProfile(profile.id, { base_url: e.target.value })}
                          placeholder={`Endpoint (opcional) · ${meta?.baseHint ?? ""}`}
                          className={`${inputClass} w-full text-[11px]`}
                        />
                      )}

                      {isOllama && <OllamaSetup model={profile.model} />}

                      <div className="flex items-center gap-3 flex-wrap">
                        <button
                          onClick={() => handleTestProfile(profile)}
                          className="px-3 py-1 text-[11px] font-medium text-indigo-700 bg-indigo-50 hover:bg-indigo-100 border border-indigo-100 rounded-lg focus:outline-none"
                        >
                          Probar
                        </button>
                        <button
                          onClick={() => removeProfile(profile.id)}
                          className="px-3 py-1 text-[11px] text-slate-500 hover:text-red-600 border border-slate-200 hover:border-red-200 rounded-lg focus:outline-none"
                        >
                          Eliminar
                        </button>
                        {status && (
                          <span
                            className={`text-[11px] truncate select-text ${
                              status.startsWith("Error") ? "text-red-600" : "text-green-600"
                            }`}
                            title={status}
                          >
                            {status}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}

                <div className="flex items-center gap-2">
                  <select
                    value=""
                    onChange={(e) => addProfile(e.target.value)}
                    className={`${inputClass} flex-1`}
                  >
                    <option value="">+ Añadir API...</option>
                    {PRESETS.map((p) => (
                      <option key={p.key} value={p.key}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </div>

                <label className="flex items-center gap-2 cursor-pointer ml-0.5">
                  <input
                    type="checkbox"
                    checked={settings.auto_fallback}
                    onChange={(e) => setSettings({ ...settings, auto_fallback: e.target.checked })}
                    className={checkboxClass}
                  />
                  <span className="text-sm text-slate-600">Usar otra API si la principal falla</span>
                </label>
              </>
            )}

            {tab === "translation" && (
              <>
                <div className="flex flex-col">
                  <label className="text-xs text-slate-500 mb-1 ml-0.5">Par de idiomas</label>
                  <div className="flex items-center gap-2">
                    <select
                      value={settings.lang_a}
                      onChange={(e) => setSettings({ ...settings, lang_a: e.target.value })}
                      className={`${inputClass} p-3 flex-1`}
                    >
                      {LANGS.map((l) => (
                        <option key={l.code} value={l.code}>{l.name}</option>
                      ))}
                    </select>
                    <span className="text-slate-400 text-sm font-semibold">↔</span>
                    <select
                      value={settings.lang_b}
                      onChange={(e) => setSettings({ ...settings, lang_b: e.target.value })}
                      className={`${inputClass} p-3 flex-1`}
                    >
                      {LANGS.map((l) => (
                        <option key={l.code} value={l.code}>{l.name}</option>
                      ))}
                    </select>
                  </div>
                  <p className="text-[10px] text-slate-400 mt-1 ml-0.5">
                    Los atajos alternan entre los dos según el idioma que detecten. Si el texto no
                    está en ninguno de los dos, se traduce al primero ({langShort(settings.lang_a)}).
                    {langFamily(settings.lang_a) === langFamily(settings.lang_b) && (
                      <span className="text-amber-600"> · Elige dos idiomas distintos.</span>
                    )}
                  </p>
                </div>
                <div className="flex flex-col flex-1">
                  <label className="text-xs text-slate-500 mb-1 ml-0.5">
                    Contexto de traducción (prompt general)
                  </label>
                  <textarea
                    value={settings.context}
                    onChange={(e) => setSettings({ ...settings, context: e.target.value })}
                    className={`${inputClass} flex-1 resize-none p-3 custom-scrollbar min-h-[80px]`}
                    placeholder="Instrucciones para la traducción normal..."
                  />
                </div>
                <div className="flex flex-col flex-1">
                  <label className="text-xs text-slate-500 mb-1 ml-0.5">
                    Contexto modo Slack (prompt informal)
                  </label>
                  <textarea
                    value={settings.slack_context}
                    onChange={(e) => setSettings({ ...settings, slack_context: e.target.value })}
                    className={`${inputClass} flex-1 resize-none p-3 custom-scrollbar min-h-[80px]`}
                    placeholder="Instrucciones para el modo Slack..."
                  />
                </div>
                <p className="text-[10.5px] text-slate-400 ml-0.5">
                  El contexto solo lo aplican DeepL y los motores con IA (Gemini, Claude, OpenAI, Ollama).
                  "Google Traducción" sin clave lo ignora.
                </p>
              </>
            )}

            {tab === "style" && (
              <>
                <label className="flex items-center gap-2 cursor-pointer ml-0.5">
                  <input
                    type="checkbox"
                    checked={settings.sanitize.enabled}
                    onChange={(e) => setSanitize({ enabled: e.target.checked })}
                    className={checkboxClass}
                  />
                  <span className="text-sm text-slate-600">Limpiar la puntuación después de traducir</span>
                </label>
                <p className="text-[11px] text-slate-400 ml-0.5 -mt-2">
                  Se aplica a los tres atajos y a todos los motores. También se le pide al modelo que no la use.
                </p>

                <div className={`flex flex-col gap-3 ml-0.5 ${settings.sanitize.enabled ? "" : "opacity-40 pointer-events-none"}`}>
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.dashes}
                      onChange={(e) => setSanitize({ dashes: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">Guiones largos (— –) → coma, viñeta o nada</span>
                  </label>

                  <div className="flex items-center gap-2 flex-wrap">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={settings.sanitize.semicolons}
                        onChange={(e) => setSanitize({ semicolons: e.target.checked })}
                        className={checkboxClass}
                      />
                      <span className="text-sm text-slate-600">Punto y coma (;) →</span>
                    </label>
                    <select
                      value={settings.sanitize.semicolon_replacement}
                      onChange={(e) => setSanitize({ semicolon_replacement: e.target.value })}
                      className={`${inputClass} py-1`}
                    >
                      <option value=",">coma ( , )</option>
                      <option value=".">punto y mayúscula ( . )</option>
                      <option value=";">no tocar</option>
                    </select>
                  </div>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.quotes}
                      onChange={(e) => setSanitize({ quotes: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">Comillas y apóstrofos tipográficos → rectos</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.ellipsis}
                      onChange={(e) => setSanitize({ ellipsis: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">Puntos suspensivos (…) → ...</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.bullets}
                      onChange={(e) => setSanitize({ bullets: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">Viñetas (• ▪) → -</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.spaces}
                      onChange={(e) => setSanitize({ spaces: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">Espacios raros y dobles espacios</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.sanitize.preserve_code}
                      onChange={(e) => setSanitize({ preserve_code: e.target.checked })}
                      className={checkboxClass}
                    />
                    <span className="text-sm text-slate-600">No tocar `código`, ```bloques``` ni URLs</span>
                  </label>

                  <div className="flex flex-col gap-2 mt-2">
                    <span className="text-xs text-slate-500">Reemplazos propios</span>
                    {settings.sanitize.custom.map((rule, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <input
                          value={rule.from}
                          onChange={(e) =>
                            setSanitize({
                              custom: settings.sanitize.custom.map((r, j) =>
                                j === i ? { ...r, from: e.target.value } : r
                              ),
                            })
                          }
                          placeholder="buscar"
                          className={`${inputClass} flex-1 py-1`}
                        />
                        <span className="text-slate-400 text-xs">→</span>
                        <input
                          value={rule.to}
                          onChange={(e) =>
                            setSanitize({
                              custom: settings.sanitize.custom.map((r, j) =>
                                j === i ? { ...r, to: e.target.value } : r
                              ),
                            })
                          }
                          placeholder="reemplazar por"
                          className={`${inputClass} flex-1 py-1`}
                        />
                        <button
                          onClick={() =>
                            setSanitize({
                              custom: settings.sanitize.custom.filter((_, j) => j !== i),
                            })
                          }
                          className="text-slate-400 hover:text-red-500 text-sm px-1 focus:outline-none"
                        >
                          ✕
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        setSanitize({ custom: [...settings.sanitize.custom, { from: "", to: "" }] })
                      }
                      className="self-start px-3 py-1 text-[11px] font-medium text-indigo-700 bg-indigo-50 hover:bg-indigo-100 border border-indigo-100 rounded-lg focus:outline-none"
                    >
                      + Añadir reemplazo
                    </button>
                  </div>
                </div>
              </>
            )}

            {tab === "general" && (
              <>
                <label className="flex items-center gap-2 cursor-pointer ml-0.5">
                  <input
                    type="checkbox"
                    checked={autostart}
                    onChange={async (e) => {
                      const val = e.target.checked;
                      setAutostart(val);
                      if (val) {
                        await enable();
                      } else {
                        await disable();
                      }
                    }}
                    className={checkboxClass}
                  />
                  <span className="text-sm text-slate-600">Iniciar automáticamente con Windows</span>
                </label>

                <div className="flex flex-col">
                  <div className="flex justify-between items-center mb-2 ml-0.5">
                    <label className="text-xs text-slate-500">Opacidad de la ventana</label>
                    <span className="text-xs text-indigo-600 font-semibold">{settings.opacity}%</span>
                  </div>
                  <input
                    type="range"
                    min="35"
                    max="100"
                    value={settings.opacity}
                    onChange={(e) => setSettings({ ...settings, opacity: parseInt(e.target.value) })}
                    className="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-600"
                  />
                </div>

                <div className="rounded-2xl border border-slate-200 bg-white p-3 text-[11px] text-slate-500 mt-2 flex flex-col gap-3">
                  <p className="text-slate-700 font-medium -mb-1">Atajos y Motores</p>
                  
                  <div className="flex flex-col gap-1.5">
                    <p className="text-slate-600">Traducir y Reemplazar</p>
                    <div className="flex gap-2">
                      <input 
                        value={settings.shortcut_a} 
                        onChange={(e) => setSettings({ ...settings, shortcut_a: e.target.value })} 
                        className={`${inputClass} flex-1 py-1`} 
                        placeholder="Ej: Ctrl+Alt+D"
                      />
                      <select 
                        value={settings.profile_a || ""} 
                        onChange={(e) => setSettings({ ...settings, profile_a: e.target.value || null })} 
                        className={`${inputClass} flex-1 py-1`}
                      >
                        <option value="">(Perfil Activo)</option>
                        {settings.profiles.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                      </select>
                    </div>
                  </div>

                  <div className="flex flex-col gap-1.5">
                    <p className="text-slate-600">Abrir Revisión</p>
                    <div className="flex gap-2">
                      <input 
                        value={settings.shortcut_b} 
                        onChange={(e) => setSettings({ ...settings, shortcut_b: e.target.value })} 
                        className={`${inputClass} flex-1 py-1`} 
                        placeholder="Ej: Ctrl+Alt+F"
                      />
                      <select 
                        value={settings.profile_b || ""} 
                        onChange={(e) => setSettings({ ...settings, profile_b: e.target.value || null })} 
                        className={`${inputClass} flex-1 py-1`}
                      >
                        <option value="">(Perfil Activo)</option>
                        {settings.profiles.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                      </select>
                    </div>
                  </div>

                  <div className="flex flex-col gap-1.5">
                    <p className="text-slate-600">Modo Slack (Reemplazar Informal)</p>
                    <div className="flex gap-2">
                      <input 
                        value={settings.shortcut_slack} 
                        onChange={(e) => setSettings({ ...settings, shortcut_slack: e.target.value })} 
                        className={`${inputClass} flex-1 py-1`} 
                        placeholder="Ej: Ctrl+Alt+S"
                      />
                      <select 
                        value={settings.profile_slack || ""} 
                        onChange={(e) => setSettings({ ...settings, profile_slack: e.target.value || null })} 
                        className={`${inputClass} flex-1 py-1`}
                      >
                        <option value="">(Perfil Activo)</option>
                        {settings.profiles.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                      </select>
                    </div>
                  </div>
                </div>

                <button
                  onClick={() => setView("onboarding")}
                  className="self-start text-[11px] font-medium text-indigo-600 hover:text-indigo-700 focus:outline-none"
                >
                  Ver las opciones de motor gratis
                </button>
              </>
            )}
          </div>

          <div className="flex items-center justify-end gap-3 mt-4">
            {saveStatus && (
              <span
                className={`text-xs mr-auto ml-1 ${
                  saveStatus.startsWith("Error") ? "text-red-600" : "text-green-600"
                }`}
              >
                {saveStatus}
              </span>
            )}
            <button onClick={() => setView("translate")} className={secondaryBtn}>
              Volver
            </button>
            <button onClick={handleSaveSettings} className={primaryBtn}>
              Guardar ajustes
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export default App;
