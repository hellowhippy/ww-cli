use serde_json::{json, Value};

/// Rewrites `operation.sample` values so they match workflow step output shape.
///
/// Authors write flat Zapier-style samples (a single object). At build time we always wrap:
/// - creates / hydrators → `{ "data": <sample> }`
/// - searches → `{ "data": [<sample>] }`
/// - triggers → unchanged (raw webhook payload for `sample_input`)
///
/// Do not pre-wrap samples in source — build always applies the envelope.
pub fn normalize_definition_samples(definition: &mut Value) {
    let Some(root) = definition.as_object_mut() else {
        return;
    };

    normalize_action_section(root.get_mut("creates"), SampleWrap::Object);
    normalize_action_section(root.get_mut("searches"), SampleWrap::Array);
    normalize_action_section(root.get_mut("hydrators"), SampleWrap::Object);
}

#[derive(Clone, Copy)]
enum SampleWrap {
    Object,
    Array,
}

fn normalize_action_section(section: Option<&mut Value>, wrap: SampleWrap) {
    let Some(actions) = section.and_then(|value| value.as_object_mut()) else {
        return;
    };

    for action in actions.values_mut() {
        let Some(operation) = action.get_mut("operation") else {
            continue;
        };
        normalize_operation_sample(operation, wrap);
    }
}

fn normalize_operation_sample(operation: &mut Value, wrap: SampleWrap) {
    let Some(op) = operation.as_object_mut() else {
        return;
    };

    let Some(sample) = op.remove("sample") else {
        return;
    };

    op.insert("sample".to_string(), normalize_sample_value(sample, wrap));
}

fn normalize_sample_value(sample: Value, wrap: SampleWrap) -> Value {
    match wrap {
        SampleWrap::Object => json!({ "data": sample }),
        SampleWrap::Array => json!({ "data": [sample] }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_create_sample_to_data_object() {
        let mut def = json!({
            "creates": {
                "create_contact": {
                    "operation": {
                        "sample": { "id": "123", "name": "Jane" }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["creates"]["create_contact"]["operation"]["sample"],
            json!({ "data": { "id": "123", "name": "Jane" } })
        );
    }

    #[test]
    fn normalizes_search_sample_to_data_array() {
        let mut def = json!({
            "searches": {
                "list_channels": {
                    "operation": {
                        "sample": { "id": "C1", "name": "#general" }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["searches"]["list_channels"]["operation"]["sample"],
            json!({ "data": [{ "id": "C1", "name": "#general" }] })
        );
    }

    #[test]
    fn leaves_trigger_sample_unchanged() {
        let mut def = json!({
            "triggers": {
                "new_contact": {
                    "operation": {
                        "sample": { "id": "evt-1", "type": "contact.created" }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["triggers"]["new_contact"]["operation"]["sample"],
            json!({ "id": "evt-1", "type": "contact.created" })
        );
    }

    #[test]
    fn wraps_create_sample_when_data_is_a_scalar_field() {
        let mut def = json!({
            "creates": {
                "echo": {
                    "operation": {
                        "sample": { "data": "hello" }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["creates"]["echo"]["operation"]["sample"],
            json!({ "data": { "data": "hello" } })
        );
    }

    #[test]
    fn wraps_search_sample_when_data_is_an_object_not_array() {
        let mut def = json!({
            "searches": {
                "find_item": {
                    "operation": {
                        "sample": { "data": { "id": "123" } }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["searches"]["find_item"]["operation"]["sample"],
            json!({ "data": [{ "data": { "id": "123" } }] })
        );
    }

    #[test]
    fn wraps_pre_wrapped_source_samples() {
        let mut def = json!({
            "creates": {
                "create_contact": {
                    "operation": {
                        "sample": { "data": { "id": "123" } }
                    }
                }
            },
            "searches": {
                "list_channels": {
                    "operation": {
                        "sample": { "data": [{ "id": "C1" }] }
                    }
                }
            }
        });

        normalize_definition_samples(&mut def);

        assert_eq!(
            def["creates"]["create_contact"]["operation"]["sample"],
            json!({ "data": { "data": { "id": "123" } } })
        );
        assert_eq!(
            def["searches"]["list_channels"]["operation"]["sample"],
            json!({ "data": [{ "data": [{ "id": "C1" }] }] })
        );
    }
}
