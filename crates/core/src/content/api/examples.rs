//! Executable documentary examples, independent of adapter envelopes and permissions.
use super::Call;
use serde_json::{Value, json};
fn paragraph(text: &str) -> Value {
    if text.is_empty() {
        json!({"type":"paragraph"})
    } else {
        json!({"type":"paragraph","content":[{"type":"text","text":text}]})
    }
}
fn at() -> Value {
    json!({"zone":{"kind":"body","path":[0]},"from":0,"to":0})
}
pub fn for_id(id: &str) -> Vec<Call> {
    let node = match id {
        "block.bullet" => {
            json!({"type":"bulletList","content":[{"type":"listItem","content":[paragraph("Item")]}]})
        }
        "block.numbered" => {
            json!({"type":"orderedList","attrs":{"start":1},"content":[{"type":"listItem","content":[paragraph("Item")]}]})
        }
        "block.description" => {
            json!({"type":"descriptionList","content":[{"type":"descriptionItem","content":[{"type":"descriptionTerm","content":[{"type":"text","text":"Term"}]},{"type":"descriptionDefinition","content":[{"type":"text","text":"Definition"}]}]}]})
        }
        "block.quote" => {
            json!({"type":"quote","content":[paragraph("Quoted text"),{"type":"quoteAttribution","content":[{"type":"text","text":"Author, source"}]}]})
        }
        "insert.footnote" => json!({"type":"footnote","attrs":{"text":"A plain-text note."}}),
        "insert.citation" => {
            json!({"type":"citation","attrs":{"items":[{"file_id":"main","key":"example2026"}],"form":"normal"}})
        }
        "insert.image" => {
            json!({"type":"figure","attrs":{"src":"images/example.svg","alt":"Describe the image"},"content":[{"type":"text","text":"Optional caption"}]})
        }
        "letter.insert_postscript" => {
            json!({"type":"postscript","content":[{"type":"text","text":"Additional information."}]})
        }
        "letter.insert_attachment" => {
            json!({"type":"attachment","content":[{"type":"text","text":"Signed form"}]})
        }
        "text.insert" => {
            return vec![Call {
                id: id.into(),
                params: json!({"target":at(),"text":"Literal prose","marks":[{"type":"bold"}]}),
            }];
        }
        "inline.replace" => {
            return vec![Call {
                id: id.into(),
                params: json!({"target":at(),"node":{"type":"softBreak"}}),
            }];
        }
        "list.item.split" | "list.item.join" | "list.item.exit" => {
            return vec![
                Call {
                    id: "block.insert".into(),
                    params: json!({"parent":{"root":{"kind":"body"}},"index":0,"node":{"type":"bulletList","content":[{"type":"listItem","content":[paragraph("Left right")]},{"type":"listItem","content":[paragraph("")]}]}}),
                },
                Call {
                    id: id.into(),
                    params: match id {
                        "list.item.split" => {
                            json!({"target":{"zone":{"kind":"body","path":[0,0,0]},"from":4,"to":5}})
                        }
                        "list.item.join" => {
                            json!({"left":{"root":{"kind":"body"},"path":[0,0]},"right":{"root":{"kind":"body"},"path":[0,1]}})
                        }
                        _ => json!({"target":{"root":{"kind":"body"},"path":[0,1]}}),
                    },
                },
            ];
        }
        "text.bold" | "text.italic" => {
            return vec![
                Call {
                    id: "text.insert".into(),
                    params: json!({"target":at(),"text":"Emphasized plain"}),
                },
                Call {
                    id: id.into(),
                    params: json!({"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":10},"value":"on"}),
                },
            ];
        }
        "insert.table" => {
            return vec![Call {
                id: id.into(),
                params: json!({"parent":{"root":{"kind":"body"}},"index":0,"rows":3,"columns":2,"header":true}),
            }];
        }
        "block.join" => {
            return vec![
                Call {
                    id: "block.insert".into(),
                    params: json!({"parent":{"root":{"kind":"body"}},"index":0,"node":paragraph("Left ")}),
                },
                Call {
                    id: "block.insert".into(),
                    params: json!({"parent":{"root":{"kind":"body"}},"index":1,"node":paragraph("right")}),
                },
                Call {
                    id: id.into(),
                    params: json!({"left":{"root":{"kind":"body"},"path":[0]},"right":{"root":{"kind":"body"},"path":[1]}}),
                },
            ];
        }
        _ => return vec![],
    };
    let inline = matches!(id, "insert.footnote" | "insert.citation");
    vec![Call {
        id: if inline {
            "inline.replace"
        } else {
            "block.insert"
        }
        .into(),
        params: if inline {
            json!({"target":at(),"node":node})
        } else {
            json!({"parent":{"root":{"kind":"body"}},"index":0,"node":node})
        },
    }]
}
