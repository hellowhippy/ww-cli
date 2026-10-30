use serde_json::Value;

const MIN_LABEL_LENGTH: usize = 3;
const VALID_INPUT_FIELD_TYPES: &[&str] =
    &["string", "text", "integer", "number", "boolean", "file", "json_schema"];
const VALID_AUTH_FIELD_TYPES: &[&str] = &["string", "password", "boolean"];

pub fn validate_definition_quality(definition: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(auth) = definition.get("authentication") {
        validate_authentication_quality(auth, &mut errors);
    }

    validate_actions_quality(definition.get("triggers"), "triggers", SampleShape::Trigger, &mut errors);
    validate_actions_quality(definition.get("creates"), "creates", SampleShape::DataObject, &mut errors);
    validate_actions_quality(definition.get("searches"), "searches", SampleShape::DataArray, &mut errors);
    validate_actions_quality(definition.get("hydrators"), "hydrators", SampleShape::DataObject, &mut errors);

    errors
}

fn validate_authentication_quality(auth: &Value, errors: &mut Vec<String>) {
    let Some(obj) = auth.as_object() else {
        return;
    };

    if obj.get("type").and_then(|v| v.as_str()) == Some("none") {
        return;
    }

    if let Some(fields) = auth.get("fields") {
        validate_input_field_list_quality(fields, "authentication.fields", true, errors);
    }
}

#[derive(Clone, Copy)]
enum SampleShape {
    Trigger,
    DataObject,
    DataArray,
}

fn validate_actions_quality(
    actions: Option<&Value>,
    path: &str,
    sample_shape: SampleShape,
    errors: &mut Vec<String>,
) {
    let Some(actions_obj) = actions.and_then(|v| v.as_object()) else {
        return;
    };

    for (action_key, action) in actions_obj {
        let action_path = format!("{path}.{action_key}");
        let hidden = is_hidden_action(action);

        if !hidden {
            if let Some(display) = action.get("display").and_then(|v| v.as_object()) {
                if let Some(label) = display.get("label").and_then(|v| v.as_str()) {
                    validate_label_quality(label, &format!("{action_path}.display"), errors);
                }
            }
        }

        if let Some(operation) = action.get("operation") {
            validate_operation_quality(operation, &action_path, sample_shape, hidden, errors);
        }
    }
}

fn is_hidden_action(action: &Value) -> bool {
    action
        .get("display")
        .and_then(|display| display.get("hidden"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn validate_operation_quality(
    operation: &Value,
    path: &str,
    sample_shape: SampleShape,
    hidden: bool,
    errors: &mut Vec<String>,
) {
    let Some(obj) = operation.as_object() else {
        return;
    };

    if !hidden {
        if let Some(fields) = obj.get("input_fields") {
            validate_input_field_list_quality(
                fields,
                &format!("{path}.operation.input_fields"),
                false,
                errors,
            );
        }

    }

    match obj.get("sample") {
        None => errors.push(format!("{path}.operation.sample is required")),
        Some(sample) if sample.is_object() => {
            let empty = sample
                .as_object()
                .map(|map| map.is_empty())
                .unwrap_or(false);
            if empty {
                errors.push(format!("{path}.operation.sample must not be empty"));
            }
            if !hidden {
                validate_normalized_sample_shape(sample, path, sample_shape, errors);
            }
        }
        Some(_) => errors.push(format!("{path}.operation.sample must be a map")),
    }
}

fn validate_input_field_list_quality(
    fields: &Value,
    path: &str,
    auth_field: bool,
    errors: &mut Vec<String>,
) {
    let Some(items) = fields.as_array() else {
        errors.push(format!("{path} must be a list"));
        return;
    };

    for (index, field) in items.iter().enumerate() {
        let field_path = format!("{path}[{index}]");
        validate_input_field_quality(field, &field_path, auth_field, errors);
    }
}

fn validate_input_field_quality(
    field: &Value,
    path: &str,
    auth_field: bool,
    errors: &mut Vec<String>,
) {
    let Some(obj) = field.as_object() else {
        errors.push(format!("{path} must be a map"));
        return;
    };

    match obj.get("label").and_then(|v| v.as_str()) {
        Some(label) => validate_label_quality(label, path, errors),
        None => errors.push(format!("{path} must include label")),
    }

    validate_help_text_quality(obj.get("help_text"), path, errors);
    validate_field_type_quality(obj.get("type"), path, auth_field, errors);
}

fn validate_field_type_quality(
    field_type: Option<&Value>,
    path: &str,
    auth_field: bool,
    errors: &mut Vec<String>,
) {
    let allowed = if auth_field {
        VALID_AUTH_FIELD_TYPES
    } else {
        VALID_INPUT_FIELD_TYPES
    };

    match field_type {
        None => {}
        Some(Value::String(type_name)) if allowed.contains(&type_name.as_str()) => {}
        Some(Value::String(type_name)) => {
            errors.push(format!(
                "{path}.type must be one of: {} (got {type_name:?})",
                allowed.join(", ")
            ));
        }
        Some(_) => errors.push(format!("{path}.type must be a string")),
    }
}

fn validate_label_quality(label: &str, path: &str, errors: &mut Vec<String>) {
    let trimmed = label.trim();

    if trimmed.chars().count() < MIN_LABEL_LENGTH {
        errors.push(format!(
            "{path}.label must be at least {MIN_LABEL_LENGTH} characters (got {label:?})"
        ));
    }
}

fn validate_help_text_quality(help_text: Option<&Value>, path: &str, errors: &mut Vec<String>) {
    match help_text.and_then(|v| v.as_str()) {
        Some(text) if !text.trim().is_empty() => {}
        Some(_) => errors.push(format!("{path}.help_text must not be empty")),
        None => errors.push(format!("{path} must include help_text")),
    }
}

fn validate_normalized_sample_shape(
    sample: &Value,
    path: &str,
    sample_shape: SampleShape,
    errors: &mut Vec<String>,
) {
    match sample_shape {
        SampleShape::Trigger => {}
        SampleShape::DataObject => {
            if !sample_data_is_object(sample) {
                errors.push(format!(
                    "{path}.operation.sample must be {{ \"data\": <object> }} (run ww-cli build)"
                ));
            }
        }
        SampleShape::DataArray => {
            if !sample_data_is_array(sample) {
                errors.push(format!(
                    "{path}.operation.sample must be {{ \"data\": [<object>] }} (run ww-cli build)"
                ));
            }
        }
    }
}

fn sample_data_is_object(sample: &Value) -> bool {
    sample
        .get("data")
        .and_then(|value| value.as_object())
        .map(|map| !map.is_empty())
        .unwrap_or(false)
}

fn sample_data_is_array(sample: &Value) -> bool {
    sample
        .get("data")
        .and_then(|value| value.as_array())
        .map(|items| {
            !items.is_empty()
                && items.iter().all(|item| {
                    item.as_object().map(|map| !map.is_empty()).unwrap_or(false)
                })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_missing_help_text() {
        let def = json!({
            "authentication": {
                "type": "api_key",
                "fields": [{ "key": "api_key", "label": "API Key" }]
            },
            "creates": {
                "create_contact": {
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "input_fields": [{ "key": "name", "label": "Full name" }],
                        "sample": { "data": { "id": "123" } }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains("help_text")));
    }

    #[test]
    fn rejects_create_sample_without_data_wrapper() {
        let def = json!({
            "creates": {
                "create_contact": {
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "sample": { "id": "123" }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains("data")));
    }

    #[test]
    fn rejects_search_sample_without_data_array_wrapper() {
        let def = json!({
            "searches": {
                "find_contact": {
                    "display": { "label": "Find Contact" },
                    "operation": {
                        "sample": { "id": "123" }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains("data")));
    }

    #[test]
    fn rejects_invalid_input_field_type() {
        let def = json!({
            "creates": {
                "create_contact": {
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "input_fields": [{
                            "key": "name",
                            "label": "Full name",
                            "type": "strng",
                            "help_text": "The contact full name as it should appear in the CRM."
                        }],
                        "sample": { "data": { "id": "123" } }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains(".type must be one of")));
    }

    #[test]
    fn rejects_create_sample_with_empty_data_object() {
        let def = json!({
            "creates": {
                "create_contact": {
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "sample": { "data": {} }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains("data") && e.contains("object")));
    }

    #[test]
    fn rejects_search_sample_with_empty_data_object_in_array() {
        let def = json!({
            "searches": {
                "find_contact": {
                    "display": { "label": "Find Contact" },
                    "operation": {
                        "sample": { "data": [{}] }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(errors.iter().any(|e| e.contains("data") && e.contains("object")));
    }

    #[test]
    fn accepts_boolean_auth_field_type() {
        let def = json!({
            "authentication": {
                "type": "custom",
                "fields": [{
                    "key": "use_production",
                    "label": "Use Production API",
                    "type": "boolean",
                    "help_text": "When enabled, calls the production API host."
                }]
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(!errors.iter().any(|e| e.contains(".type must be one of")));
    }

    #[test]
    fn skips_field_quality_for_hidden_actions() {
        let def = json!({
            "creates": {
                "run_javascript": {
                    "display": { "label": "Run JavaScript", "hidden": true },
                    "operation": {
                        "input_fields": [
                            { "key": "code", "label": "Code", "type": "code" },
                            { "key": "inputs", "label": "Inputs", "type": "array" }
                        ],
                        "sample": { "result": "ok" }
                    }
                }
            }
        });

        let errors = validate_definition_quality(&def);
        assert!(!errors.iter().any(|e| e.contains("help_text")));
        assert!(!errors.iter().any(|e| e.contains(".type must be one of")));
        assert!(!errors.iter().any(|e| e.contains("data")));
    }
}
