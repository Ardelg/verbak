/*
 * Limpieza de puntuación "de traductor".
 *
 * DeepL y los LLM devuelven em-dashes (—), punto y coma, comillas tipográficas
 * y puntos suspensivos de un solo carácter. Nada de eso es habitual al escribir
 * en Slack o en un informe interno, así que se reemplaza por su equivalente
 * plano después de recibir la traducción.
 *
 * El texto dentro de `código`, ```bloques``` y las URLs no se toca.
 */

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_semicolon_replacement() -> String {
    ",".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomRule {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanitizeSettings {
    /// Interruptor general.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Guiones largos (— – ―) -> coma, viñeta o nada según la posición.
    #[serde(default = "default_true")]
    pub dashes: bool,
    #[serde(default = "default_true")]
    pub semicolons: bool,
    /// "," | "." | ";" (";" = no tocar)
    #[serde(default = "default_semicolon_replacement")]
    pub semicolon_replacement: String,
    /// Comillas tipográficas -> comillas rectas.
    #[serde(default = "default_true")]
    pub quotes: bool,
    /// … -> ...
    #[serde(default = "default_true")]
    pub ellipsis: bool,
    /// Espacios raros (nbsp, finos) y espacios dobles.
    #[serde(default = "default_true")]
    pub spaces: bool,
    /// Viñetas • ▪ ‣ -> "-"
    #[serde(default = "default_true")]
    pub bullets: bool,
    /// No tocar `código` ni URLs.
    #[serde(default = "default_true")]
    pub preserve_code: bool,
    #[serde(default)]
    pub custom: Vec<CustomRule>,
}

impl Default for SanitizeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            dashes: true,
            semicolons: true,
            semicolon_replacement: default_semicolon_replacement(),
            quotes: true,
            ellipsis: true,
            spaces: true,
            bullets: true,
            preserve_code: true,
            custom: Vec::new(),
        }
    }
}

pub fn sanitize(input: &str, settings: &SanitizeSettings) -> String {
    if !settings.enabled || input.is_empty() {
        return input.to_string();
    }

    let segments = split_protected(input, settings.preserve_code);
    let last_editable = segments
        .iter()
        .rposition(|(_, protected)| !protected)
        .unwrap_or(0);

    let mut out = String::with_capacity(input.len());
    for (idx, (segment, protected)) in segments.iter().enumerate() {
        if *protected {
            out.push_str(segment);
        } else {
            let replaced = replace_chars(segment, settings, idx == last_editable);
            out.push_str(&polish(&replaced, settings));
        }
    }
    out
}

/* ------------------------------------------------------------------ */
/* Segmentación: separa lo que no se debe tocar                        */
/* ------------------------------------------------------------------ */

fn split_protected(input: &str, preserve_code: bool) -> Vec<(String, bool)> {
    let chars: Vec<char> = input.chars().collect();
    let mut segments: Vec<(String, bool)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < chars.len() {
        if preserve_code && chars[i] == '`' {
            let mut run = 0;
            while i + run < chars.len() && chars[i + run] == '`' {
                run += 1;
            }
            if let Some(close) = find_backtick_run(&chars, i + run, run) {
                if !buf.is_empty() {
                    segments.push((std::mem::take(&mut buf), false));
                }
                segments.push((chars[i..close + run].iter().collect(), true));
                i = close + run;
                continue;
            }
            // Sin cierre: es un backtick suelto, se trata como texto normal.
            for _ in 0..run {
                buf.push('`');
            }
            i += run;
            continue;
        }

        if preserve_code && is_url_start(&chars, i) {
            let mut j = i;
            while j < chars.len() && !chars[j].is_whitespace() {
                j += 1;
            }
            if !buf.is_empty() {
                segments.push((std::mem::take(&mut buf), false));
            }
            segments.push((chars[i..j].iter().collect(), true));
            i = j;
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        segments.push((buf, false));
    }
    if segments.is_empty() {
        segments.push((String::new(), false));
    }
    segments
}

fn find_backtick_run(chars: &[char], from: usize, len: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '`' {
            let mut run = 0;
            while i + run < chars.len() && chars[i + run] == '`' {
                run += 1;
            }
            if run >= len {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

fn is_url_start(chars: &[char], i: usize) -> bool {
    let boundary = i == 0 || matches!(chars[i - 1], ' ' | '\n' | '\t' | '(' | '<' | '[');
    if !boundary {
        return false;
    }
    let rest: String = chars[i..chars.len().min(i + 8)].iter().collect();
    rest.starts_with("http://") || rest.starts_with("https://") || rest.starts_with("www.")
}

/* ------------------------------------------------------------------ */
/* Reemplazo de caracteres                                             */
/* ------------------------------------------------------------------ */

fn replace_chars(segment: &str, s: &SanitizeSettings, is_last: bool) -> String {
    let mut working = segment.to_string();
    for rule in &s.custom {
        if !rule.from.is_empty() {
            working = working.replace(&rule.from, &rule.to);
        }
    }

    let chars: Vec<char> = working.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Guiones largos: — – ― ‒
        if s.dashes && matches!(c, '\u{2014}' | '\u{2013}' | '\u{2015}' | '\u{2012}') {
            let prev = out.iter().rev().find(|ch| !ch.is_whitespace()).copied();
            let next = chars.get(i + 1).copied();

            // Rango numérico pegado: 2020–2024 -> 2020-2024
            let glued = out.last().map_or(false, |l| !l.is_whitespace());
            if glued
                && prev.map_or(false, |p| p.is_ascii_digit())
                && next.map_or(false, |n| n.is_ascii_digit())
            {
                out.push('-');
                i += 1;
                continue;
            }

            while out.last().map_or(false, |l| *l == ' ' || *l == '\u{a0}') {
                out.pop();
            }
            let mut j = i + 1;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\u{a0}') {
                j += 1;
            }

            let at_line_start = out.is_empty() || out.last() == Some(&'\n');
            let at_line_end = (j >= chars.len() && is_last) || chars.get(j) == Some(&'\n');

            if at_line_start {
                // Viñeta o diálogo al principio de línea.
                out.push('-');
                out.push(' ');
            } else if at_line_end {
                // Guion colgando al final: se descarta.
            } else {
                if out
                    .last()
                    .map_or(false, |l| !",.;:!?¡¿(«\"'".contains(*l))
                {
                    out.push(',');
                }
                out.push(' ');
            }
            i = j;
            continue;
        }

        // Punto y coma
        if s.semicolons && c == ';' && s.semicolon_replacement != ";" {
            if ends_with_html_entity(&out) {
                out.push(';');
                i += 1;
                continue;
            }
            while out.last() == Some(&' ') {
                out.pop();
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let rest_on_line = j < chars.len() && chars[j] != '\n';

            if s.semicolon_replacement == "." {
                out.push('.');
                if rest_on_line {
                    out.push(' ');
                    if chars[j].is_alphabetic() {
                        for u in chars[j].to_uppercase() {
                            out.push(u);
                        }
                        i = j + 1;
                        continue;
                    }
                }
            } else {
                out.push(',');
                if rest_on_line {
                    out.push(' ');
                }
            }
            i = j;
            continue;
        }

        // Comillas tipográficas y apóstrofos
        if s.quotes {
            match c {
                '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{00ab}' | '\u{00bb}' => {
                    out.push('"');
                    i += 1;
                    continue;
                }
                '\u{2018}' | '\u{2019}' | '\u{201a}' => {
                    out.push('\'');
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }

        if s.ellipsis && c == '\u{2026}' {
            out.push('.');
            out.push('.');
            out.push('.');
            i += 1;
            continue;
        }

        if s.spaces && matches!(c, '\u{a0}' | '\u{2009}' | '\u{202f}' | '\u{2007}' | '\u{200a}') {
            out.push(' ');
            i += 1;
            continue;
        }

        if s.bullets && matches!(c, '\u{2022}' | '\u{25aa}' | '\u{2023}' | '\u{25e6}') {
            out.push('-');
            i += 1;
            continue;
        }

        out.push(c);
        i += 1;
    }

    out.into_iter().collect()
}

fn ends_with_html_entity(out: &[char]) -> bool {
    // &nbsp  &amp  &#39 ...
    let mut k = out.len();
    let mut seen = 0;
    while k > 0 && seen < 8 {
        k -= 1;
        seen += 1;
        let c = out[k];
        if c == '&' {
            return seen > 1;
        }
        if !(c.is_ascii_alphanumeric() || c == '#') {
            return false;
        }
    }
    false
}

/* ------------------------------------------------------------------ */
/* Pulido de espacios y puntuación repetida                            */
/* ------------------------------------------------------------------ */

/// ¿Todo lo escrito desde el último salto de línea son espacios? (sangría)
fn in_leading_whitespace(out: &[char]) -> bool {
    for &c in out.iter().rev() {
        if c == '\n' {
            return true;
        }
        if c != ' ' && c != '\t' {
            return false;
        }
    }
    true
}

fn polish(segment: &str, s: &SanitizeSettings) -> String {
    let chars: Vec<char> = segment.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());

    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' && s.spaces {
            // La sangría de listas y citas se respeta; los dobles espacios
            // dentro de la línea no.
            if !in_leading_whitespace(&out) && out.last() == Some(&' ') {
                continue;
            }
            out.push(' ');
            continue;
        }

        if matches!(c, ',' | '.' | ':' | '!' | '?' | ')') {
            while out.last() == Some(&' ') && out.len() > 1 && out[out.len() - 2] != '\n' {
                out.pop();
            }
        }

        if c == ',' {
            let prev = out.iter().rev().find(|ch| !ch.is_whitespace()).copied();
            if matches!(prev, Some(',') | Some('.') | Some(':') | Some('!') | Some('?')) {
                continue;
            }
            // ", y" / ", and" duplicado tras el reemplazo de guiones no se toca:
            // solo evitamos comas pegadas.
        }

        // Espacio colgando al final de línea.
        if c == '\n' {
            while out.last() == Some(&' ') {
                out.pop();
            }
        }

        let _ = i;
        out.push(c);
    }

    out.into_iter().collect()
}

/* ------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> SanitizeSettings {
        SanitizeSettings::default()
    }

    #[test]
    fn em_dash_becomes_comma() {
        assert_eq!(
            sanitize("The agent works — and it is fast", &s()),
            "The agent works, and it is fast"
        );
        assert_eq!(sanitize("foo—bar", &s()), "foo, bar");
    }

    #[test]
    fn em_dash_pair_reads_as_aside() {
        assert_eq!(
            sanitize("El deploy — que ya probamos — salió bien", &s()),
            "El deploy, que ya probamos, salió bien"
        );
    }

    #[test]
    fn dash_at_line_start_is_a_bullet() {
        assert_eq!(sanitize("— uno\n— dos", &s()), "- uno\n- dos");
    }

    #[test]
    fn dash_at_end_is_dropped() {
        assert_eq!(sanitize("ya lo vimos —", &s()), "ya lo vimos");
    }

    #[test]
    fn numeric_range_keeps_hyphen() {
        assert_eq!(sanitize("2020–2024", &s()), "2020-2024");
    }

    #[test]
    fn semicolon_becomes_comma_by_default() {
        assert_eq!(sanitize("uno; dos", &s()), "uno, dos");
    }

    #[test]
    fn semicolon_can_become_period_with_capital() {
        let mut cfg = s();
        cfg.semicolon_replacement = ".".into();
        assert_eq!(sanitize("uno; dos", &cfg), "uno. Dos");
    }

    #[test]
    fn html_entity_semicolon_survives() {
        assert_eq!(sanitize("Tom &amp; Jerry", &s()), "Tom &amp; Jerry");
    }

    #[test]
    fn quotes_and_ellipsis_are_flattened() {
        assert_eq!(
            sanitize("\u{201c}listo\u{201d}\u{2026} don\u{2019}t", &s()),
            "\"listo\"... don't"
        );
    }

    #[test]
    fn code_spans_are_untouched() {
        let input = "usa `a; b — c` y sigue; ok";
        assert_eq!(sanitize(input, &s()), "usa `a; b — c` y sigue, ok");
    }

    #[test]
    fn fenced_blocks_are_untouched() {
        let input = "mira:\n```js\nconst a = 1; // — nota\n```\nfin; listo";
        let out = sanitize(input, &s());
        assert!(out.contains("const a = 1; // — nota"));
        assert!(out.ends_with("fin, listo"));
    }

    #[test]
    fn urls_are_untouched() {
        let input = "abre https://x.com/a?b=1;c=2 ahora; gracias";
        assert_eq!(
            sanitize(input, &s()),
            "abre https://x.com/a?b=1;c=2 ahora, gracias"
        );
    }

    #[test]
    fn disabled_is_identity() {
        let mut cfg = s();
        cfg.enabled = false;
        assert_eq!(sanitize("a — b; c", &cfg), "a — b; c");
    }

    #[test]
    fn custom_rules_apply() {
        let mut cfg = s();
        cfg.custom.push(CustomRule {
            from: "asimismo".into(),
            to: "además".into(),
        });
        assert_eq!(sanitize("asimismo, sí", &cfg), "además, sí");
    }

    #[test]
    fn indentation_is_preserved() {
        assert_eq!(sanitize("- a\n  - b; c", &s()), "- a\n  - b, c");
    }

    #[test]
    fn double_spaces_collapse() {
        assert_eq!(sanitize("hola  mundo", &s()), "hola mundo");
    }
}
