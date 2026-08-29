# Changelog

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
