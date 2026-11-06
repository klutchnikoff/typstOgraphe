use super::*;

#[test]
fn partial_conflicts_are_never_published_and_skip_rechecks_the_remaining_group() {
    let base = Library::new(vec![record(
        "base",
        "existing",
        json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020}),
    )])
    .unwrap();
    let candidates = vec![
        record(
            "a",
            "first",
            json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020,"serial-number":{"doi":"10.5555/one"}}),
        ),
        record(
            "b",
            "second",
            json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020,"serial-number":{"doi":"10.5555/two"}}),
        ),
        record(
            "c",
            "unrelated",
            json!({"type":"book","title":"Other work","author":"Roe, Richard"}),
        ),
    ];
    let plan = ImportPlan::prepare(&base, &candidates).unwrap();
    assert_eq!(
        plan.apply(&base).unwrap_err().code,
        Code::UnresolvedConflict
    );
    assert_eq!(base.records().len(), 1);
    let resolved = plan
        .resolve(&[Decision::Skip { incoming: id("b") }])
        .unwrap();
    assert!(plan.blocked());
    let result = resolved.apply(&base).unwrap();
    assert_eq!(result.records().len(), 2);
    assert_eq!(
        result.get(&id("base")).unwrap().entry().doi(),
        Some("10.5555/one")
    );
    assert!(result.get(&id("b")).is_none());
    assert!(result.get(&id("c")).is_some());
}

#[test]
fn stale_plan_or_invalid_decisions_do_not_modify_the_library() {
    let original = record(
        "base",
        "existing",
        json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020}),
    );
    let base = Library::new(vec![original.clone()]).unwrap();
    let candidates = vec![record(
        "a",
        "new",
        json!({"type":"book","title":"Unrelated work","author":"Other, Alice"}),
    )];
    let plan = ImportPlan::prepare(&base, &candidates).unwrap();
    let updated = original
        .edited(
            &original.fingerprint(),
            &[FieldPatch {
                path: "/note".into(),
                value: Some(json!("Concurrent edit")),
            }],
        )
        .unwrap();
    let changed = Library::new(vec![updated]).unwrap();
    assert_eq!(plan.apply(&changed).unwrap_err().code, Code::StaleBase);
    for decision in [
        Decision::Skip {
            incoming: id("absent"),
        },
        Decision::Same {
            incoming: id("a"),
            target: id("base"),
        },
        Decision::Same {
            incoming: id("a"),
            target: id("absent"),
        },
        Decision::Same {
            incoming: id("a"),
            target: id("a"),
        },
    ] {
        assert_eq!(
            plan.resolve(&[decision]).unwrap_err().code,
            Code::InvalidDecision
        );
    }
    assert_eq!(base.records(), &[original]);
}

#[test]
fn same_decisions_between_incoming_records_support_forward_targets_but_refuse_cycles() {
    let entries = vec![
        record(
            "a",
            "one",
            json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020,"serial-number":{"doi":"10.5555/one"}}),
        ),
        record(
            "b",
            "two",
            json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020,"serial-number":{"doi":"10.5555/two"}}),
        ),
    ];
    let base = Library::default();
    let plan = ImportPlan::prepare(&base, &entries).unwrap();
    let resolved = plan
        .resolve(&[
            Decision::Same {
                incoming: id("a"),
                target: id("b"),
            },
            Decision::Distinct {
                incoming: id("b"),
                key: "retained".into(),
            },
        ])
        .unwrap();
    let result = resolved.apply(&base).unwrap();
    assert_eq!(result.records().len(), 1);
    assert_eq!(result.get(&id("b")).unwrap().key(), "retained");
    assert_eq!(
        result.get(&id("b")).unwrap().entry().doi(),
        Some("10.5555/two")
    );
    assert_eq!(
        plan.resolve(&[
            Decision::Same {
                incoming: id("a"),
                target: id("b")
            },
            Decision::Same {
                incoming: id("b"),
                target: id("a")
            }
        ])
        .unwrap_err()
        .code,
        Code::InvalidDecision
    );
}

#[test]
fn reordered_candidates_keep_the_same_groups_and_composition() {
    let one = record(
        "a",
        "one",
        json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020}),
    );
    let two = record(
        "b",
        "two",
        json!({"type":"article","title":"A study","author":"Doe, Jane","date":2020,"volume":12}),
    );
    let three = record(
        "c",
        "three",
        json!({"type":"book","title":"Other work","author":"Roe, Richard"}),
    );
    let a = vec![one.clone(), two.clone(), three.clone()];
    let b = vec![three, two, one];
    let base = Library::default();
    assert_eq!(candidate_groups(&a).unwrap(), candidate_groups(&b).unwrap());
    assert_eq!(
        ImportPlan::prepare(&base, &a)
            .unwrap()
            .apply(&base)
            .unwrap(),
        ImportPlan::prepare(&base, &b)
            .unwrap()
            .apply(&base)
            .unwrap()
    );
}

#[test]
fn repeated_import_is_noop_even_when_source_and_record_ids_are_fresh() {
    let text = "k: {type: article, title: A study, author: 'Doe, Jane', date: 2020}";
    let prepared = import(text, ImportFormat::Hayagriva);
    let base = Library::default();
    let first = ImportPlan::prepare(&base, &prepared.records)
        .unwrap()
        .apply(&base)
        .unwrap();
    let source = ImportSource::new(
        id("different-source-id"),
        "renamed.yaml",
        ImportFormat::Hayagriva,
        text.into(),
    )
    .unwrap();
    let again = prepare_import(source, |_| id("fresh-record-id")).unwrap();
    let second = ImportPlan::prepare(&first, &again.records)
        .unwrap()
        .apply(&first)
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn identifiers_are_normalized_only_for_matching_not_for_storage() {
    assert_eq!(
        normalize_doi(" https://doi.org/10.5555/Example ").as_deref(),
        Some("10.5555/example")
    );
    for value in [
        "arXiv:2108.06507v2",
        "https://arxiv.org/pdf/2108.06507v3.pdf",
        "10.48550/arXiv.2108.06507",
    ] {
        assert_eq!(normalize_arxiv(value).as_deref(), Some("2108.06507"));
    }
    assert_eq!(
        normalize_arxiv("math.ST/0503083").as_deref(),
        Some("math.st/0503083")
    );
    assert!(normalize_arxiv("hal-01234567").is_none());
    assert!(normalize_arxiv("10.1214/aos/1176350258").is_none());
    let a = record(
        "a",
        "a",
        json!({"type":"article","serial-number":{"arxiv":"2108.06507"}}),
    );
    let b = record(
        "b",
        "b",
        json!({"type":"article","url":"https://arxiv.org/abs/2108.06507"}),
    );
    assert_eq!(evidence(&a, &b).contradictions, vec![]);
    assert_eq!(a.entry().arxiv(), Some("2108.06507"));
}
