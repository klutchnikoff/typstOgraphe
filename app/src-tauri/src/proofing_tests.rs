use super::*;
fn check(segments: Vec<String>, language: DocumentLanguage, ignored: Vec<String>) -> Value {
    tauri::async_runtime::block_on(spelling_check(segments, language, ignored)).unwrap()
}
#[test]
fn issues_target_the_exact_utf16_word_after_emoji_and_combining_characters_in_each_segment() {
    let segments = vec![
        "😀 cafe\u{301} bonjouur".to_owned(),
        "日本 😀 bonjouur".to_owned(),
    ];
    let result = check(segments.clone(), DocumentLanguage::French, vec![]);
    let issues: Vec<_> = result
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["word"] == "bonjouur")
        .collect();
    assert_eq!(issues.len(), 2, "{result}");
    for (index, source) in segments.iter().enumerate() {
        let issue = issues.iter().find(|i| i["segment"] == index).unwrap();
        let encoded: Vec<_> = source.encode_utf16().collect();
        let from = issue["from"].as_u64().unwrap() as usize;
        let to = issue["to"].as_u64().unwrap() as usize;
        assert_eq!(String::from_utf16(&encoded[from..to]).unwrap(), "bonjouur");
        assert_eq!(
            from,
            source[..source.find("bonjouur").unwrap()]
                .encode_utf16()
                .count()
        );
    }
    let ignored = check(segments, DocumentLanguage::French, vec!["BONJOUUR".into()]);
    assert!(
        ignored
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["word"] != "bonjouur")
    );
}
#[test]
fn unsupported_language_and_empty_segments_return_no_issues_and_suggestions_correct_the_word() {
    let language = [
        DocumentLanguage::German,
        DocumentLanguage::Spanish,
        DocumentLanguage::Italian,
    ]
    .into_iter()
    .find(|l| !typstographe_core::spelling::supported(*l))
    .unwrap();
    assert_eq!(check(vec!["bonjouur".into()], language, vec![]), json!([]));
    assert_eq!(
        check(
            vec!["".into(), "Bonjour".into()],
            DocumentLanguage::French,
            vec![]
        ),
        json!([])
    );
    let suggestions = tauri::async_runtime::block_on(spelling_suggestions(
        DocumentLanguage::French,
        "bonjouur".into(),
    ))
    .unwrap();
    assert!(
        suggestions.iter().any(|s| s.to_lowercase() == "bonjour"),
        "{suggestions:?}"
    );
}
#[test]
fn oversized_spelling_requests_are_refused_before_background_work() {
    assert!(
        tauri::async_runtime::block_on(spelling_check(
            vec![String::new(); 20001],
            DocumentLanguage::French,
            vec![]
        ))
        .is_err()
    );
    assert!(
        tauri::async_runtime::block_on(spelling_check(
            vec!["x".repeat(2 * 1024 * 1024 + 1)],
            DocumentLanguage::French,
            vec![]
        ))
        .is_err()
    );
    assert!(
        tauri::async_runtime::block_on(spelling_suggestions(
            DocumentLanguage::French,
            "x".repeat(513)
        ))
        .is_err()
    );
}
