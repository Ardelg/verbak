/*
 * Texto entre comillas que no se debe traducir.
 *
 * Pedírselo al modelo en el prompt no basta: con textos largos a veces lo
 * traduce igual. Aquí se quita del texto antes de mandarlo: cada frase entre
 * comillas se cambia por un marcador {{Q1}}, {{Q2}}... (los prompts ya piden
 * dejar los placeholders intactos) y al volver se pone el original en su lugar.
 */

/// Pares de comillas reconocidos. Las simples (' ') no: chocan con los
/// apóstrofos ("don't", "it's").
const PAIRS: [(char, char); 4] = [('"', '"'), ('\u{201C}', '\u{201D}'), ('\u{00AB}', '\u{00BB}'), ('\u{201E}', '\u{201C}')];

/// Una cita más larga que esto probablemente es un par de comillas mal cerrado.
const MAX_QUOTE_CHARS: usize = 2000;

pub struct Protected {
    pub text: String,
    /// (marcador, texto original con sus comillas)
    pub segments: Vec<(String, String)>,
}

fn closing_for(c: char) -> Option<char> {
    PAIRS.iter().find(|(open, _)| *open == c).map(|(_, close)| *close)
}

/// Cambia cada frase entre comillas por un marcador.
pub fn protect_quotes(text: &str) -> Protected {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut segments = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if let Some(close) = closing_for(c) {
            let end = chars[i + 1..]
                .iter()
                .take(MAX_QUOTE_CHARS)
                .position(|&x| x == close)
                .map(|p| i + 1 + p);
            if let Some(end) = end {
                let inner: String = chars[i + 1..end].iter().collect();
                if !inner.trim().is_empty() {
                    let marker = format!("{{{{Q{}}}}}", segments.len() + 1);
                    let original: String = chars[i..=end].iter().collect();
                    out.push_str(&marker);
                    segments.push((marker, original));
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }

    Protected { text: out, segments }
}

/// Vuelve a poner las frases originales. Devuelve el texto y los marcadores
/// que el modelo perdió por el camino.
pub fn restore_quotes(translated: &str, segments: &[(String, String)]) -> (String, Vec<String>) {
    let mut out = translated.to_string();
    let mut missing = Vec::new();
    for (marker, original) in segments {
        let id = &marker[2..marker.len() - 2]; // "Q1"
        let variants = [
            marker.clone(),
            format!("{{{{ {} }}}}", id),
            format!("{{{}}}", id),
        ];
        // Si el modelo dejó comillas alrededor del marcador, no duplicarlas.
        let mut found = false;
        for v in variants.iter() {
            for (open, close) in PAIRS.iter() {
                let wrapped = format!("{}{}{}", open, v, close);
                if out.contains(&wrapped) {
                    out = out.replace(&wrapped, original);
                    found = true;
                }
            }
            if out.contains(v.as_str()) {
                out = out.replace(v.as_str(), original);
                found = true;
            }
            if found {
                break;
            }
        }
        if !found {
            missing.push(original.clone());
        }
    }
    (out, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_are_swapped_and_restored() {
        let text = "La Respuesta A dice \u{201C}the model is correct\u{201D} y (\"no rompe nada\").";
        let p = protect_quotes(text);
        assert_eq!(p.text, "La Respuesta A dice {{Q1}} y ({{Q2}}).");
        let (back, missing) = restore_quotes("Response A says {{Q1}} and ({{Q2}}).", &p.segments);
        assert_eq!(back, "Response A says \u{201C}the model is correct\u{201D} and (\"no rompe nada\").");
        assert!(missing.is_empty());
    }

    #[test]
    fn apostrophes_and_unclosed_quotes_are_left_alone() {
        let text = "It's fine, don't worry. Un \"suelto";
        let p = protect_quotes(text);
        assert_eq!(p.text, text);
        assert!(p.segments.is_empty());
    }

    #[test]
    fn tolerant_restore_and_missing_report() {
        let p = protect_quotes("Dice \"hola\" y \"chau\"");
        let (back, missing) = restore_quotes("Says \"{{Q1}}\" and {Q2}", &p.segments);
        assert_eq!(back, "Says \"hola\" and \"chau\"");
        assert!(missing.is_empty());
        let (_, missing) = restore_quotes("Says hi", &p.segments);
        assert_eq!(missing.len(), 2);
    }
}
