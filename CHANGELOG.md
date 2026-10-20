# Changelog

## 1.0.9

- **No traducir el texto entre comillas** (Ajustes → Traducción). Lo que va
  entre "comillas", “comillas” o «comillas» se quita del texto antes de
  mandarlo al motor y se vuelve a poner tal cual después, así que ningún motor
  puede traducirlo. Si el motor pierde una de esas frases, Verbak avisa.

## 1.0.8

- **Traducciones que suenan humanas.** El prompt de Gemini, OpenAI, Claude y
  Ollama ahora pide mantener la voz, el ritmo y el registro del original, y
  evitar el vocabulario y los conectores típicos de texto generado por IA. La
  temperatura por defecto sube de 0.2 a 0.7. Hay una versión del prompt para usar en la web
  de Gemini en `docs/prompt-traduccion-natural.md`.
- **macOS: ya no aparece una "c" o una "v" suelta al reemplazar.** El Cmd+C/V
  sintético se manda con CoreGraphics marcando Cmd en cada evento, y se espera
  a que sueltes las teclas del atajo antes de enviarlo.
- Si la copia de la selección falla, Verbak avisa en vez de traducir y pegar lo
  que hubiera antes en el portapapeles.
- **Pestaña "Naturalidad"** en Ajustes para ajustar y probar el tono de los
  motores con IA: estilo del prompt (Natural, Literal o Personalizado),
  temperatura, top-p, ejemplos de cómo escribes, lista de palabras prohibidas,
  segunda pasada de revisión y "pensar" en Gemini 2.5 Flash. Incluye una zona
  de pruebas que traduce con los ajustes sin guardar, muestra el prompt exacto
  y guarda los últimos resultados para comparar.

## 1.0.0

Primera versión pública.

- Traducción de la selección en el sitio con atajos globales
  (`Ctrl+Alt+D` / `F` / `S`).
- Motores: DeepL, Google Gemini, OpenAI y compatibles (Groq, OpenRouter,
  DeepSeek, Mistral), Anthropic, Ollama local y **Google Traducción sin clave**.
- **Asistente de primer uso** que ayuda a elegir motor y, para Ollama, lo
  instala y descarga el modelo sin usar la terminal.
- **Par de idiomas configurable** (ES, EN, PT, FR, DE, IT, NL, PL, RU, JA, ZH,
  KO) con detección automática de la dirección.
- Limpieza de la puntuación "de traductor" (guiones largos, punto y coma,
  comillas tipográficas), configurable.
- Fallback automático entre perfiles de API.
- Interfaz clara, arranque con el sistema, ventana translúcida.
- Las claves de API se guardan solo en el equipo del usuario, nunca en el
  ejecutable.
