use super::*;
use std::collections::BTreeSet;

fn run(name: &str) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/bibliography/identity-cases.json"
    ))
    .unwrap();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == name)
        .unwrap();
    let existing: Vec<Record> = serde_json::from_value(case["existing"].clone()).unwrap();
    let incoming: Vec<Record> = serde_json::from_value(case["incoming"].clone()).unwrap();
    let library = Library::new(existing.clone()).unwrap();
    let mut plan = ImportPlan::prepare(&library, &incoming).unwrap();
    if let Some(decision) = case.get("decision") {
        let incoming = id(decision["incoming"].as_str().unwrap());
        let choice = match decision["kind"].as_str().unwrap() {
            "same" => Decision::Same {
                incoming,
                target: id(decision["target"].as_str().unwrap()),
            },
            "distinct" => Decision::Distinct {
                incoming,
                key: decision["key"].as_str().unwrap().into(),
            },
            "skip" => Decision::Skip { incoming },
            _ => panic!("Unknown fixture decision"),
        };
        plan = plan.resolve(&[choice]).unwrap();
    }
    let all = existing
        .iter()
        .cloned()
        .chain(incoming.iter().cloned())
        .collect::<Vec<_>>();
    let groups = candidate_groups(&all).unwrap();
    let result = plan.apply(&library);
    for (assertion, expected) in case["expected"].as_object().unwrap() {
        match assertion.as_str() {
            "action" => {
                let action = serde_json::to_value(plan.items()[0].action).unwrap();
                assert_eq!(&action, expected, "{name}");
                assert!(
                    plan.items()
                        .iter()
                        .all(|i| serde_json::to_value(i.action).unwrap() == *expected)
                );
            }
            "target" => assert_eq!(
                plan.items()[0].target.as_ref().unwrap().as_str(),
                expected.as_str().unwrap()
            ),
            "keep_key" => assert_eq!(
                result.as_ref().unwrap().get(&id("r1")).unwrap().key(),
                expected.as_str().unwrap()
            ),
            "added_fields" => {
                for field in expected.as_array().unwrap() {
                    assert!(
                        plan.items().iter().any(|i| i
                            .added_fields
                            .contains(&format!("/{}", field.as_str().unwrap()))),
                        "{name}: {field}"
                    );
                }
            }
            "preserve_fields" if expected.is_object() => {
                let result = result.as_ref().unwrap();
                let before = &existing[0];
                let after = result.get(before.id()).unwrap();
                for field in expected.as_object().unwrap().keys() {
                    assert_eq!(
                        before.native_value().get(field),
                        after.native_value().get(field)
                    );
                }
            }
            "preserve_fields" => {
                for field in expected.as_array().unwrap() {
                    assert!(
                        result
                            .as_ref()
                            .unwrap()
                            .records()
                            .iter()
                            .any(|r| r.native_value().get(field.as_str().unwrap()).is_some())
                    );
                }
            }
            "replacement_requires_decision" => {
                assert_eq!(expected, &json!(true));
                assert!(!plan.items()[0].differing_fields.is_empty());
                let after = result.as_ref().unwrap().get(existing[0].id()).unwrap();
                assert_eq!(after.entry().publisher(), existing[0].entry().publisher());
            }
            "new_count" => assert_eq!(
                plan.items()
                    .iter()
                    .filter(|i| i.action == Action::New)
                    .count(),
                expected.as_u64().unwrap() as usize
            ),
            "distinct_keys" => {
                let records = result.as_ref().unwrap().records();
                assert_eq!(
                    records
                        .iter()
                        .map(Record::key)
                        .collect::<BTreeSet<_>>()
                        .len(),
                    records.len()
                );
            }
            "identity_conflict" => {
                assert_eq!(expected, &json!(false));
                assert!(!plan.blocked());
            }
            "rename_existing" | "existing_unchanged" | "other_record_unchanged" => {
                let after = result.as_ref().ok();
                if assertion == "other_record_unchanged" {
                    let identity = id(expected.as_str().unwrap());
                    assert_eq!(after.unwrap().get(&identity), library.get(&identity));
                } else if assertion == "existing_unchanged" {
                    for original in &existing {
                        assert_eq!(after.unwrap().get(original.id()), Some(original));
                    }
                } else {
                    assert_eq!(expected, &json!(false));
                    for original in &existing {
                        assert_eq!(library.get(original.id()).unwrap().key(), original.key());
                    }
                }
            }
            "key_choice_required" => {
                assert!(plan.blocked());
                assert!(
                    plan.items()
                        .iter()
                        .any(|i| i.conflicts.contains(&ConflictReason::KeyCollision))
                );
            }
            "identity_from_key" => {
                assert_eq!(expected, &json!(false));
                let mut renamed = incoming[0].clone();
                renamed = renamed
                    .renamed(&renamed.fingerprint(), "totally-different-key")
                    .unwrap();
                assert_eq!(
                    evidence(&existing[0], &incoming[0]),
                    evidence(&existing[0], &renamed)
                );
            }
            "reason" => {
                let reasons = all
                    .iter()
                    .enumerate()
                    .flat_map(|(i, one)| {
                        all[i + 1..]
                            .iter()
                            .flat_map(move |other| evidence(one, other).contradictions)
                    })
                    .collect::<Vec<_>>();
                assert!(
                    reasons
                        .into_iter()
                        .any(|r| serde_json::to_value(r).unwrap() == *expected),
                    "{name}"
                );
            }
            "publish_before_decision" | "automatic_identifier_overwrite" => {
                assert_eq!(expected, &json!(false));
                assert!(plan.blocked());
                assert_eq!(result.as_ref().unwrap_err().code, Code::UnresolvedConflict);
            }
            "merge" | "automatic_merge" => {
                assert_eq!(expected, &json!(false));
                assert!(plan.items().iter().all(|i| i.action == Action::New));
                assert_eq!(
                    result.as_ref().unwrap().records().len(),
                    existing.len() + incoming.len()
                );
            }
            "candidate_target" => assert!(
                groups
                    .iter()
                    .any(|g| g.members.contains(&incoming[0].id().clone())
                        && g.members.contains(&id(expected.as_str().unwrap())))
            ),
            "identifier_contradiction" => {
                assert_eq!(expected, &json!(false));
                assert!(
                    !evidence(&existing[0], &incoming[0])
                        .contradictions
                        .contains(&ConflictReason::ContradictoryIdentifiers)
                );
            }
            "reason_to_reject_from_subtitle_alone"
            | "reason_to_reject_from_subset_alone"
            | "missing_title_is_not_contradiction" => {
                assert!(groups.iter().all(|g| g.conflicts.is_empty()))
            }
            "replace_existing_title" | "preserve_existing_title" => assert_eq!(
                result
                    .as_ref()
                    .unwrap()
                    .get(existing[0].id())
                    .unwrap()
                    .entry()
                    .title(),
                existing[0].entry().title()
            ),
            "preserve_complete_authorship" => assert_eq!(
                result
                    .as_ref()
                    .unwrap()
                    .get(existing[0].id())
                    .unwrap()
                    .entry()
                    .authors(),
                existing[0].entry().authors()
            ),
            "record_storable" => {
                assert_eq!(expected, &json!(true));
                assert!(Library::new(incoming.clone()).is_ok());
            }
            "completeness_warning" => {
                assert_eq!(expected, &json!(true));
                assert!(!completeness(&incoming[0]).is_empty());
            }
            "whole_group_coherence_required" => assert!(
                groups
                    .iter()
                    .any(|g| g.members.len() == 3 && !g.conflicts.is_empty())
            ),
            "automatic_merge_all" => {
                assert_eq!(expected, &json!(false));
                assert!(plan.blocked());
            }
            "decision_required" => {
                assert_eq!(expected, &json!(true));
                assert!(plan.blocked());
            }
            "preserve_publication_states" => {
                assert!(plan.blocked());
                assert_ne!(existing[0].profile(), incoming[0].profile());
            }
            "repeat_import_no_change" => {
                let confirmed = result.as_ref().unwrap();
                let repeated = incoming
                    .iter()
                    .map(|r| {
                        let mut value = serde_json::to_value(r).unwrap();
                        value["id"] = json!(format!("repeat-{}", r.id().as_str()));
                        serde_json::from_value(value).unwrap()
                    })
                    .collect::<Vec<Record>>();
                assert_eq!(
                    &ImportPlan::prepare(confirmed, &repeated)
                        .unwrap()
                        .apply(confirmed)
                        .unwrap(),
                    confirmed
                );
            }
            "new_key" => assert!(
                result
                    .as_ref()
                    .unwrap()
                    .records()
                    .iter()
                    .any(|r| r.key() == expected.as_str().unwrap())
            ),
            "preserve_versions" => {
                let editions = result
                    .as_ref()
                    .unwrap()
                    .records()
                    .iter()
                    .map(|r| r.native_value()["edition"].clone())
                    .collect::<Vec<_>>();
                assert_eq!(editions, *expected.as_array().unwrap());
            }
            _ => panic!("Unimplemented corpus assertion: {assertion}"),
        }
    }
    assert_eq!(library.records(), existing);
}

macro_rules! cases { ($($case:ident),* $(,)?) => { $(#[test] fn $case() { run(&stringify!($case).replace('_',"-")); })* }; }
cases!(
    doi_case_and_key_independent,
    missing_fields_enrichment,
    known_value_not_overwritten,
    distinct_works_same_derived_key,
    existing_key_collision,
    same_title_two_dois,
    same_doi_different_title,
    different_dois_distinct_description,
    two_arxiv_identifiers,
    missing_doi_is_not_contradiction,
    subtitle_prefix,
    truncated_authorship,
    untitled_unidentified_not_evidence,
    untitled_shared_doi,
    transitive_group_not_proof,
    preprint_published_shared_arxiv,
    arxiv_overlapping_two_dois,
    incoming_variants_meet,
    incoming_enrichments_contradict,
    resolved_same_explicit_target,
    resolved_distinct_new_key,
    resolved_skip_no_mutation,
    software_versions_distinct,
    software_identical_version_candidate
);
