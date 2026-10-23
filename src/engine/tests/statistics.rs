use super::*;
use crate::statistics::TextStatistics;

#[test]
fn statistics_count_published_unicode_text_and_guarded_targets_without_side_effects() {
    let mut e = engine(
        "= Été\n\n#quote(block: true, attribution: [Élodie])[é 👩🏽‍💻]\n\n/ Terme: Définition\n\n#figure(image(\"images/parc.svg\", alt: \"Ignore this\"), caption: [Le parc.])",
    );
    let guard = e.target_guard();
    let source = e.document().source().to_owned();
    let selection = e.selection();
    let current = explicit(&e, 0..3);
    let result: StatisticsResult = serde_json::from_value(
        e.query(Query::Statistics(StatisticsParams {
            target: Some(current),
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result.guard, guard);
    assert_eq!(
        result.document,
        TextStatistics::count(&e.document().view.text)
    );
    assert_eq!(result.target, Some(TextStatistics::count("Été")));
    let no_target = e
        .query_call(Call {
            id: "document.statistics".to_owned().try_into().unwrap(),
            params: json!({}),
        })
        .unwrap();
    assert!(no_target["target"].is_null());
    assert_eq!(
        e.statistics(StatisticsParams {
            target: Some(TextTarget::All {})
        })
        .unwrap()
        .target,
        Some(result.document)
    );
    assert_eq!(
        e.statistics(StatisticsParams {
            target: Some(explicit(&e, std::ops::Range { start: 3, end: 0 }))
        })
        .unwrap()
        .target,
        result.target
    );
    assert_eq!(e.target_guard(), guard);
    assert_eq!(e.selection(), selection);
    assert_eq!(e.document().source(), source);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
    call(&mut e, "text.insert", json!({"text":"Suite "})).unwrap();
    assert_eq!(
        e.statistics(StatisticsParams {
            target: Some(current)
        })
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    assert_eq!(
        e.statistics(StatisticsParams::default())
            .unwrap()
            .document
            .words,
        result.document.words + 1
    );
}

#[test]
fn statistics_reject_foreign_and_closed_targets_and_recount_on_document_replacement() {
    let mut e = engine("Un texte.");
    let other = engine("Autre.");
    assert_eq!(
        e.statistics(StatisticsParams {
            target: Some(explicit(&other, 0..1))
        })
        .unwrap_err()
        .reason(),
        "foreign_document"
    );
    e.statistics(StatisticsParams::default()).unwrap();
    // Même révision : l’identité de document fait partie de la clé du cache.
    e.session = engine("Deux nouveaux mots.").session;
    assert_eq!(
        e.statistics(StatisticsParams::default())
            .unwrap()
            .document
            .words,
        3
    );
    e.open = false;
    assert_eq!(
        e.statistics(StatisticsParams::default())
            .unwrap_err()
            .reason(),
        "no_document"
    );
}
