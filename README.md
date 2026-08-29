# Verbak

**Traductor de escritorio con atajos globales.** Seleccionas texto en cualquier
aplicación, pulsas un atajo y Verbak lo traduce y lo reemplaza en el sitio, sin
cambiar de ventana. Funciona con DeepL, Google Gemini, OpenAI (y compatibles),
Anthropic, un motor local con Ollama y Google Traducción sin clave. Además limpia
la puntuación "de traductor" (guiones largos, punto y coma, comillas tipográficas)
para que el texto no parezca escrito por una máquina.

<sub>*Desktop translator with global hotkeys: translate the current selection in
place, anywhere, using DeepL / Gemini / OpenAI / Claude / a local Ollama model /
key-less Google Translate. Windows, macOS and Linux (Tauri + React + Rust).*</sub>

<p align="center">
  <img src="branding/verbak-logo-light.svg#gh-light-mode-only" width="360" alt="Verbak">
  <img src="branding/verbak-logo-dark.svg#gh-dark-mode-only" width="360" alt="Verbak">
</p>

---

## Descargar

Los instaladores están en la página de **[Releases](../../releases/latest)**.

| Sistema | Archivo | Notas |
|---|---|---|
| **Windows** (recomendado) | `Verbak_x.y.z_x64-setup.exe` | Instalador normal. Crea acceso directo y desinstalador. |
| **Windows** (portable) | `verbak.exe` | Sin instalar: copiar a cualquier carpeta o USB y doble clic. |
| **Windows** (empresa) | `Verbak_x.y.z_x64_en-US.msi` | Para despliegue gestionado por IT: `msiexec /i Verbak.msi /qn`, Intune, GPO. |
| **macOS** | `Verbak_x.y.z_x64.dmg` / `_aarch64.dmg` | Ver *Instalar en macOS* más abajo. |
| **Linux** | `.AppImage` / `.deb` | — |

> **Windows** puede mostrar un aviso azul de SmartScreen porque el instalador no
> está firmado: *Más información → Ejecutar de todas formas*.

### Instalar en macOS

1. Abre el `.dmg` y arrastra **Verbak** a *Aplicaciones*.
2. La primera vez, macOS bloquea las apps sin firmar: clic derecho sobre Verbak →
   **Abrir** → **Abrir**. (O en Terminal: `xattr -cr /Applications/Verbak.app`.)
3. **Ajustes del Sistema → Privacidad y seguridad → Accesibilidad** y activa
   Verbak. Es imprescindible: sin ese permiso no puede copiar ni pegar la
   selección.

## Primer uso

Verbak vive en la **bandeja del sistema** (no abre ventana al arrancar). La
primera vez aparece un asistente: **¿Cómo quieres usar Verbak?** con estas
opciones:

- **Google Gemini** – gratis con una clave que se crea en 30 s sin tarjeta.
  Mejor calidad y entiende el contexto. Recomendado.
- **Google Traducción** – sin clave ni registro. Empieza a funcionar al
  instante. Traduce frase a frase: no aplica el contexto ni el tono.
- **Ollama** – motor de IA local: todo offline, nada sale de tu equipo. Verbak
  puede instalarlo y descargar el modelo por ti (~4 GB, necesita bastante RAM).
- **DeepL** – muy buena calidad. La API gratuita cerró para cuentas nuevas
  en 2026, puede pedir tarjeta o plan de pago.

Se puede volver a abrir en **Ajustes → General → "Ver las opciones de motor
gratis"**.

## Atajos

| Atajo | Qué hace |
|---|---|
| `Ctrl + Alt + D` | Traduce la selección al otro idioma del par y la reemplaza al instante. |
| `Ctrl + Alt + F` | Traduce y abre la ventana de revisión antes de reemplazar. |
| `Ctrl + Alt + S` | Modo Slack: tono informal, traduce y reemplaza. |

### Par de idiomas

En **Ajustes → Traducción** eliges dos idiomas, por ejemplo `Español ↔ Portugués`
o `Español ↔ Alemán`. Los tres atajos alternan entre ellos según el idioma que
detecten en el texto; si no está en ninguno de los dos, traduce al primero.

Idiomas: ES, EN (US/UK), PT (BR/PT), FR, DE, IT, NL, PL, RU, JA, ZH, KO.

Para pares parecidos (ES/PT) la detección automática usa un desempate por
vocabulario, porque el detector a veces confunde frases cortas. Los botones de la
ventana de revisión siempre permiten forzar la dirección.

## Motores y APIs

Se guardan **varios perfiles** a la vez. El marcado con el círculo es el que
usan los atajos; si falla (cuota, caída, clave inválida) se intenta con el resto
automáticamente. Cada perfil tiene un botón **Probar**.

| Proveedor | Clave | ¿Contexto? | Modelo por defecto |
|---|---|---|---|
| **Google Traducción** | no necesita | ❌ frase a frase | — |
| Google Gemini | `AIza…` (gratis, sin tarjeta) | ✅ | `gemini-2.5-flash` |
| Ollama (local) | no necesita | ✅ | `gemma3:4b` |
| DeepL | `…:fx` (Free) o Pro | ✅ (`context`) | — |
| OpenAI y compatibles | `sk-…` | ✅ | `gpt-4.1-mini` |
| Anthropic | `sk-ant-…` | ✅ | `claude-sonnet-5` |

El perfil "OpenAI y compatibles" vale para cualquier endpoint que hable
`chat/completions`: **Groq, OpenRouter, DeepSeek, Mistral y Ollama** están como
altas rápidas.

## Privacidad

Las claves de API **no se guardan en el ejecutable**. Viven solo en el equipo de
cada usuario, en texto plano, en un archivo que la app crea al guardar Ajustes:

- Windows: `%APPDATA%\com.ariel.verbak\settings.json`
- macOS: `~/Library/Application Support/com.ariel.verbak/settings.json`
- Linux: `~/.config/com.ariel.verbak/settings.json`

Ese archivo no está en el repositorio, no se compila y no se incluye en los
instaladores. Se puede compartir el instalador sin exponer ninguna clave.

## Limpieza de puntuación (Ajustes → "Estilo de salida")

Después de traducir se reemplaza:

- `—` `–` `―` → coma (`El deploy — que probamos — salió bien` → `El deploy, que
  probamos, salió bien`), `-` si abre una viñeta, y se descarta si queda colgando
  al final. Los rangos numéricos (`2020–2024`) conservan el guion.
- `;` → coma (configurable: coma, punto con mayúscula, o no tocar).
- Comillas y apóstrofos tipográficos (`" " ' ' « »`) → rectos.
- `…` → `...`, viñetas `•` → `-`, espacios duros y dobles espacios.
- Reemplazos propios definidos por el usuario.

Lo que va dentro de `` `código` ``, de bloques ```` ``` ```` y las URLs no se
toca. A los motores con IA se les pide además en el prompt que no usen esa
puntuación, así que la limpieza actúa como red de seguridad. El botón **Limpiar**
de la ventana de revisión aplica las mismas reglas al texto editado a mano.

## Rendimiento del motor local (Ollama)

`gemma3:4b` en un procesador sin GPU dedicada (por ejemplo un i5 con 16 GB) va
**lento**: puede tardar varios segundos por traducción. Es normal. Opciones:

- Usa un modelo más pequeño y rápido: en el perfil de Ollama cambia el modelo a
  `gemma3:1b`, `llama3.2:1b` o `qwen2.5:1.5b` y ejecuta `ollama pull <modelo>`.
  La calidad baja un poco pero para traducir frases va sobrado.
- O usa **Gemini** con clave gratuita: el trabajo lo hace Google y la respuesta
  es casi instantánea en cualquier equipo.
- Con una GPU (NVIDIA o Apple Silicon) Ollama la aprovecha solo y va mucho más
  rápido.

## Compilar desde el código

Requisitos: [Node.js](https://nodejs.org/) 18+, [Rust](https://rustup.rs/) y, en
Windows, *Visual Studio Build Tools* con "Desktop development with C++".

```bash
npm install            # dependencias (una vez)
npm run tauri dev      # modo desarrollo
npm run tauri build    # instaladores en src-tauri/target/release/bundle/
```

Tests de la lógica de limpieza, de los proveedores y del par de idiomas:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

## Estructura

```
src/                 Interfaz (React + Tailwind)
src-tauri/src/
  lib.rs             Atajos globales, ajustes, orquestación de la traducción
  providers.rs       DeepL, Gemini, OpenAI, Anthropic, Google sin clave
  sanitize.rs        Limpieza de puntuación
  ollama.rs          Detectar / instalar Ollama y descargar el modelo
branding/            Logo (SVG) e iconos de origen
.github/workflows/   Compilación multiplataforma y publicación de Releases
```

## Licencia

El código se publica bajo [MIT](LICENSE): puedes usarlo, modificarlo y
distribuirlo libremente.

El nombre **"Verbak"**, el logotipo y el icono **no** entran en esa licencia:
ver [NOTICE.md](NOTICE.md). Un fork puede reutilizar el código, pero con su
propio nombre e identidad.

## ¿Necesitas una herramienta así para tu equipo?

Construyo herramientas internas y de escritorio: automatización de flujos
repetitivos, integraciones con modelos de IA, apps nativas con Tauri/Rust. Si tu
equipo tiene un proceso manual que se podría quitar de en medio, escríbeme por
**[LinkedIn](https://www.linkedin.com/in/arieldelgue)**.

## Autor

Creado por **Ariel Delgue** · [LinkedIn](https://www.linkedin.com/in/arieldelgue)
