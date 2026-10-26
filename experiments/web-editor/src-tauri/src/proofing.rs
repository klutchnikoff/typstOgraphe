use serde_json::{Value, json};
use typstographe_core::{document::DocumentLanguage, statistics::TextStatistics};
#[tauri::command]
pub async fn spelling_check(
    segments: Vec<String>,
    language: DocumentLanguage,
    ignored: Vec<String>,
) -> Result<Value, String> {
    if segments.len() > 20000 || segments.iter().map(String::len).sum::<usize>() > 2 * 1024 * 1024 {
        return Err("Texte trop volumineux.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if !typstographe_core::spelling::supported(language) {
            return Ok(json!([]));
        }
        let accepted = ignored
            .iter()
            .map(|s| typstographe_core::spelling::word_key(s))
            .collect();
        let mut found = vec![];
        for (index, text) in segments.iter().enumerate() {
            for issue in typstographe_core::spelling::check_text(language, text, &accepted)
                .map_err(|e| e.to_string())?
            {
                let from: usize = text
                    .chars()
                    .take(issue.range.start)
                    .map(char::len_utf16)
                    .sum();
                let to: usize = text
                    .chars()
                    .take(issue.range.end)
                    .map(char::len_utf16)
                    .sum();
                found.push(json!({"segment":index,"from":from,"to":to,"word":issue.word}))
            }
        }
        Ok(json!(found))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn spelling_suggestions(
    language: DocumentLanguage,
    word: String,
) -> Result<Vec<String>, String> {
    if word.len() > 512 {
        return Err("Mot trop long.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        typstographe_core::spelling::suggestions(language, &word).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn count_text(text: String, selection: Option<String>) -> Value {
    let total = TextStatistics::count(&text);
    let part = selection.map(|s| TextStatistics::count(&s));
    let mut label =
        typstographe_i18n::tr!("document-stats", words = total.words, signs = total.signs);
    if let Some(selected) = &part {
        label.push_str(" | ");
        label.push_str(&typstographe_i18n::tr!(
            "selection-stats",
            words = selected.words,
            signs = selected.signs
        ));
    }
    let hint = typstographe_i18n::tr!(
        "statistics-hint",
        without_spaces = part.as_ref().unwrap_or(&total).signs_without_spaces
    );
    json!({"document":total,"selection":part,"label":label,"hint":hint})
}
