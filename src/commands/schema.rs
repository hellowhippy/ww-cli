use serde_json::Value;
use std::collections::HashSet;

const ROOT_KEYS: &[&str] = &[
    "version",
    "platform_version",
    "authentication",
    "ingress",
    "triggers",
    "creates",
    "searches",
    "hydrators",
];

const INGRESS_KEYS: &[&str] = &["handle"];
const ROUTING_KEYS: &[&str] = &["keys", "values", "from_input", "from_credential"];
const AUTH_KEYS: &[&str] = &[
    "type",
    "fields",
    "connection_label",
    "oauth2_config",
    "get_access_token",
    "refresh_access_token",
    "test",
    "session_config",
];

const AUTH_FIELD_KEYS: &[&str] = &[
    "key",
    "label",
    "type",
    "required",
    "help_text",
    "default",
    "include_in_authorize_url",
    // Platform-managed fields (filled by get_session_key / test): declared so the
    // engine injects them into auth_data, but hidden from connect forms.
    "hidden",
];

const OAUTH2_CONFIG_KEYS: &[&str] = &["authorize_url", "client_id", "client_secret", "scopes"];
const AUTHORIZE_URL_OBJECT_KEYS: &[&str] = &["url", "params"];
const SESSION_CONFIG_KEYS: &[&str] = &["perform"];
const ACTION_KEYS: &[&str] = &["key", "noun", "display", "operation"];
const DISPLAY_KEYS: &[&str] = &["label", "description", "hidden"];
const INPUT_FIELD_KEYS: &[&str] = &[
    "key",
    "label",
    "type",
    "required",
    "help_text",
    "default",
    "dynamic",
    "search",
    "alters_dynamic_fields",
    "after_dynamic_input_fields",
    "show_when",
    "choices",
    "choices_multiple",
    "description",
    "example",
    "renderer",
    "allow_manual_input",
    "children",
    "children_type",
    "item_label",
];
const VALID_INPUT_FIELD_RENDERERS: &[&str] = &[
    "jsonEditor",
    "jsonEditorFields",
    "schedule",
    "utcDatetime",
    "htmlEditor",
    "pdfTemplateEditor",
];
const SHOW_WHEN_KEYS: &[&str] = &["field", "one_of", "not_one_of", "if_unset"];
const OUTPUT_FIELD_KEYS: &[&str] = &["key", "label", "description"];
const DYNAMIC_INPUT_FIELDS_CONFIG_KEYS: &[&str] = &["perform", "required_input_keys"];

const OPERATION_BASE_KEYS: &[&str] = &[
    "input_fields",
    "output_fields",
    "sample",
    "perform",
    "dynamic_input_fields",
];

const TRIGGER_OPERATION_KEYS: &[&str] = &[
    "input_fields",
    "output_fields",
    "sample",
    "perform",
    "dynamic_input_fields",
    "type",
    "perform_subscribe",
    "perform_unsubscribe",
    "perform_list",
    "delivery",
    "routing",
];

const SEARCH_OPERATION_KEYS: &[&str] = &[
    "input_fields",
    "output_fields",
    "sample",
    "perform",
    "dynamic_input_fields",
    "perform_list",
];

pub fn validate_schema_keys(definition: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(obj) = definition.as_object() {
        unknown_keys_at(obj, ROOT_KEYS, "definition", &mut errors);
        if let Some(auth) = obj.get("authentication") {
            validate_authentication_schema(auth, &mut errors);
        }
        if let Some(ingress) = obj.get("ingress") {
            validate_ingress_schema(ingress, &mut errors);
        }
        validate_actions_schema(obj.get("triggers"), "triggers", TRIGGER_OPERATION_KEYS, &mut errors);
        validate_actions_schema(obj.get("creates"), "creates", OPERATION_BASE_KEYS, &mut errors);
        validate_actions_schema(obj.get("searches"), "searches", SEARCH_OPERATION_KEYS, &mut errors);
        validate_actions_schema(obj.get("hydrators"), "hydrators", OPERATION_BASE_KEYS, &mut errors);
    }

    errors
}

fn validate_ingress_schema(ingress: &Value, errors: &mut Vec<String>) {
    // Match API `validate_ingress/2`: null is allowed; configured objects must be maps.
    if ingress.is_null() {
        return;
    }

    let Some(obj) = ingress.as_object() else {
        errors.push("ingress must be a map".to_string());
        return;
    };

    unknown_keys_at(obj, INGRESS_KEYS, "ingress", errors);
}

fn validate_authentication_schema(auth: &Value, errors: &mut Vec<String>) {
    let Some(obj) = auth.as_object() else {
        errors.push("authentication must be a map".to_string());
        return;
    };

    unknown_keys_at(obj, AUTH_KEYS, "authentication", errors);

    if let Some(fields) = auth.get("fields") {
        validate_field_list_schema(fields, "authentication.fields", AUTH_FIELD_KEYS, errors);
    }

    if let Some(config) = auth.get("oauth2_config") {
        if let Some(config_obj) = config.as_object() {
            unknown_keys_at(
                config_obj,
                OAUTH2_CONFIG_KEYS,
                "authentication.oauth2_config",
                errors,
            );
            validate_authorize_url_schema(config.get("authorize_url"), errors);
        }
    }

    if let Some(config) = auth.get("session_config") {
        if let Some(config_obj) = config.as_object() {
            unknown_keys_at(
                config_obj,
                SESSION_CONFIG_KEYS,
                "authentication.session_config",
                errors,
            );
        }
    }
}

fn validate_authorize_url_schema(authorize_url: Option<&Value>, errors: &mut Vec<String>) {
    let Some(url_value) = authorize_url else {
        return;
    };

    let Some(obj) = url_value.as_object() else {
        return;
    };

    unknown_keys_at(
        obj,
        AUTHORIZE_URL_OBJECT_KEYS,
        "authentication.oauth2_config.authorize_url",
        errors,
    );

    if let Some(params) = obj.get("params") {
        if !params.is_object() {
            errors.push(
                "authentication.oauth2_config.authorize_url.params must be an object when present"
                    .to_string(),
            );
        }
    }
}

fn validate_actions_schema(
    actions: Option<&Value>,
    path: &str,
    operation_keys: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(actions_obj) = actions.and_then(|v| v.as_object()) else {
        return;
    };

    for (action_key, action) in actions_obj {
        let action_path = format!("{path}.{action_key}");

        let Some(action_obj) = action.as_object() else {
            errors.push(format!("{action_path} must be a map"));
            continue;
        };

        unknown_keys_at(action_obj, ACTION_KEYS, &action_path, errors);

        if let Some(display) = action.get("display") {
            if let Some(display_obj) = display.as_object() {
                unknown_keys_at(
                    display_obj,
                    DISPLAY_KEYS,
                    &format!("{action_path}.display"),
                    errors,
                );
            } else {
                errors.push(format!("{action_path}.display must be a map"));
            }
        }

        if let Some(operation) = action.get("operation") {
            validate_operation_schema(operation, operation_keys, &action_path, errors);
        }
    }
}

fn validate_operation_schema(
    operation: &Value,
    allowed_keys: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(obj) = operation.as_object() else {
        errors.push(format!("{path}.operation must be a map"));
        return;
    };

    unknown_keys_at(obj, allowed_keys, &format!("{path}.operation"), errors);

    if let Some(fields) = obj.get("input_fields") {
        validate_field_list_schema(
            fields,
            &format!("{path}.operation.input_fields"),
            INPUT_FIELD_KEYS,
            errors,
        );
    }

    if let Some(fields) = obj.get("output_fields") {
        validate_field_list_schema(
            fields,
            &format!("{path}.operation.output_fields"),
            OUTPUT_FIELD_KEYS,
            errors,
        );
    }

    if let Some(fields) = obj.get("dynamic_input_fields") {
        match fields {
            Value::Object(config) => validate_dynamic_input_fields_config(
                config,
                &format!("{path}.operation.dynamic_input_fields"),
                errors,
            ),
            _ => errors.push(format!(
                "{path}.operation.dynamic_input_fields must be a config object with perform and optional required_input_keys"
            )),
        }
    }

    if let Some(sample) = obj.get("sample") {
        if !sample.is_object() {
            errors.push(format!("{path}.operation.sample must be a map when present"));
        }
    }

    if let Some(routing) = obj.get("routing") {
        if let Some(routing) = routing.as_object() {
            unknown_keys_at(
                routing,
                ROUTING_KEYS,
                &format!("{path}.operation.routing"),
                errors,
            );
        } else {
            errors.push(format!("{path}.operation.routing must be a map"));
        }
    }
}

fn validate_dynamic_input_fields_config(
    config: &serde_json::Map<String, Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    unknown_keys_at(config, DYNAMIC_INPUT_FIELDS_CONFIG_KEYS, path, errors);

    match config.get("perform").and_then(|v| v.as_str()) {
        Some(perform) if !perform.trim().is_empty() => {}
        _ => errors.push(format!("{path} must include a non-empty perform string")),
    }

    match config.get("required_input_keys") {
        None => {}
        Some(Value::Array(keys)) => {
            if !keys.iter().all(|key| key.as_str().is_some()) {
                errors.push(format!(
                    "{path}.required_input_keys must be a list of strings"
                ));
            }
        }
        Some(_) => errors.push(format!(
            "{path}.required_input_keys must be a list of strings when present"
        )),
    }
}

fn validate_field_list_schema(
    fields: &Value,
    path: &str,
    allowed_keys: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(items) = fields.as_array() else {
        errors.push(format!("{path} must be a list"));
        return;
    };

    for (index, field) in items.iter().enumerate() {
        let field_path = format!("{path}[{index}]");

        let Some(field_obj) = field.as_object() else {
            errors.push(format!("{field_path} must be a map"));
            continue;
        };

        unknown_keys_at(field_obj, allowed_keys, &field_path, errors);

        if let Some(choices) = field.get("choices") {
            if !choices.is_object() {
                errors.push(format!("{field_path}.choices must be a map when present"));
            }
        }

        if let Some(after_dynamic) = field.get("after_dynamic_input_fields") {
            if !after_dynamic.is_boolean() {
                errors.push(format!(
                    "{field_path}.after_dynamic_input_fields must be a boolean when present"
                ));
            }
        }

        if let Some(choices_multiple) = field.get("choices_multiple") {
            if !choices_multiple.is_boolean() {
                errors.push(format!(
                    "{field_path}.choices_multiple must be a boolean when present"
                ));
            }
        }

        if let Some(allow_manual_input) = field.get("allow_manual_input") {
            if !allow_manual_input.is_boolean() {
                errors.push(format!(
                    "{field_path}.allow_manual_input must be a boolean when present"
                ));
            }
        }

        if let Some(show_when) = field.get("show_when") {
            validate_show_when_schema(show_when, &field_path, errors);
        }

        if let Some(renderer) = field.get("renderer") {
            validate_input_field_renderer_schema(renderer, &field_path, errors);
        }

        if let Some(children) = field.get("children") {
            if children.is_array() {
                validate_field_list_schema(
                    children,
                    &format!("{field_path}.children"),
                    allowed_keys,
                    errors,
                );
            } else {
                errors.push(format!("{field_path}.children must be a list when present"));
            }
        }
    }
}

fn validate_show_when_schema(show_when: &Value, path: &str, errors: &mut Vec<String>) {
    let Some(obj) = show_when.as_object() else {
        errors.push(format!("{path}.show_when must be a map when present"));
        return;
    };

    unknown_keys_at(
        obj,
        SHOW_WHEN_KEYS,
        &format!("{path}.show_when"),
        errors,
    );

    match obj.get("field").and_then(|v| v.as_str()) {
        Some(field) if !field.trim().is_empty() => {}
        _ => errors.push(format!("{path}.show_when.field must be a non-empty string")),
    }

    if let Some(one_of) = obj.get("one_of") {
        if !one_of
            .as_array()
            .map(|values| values.iter().all(|v| v.as_str().is_some()))
            .unwrap_or(false)
        {
            errors.push(format!(
                "{path}.show_when.one_of must be a list of strings when present"
            ));
        }
    }

    if let Some(not_one_of) = obj.get("not_one_of") {
        if !not_one_of
            .as_array()
            .map(|values| values.iter().all(|v| v.as_str().is_some()))
            .unwrap_or(false)
        {
            errors.push(format!(
                "{path}.show_when.not_one_of must be a list of strings when present"
            ));
        }
    }

    if let Some(if_unset) = obj.get("if_unset") {
        if !if_unset.is_string() {
            errors.push(format!(
                "{path}.show_when.if_unset must be a string when present"
            ));
        }
    }
}

fn validate_input_field_renderer_schema(renderer: &Value, path: &str, errors: &mut Vec<String>) {
    let Some(renderer) = renderer.as_str() else {
        errors.push(format!("{path}.renderer must be a string when present"));
        return;
    };

    if !VALID_INPUT_FIELD_RENDERERS.contains(&renderer) {
        errors.push(format!(
            "{path}.renderer must be one of: {} (got {renderer:?})",
            VALID_INPUT_FIELD_RENDERERS.join(", ")
        ));
    }
}

fn unknown_keys_at(
    obj: &serde_json::Map<String, Value>,
    allowed_keys: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let allowed: HashSet<&str> = allowed_keys.iter().copied().collect();

    for key in obj.keys() {
        if !allowed.contains(key.as_str()) {
            errors.push(format!("Unknown key at {path}: {key}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_unknown_top_level_key() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_contact": {
                    "key": "create_contact",
                    "noun": "Contact",
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "sample": { "id": "123" }
                    }
                }
            },
            "unexpected": true
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("Unknown key at definition: unexpected")));
    }

    #[test]
    fn accepts_dynamic_input_fields_config_object() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_row": {
                    "key": "create_row",
                    "noun": "Row",
                    "display": { "label": "Create Row" },
                    "operation": {
                        "sample": { "data": { "id": "row-123" } },
                        "dynamic_input_fields": {
                            "perform": "get_worksheet_row_input_fields",
                            "required_input_keys": ["spreadsheet_id", "worksheet_title"]
                        }
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn rejects_dynamic_input_fields_field_list() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_row": {
                    "key": "create_row",
                    "noun": "Row",
                    "display": { "label": "Create Row" },
                    "operation": {
                        "sample": { "data": { "id": "row-123" } },
                        "dynamic_input_fields": [
                            {
                                "key": "column_a",
                                "label": "Column A",
                                "help_text": "Value to write into spreadsheet column A."
                            }
                        ]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("dynamic_input_fields must be a config object")));
    }

    #[test]
    fn rejects_dynamic_input_fields_string() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_row": {
                    "key": "create_row",
                    "noun": "Row",
                    "display": { "label": "Create Row" },
                    "operation": {
                        "sample": { "data": { "id": "row-123" } },
                        "dynamic_input_fields": "get_worksheet_row_input_fields"
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("dynamic_input_fields must be a config object")));
    }

    #[test]
    fn rejects_dynamic_input_fields_config_object_without_perform() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_row": {
                    "key": "create_row",
                    "noun": "Row",
                    "display": { "label": "Create Row" },
                    "operation": {
                        "sample": { "data": { "id": "row-123" } },
                        "dynamic_input_fields": {
                            "required_input_keys": ["spreadsheet_id"]
                        }
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("must include a non-empty perform string")));
    }

    #[test]
    fn accepts_after_dynamic_input_fields_on_create_input_field() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_record": {
                    "key": "create_record",
                    "noun": "Record",
                    "display": { "label": "Create Record" },
                    "operation": {
                        "sample": { "data": { "id": "record-123" } },
                        "input_fields": [
                            {
                                "key": "properties",
                                "label": "Properties (JSON)",
                                "type": "string",
                                "after_dynamic_input_fields": true,
                                "help_text": "Optional JSON override after per-property fields."
                            }
                        ]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn rejects_non_boolean_after_dynamic_input_fields() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "searches": {
                "find_record": {
                    "key": "find_record",
                    "noun": "Record",
                    "display": { "label": "Find Record" },
                    "operation": {
                        "sample": { "data": [{ "id": "record-123" }] },
                        "input_fields": [
                            {
                                "key": "properties",
                                "label": "Properties (JSON)",
                                "type": "string",
                                "after_dynamic_input_fields": "yes",
                                "help_text": "Optional JSON override after per-property fields."
                            }
                        ]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("after_dynamic_input_fields must be a boolean")));
    }

    #[test]
    fn accepts_show_when_and_choices_multiple_on_input_field() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "merge_pdfs": {
                    "key": "merge_pdfs",
                    "noun": "PDF",
                    "display": { "label": "Merge PDFs" },
                    "operation": {
                        "sample": { "data": { "id": "pdf-123" } },
                        "input_fields": [
                            {
                                "key": "endpoint",
                                "label": "Endpoint",
                                "type": "string",
                                "choices": { "merge": "Merge" },
                                "help_text": "Choose the operation to run."
                            },
                            {
                                "key": "files",
                                "label": "Files",
                                "type": "string",
                                "show_when": {
                                    "field": "endpoint",
                                    "one_of": ["merge"]
                                },
                                "help_text": "PDF files to merge when endpoint is merge."
                            },
                            {
                                "key": "employment_types",
                                "label": "Employment Types",
                                "type": "string",
                                "choices": { "FULLTIME": "Full-time" },
                                "choices_multiple": true,
                                "allow_manual_input": true,
                                "help_text": "Filter by one or more employment types."
                            }
                        ]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn rejects_show_when_without_field() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "merge_pdfs": {
                    "key": "merge_pdfs",
                    "noun": "PDF",
                    "display": { "label": "Merge PDFs" },
                    "operation": {
                        "sample": { "data": { "id": "pdf-123" } },
                        "input_fields": [{
                            "key": "files",
                            "label": "Files",
                            "type": "string",
                            "show_when": { "one_of": ["merge"] },
                            "help_text": "PDF files to merge when endpoint is merge."
                        }]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("show_when.field must be a non-empty string")));
    }

    #[test]
    fn accepts_example_and_supported_renderer() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "merge_pdfs": {
                    "key": "merge_pdfs",
                    "noun": "PDF",
                    "display": { "label": "Merge PDFs" },
                    "operation": {
                        "sample": { "data": { "id": "pdf-123" } },
                        "input_fields": [{
                            "key": "filters",
                            "label": "Filters",
                            "type": "string",
                            "renderer": "jsonEditorFields",
                            "example": "{\"filters\":[]}",
                            "help_text": "Filter rules for the search request."
                        }]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn accepts_html_editor_renderer() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "send_email": {
                    "key": "send_email",
                    "noun": "Email",
                    "display": { "label": "Send Email" },
                    "operation": {
                        "sample": { "data": { "id": "msg-123" } },
                        "input_fields": [{
                            "key": "body",
                            "label": "Body",
                            "type": "text",
                            "renderer": "htmlEditor",
                            "help_text": "HTML email body authored in the workflow HTML editor."
                        }]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn accepts_pdf_template_editor_renderer() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "fill_pdf_template": {
                    "key": "fill_pdf_template",
                    "noun": "PDF",
                    "display": { "label": "Fill PDF Template" },
                    "operation": {
                        "sample": { "data": { "file_url": "https://example.com/filled.pdf" } },
                        "input_fields": [{
                            "key": "template",
                            "label": "PDF Template",
                            "type": "text",
                            "renderer": "pdfTemplateEditor",
                            "help_text": "PDF and field layout authored in the workflow PDF template editor."
                        }]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.is_empty(), "expected no schema errors, got: {errors:?}");
    }

    #[test]
    fn rejects_unsupported_renderer() {
        let def = json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "merge_pdfs": {
                    "key": "merge_pdfs",
                    "noun": "PDF",
                    "display": { "label": "Merge PDFs" },
                    "operation": {
                        "sample": { "data": { "id": "pdf-123" } },
                        "input_fields": [{
                            "key": "filters",
                            "label": "Filters",
                            "type": "string",
                            "renderer": "customWidget",
                            "help_text": "Filter rules for the search request."
                        }]
                    }
                }
            }
        });

        let errors = validate_schema_keys(&def);
        assert!(errors.iter().any(|e| e.contains("renderer must be one of")));
        assert!(errors.iter().any(|e| e.contains("htmlEditor")));
    }

    #[test]
    fn rejects_removed_redundant_input_field_keys() {
        for (key, value) in [
            ("placeholder", json!("e.g. 30")),
            ("json_editor_fields", json!(true)),
            ("json_editor_example", json!("{\"filters\":[]}")),
            ("manual_input_placeholder", json!("e.g. custom")),
        ] {
            let mut field = json!({
                "key": "name",
                "label": "Full name",
                "help_text": "The contact full name as it should appear in the CRM."
            });
            field
                .as_object_mut()
                .expect("field object")
                .insert(key.to_string(), value);

            let def = json!({
                "version": "1.0.0",
                "platform_version": "1.0",
                "authentication": {
                    "type": "api_key",
                    "fields": [{
                        "key": "api_key",
                        "label": "API Key",
                        "help_text": "Create or copy your API key from the provider developer settings page."
                    }]
                },
                "creates": {
                    "merge_pdfs": {
                        "key": "merge_pdfs",
                        "noun": "PDF",
                        "display": { "label": "Merge PDFs" },
                        "operation": {
                            "sample": { "data": { "id": "pdf-123" } },
                            "input_fields": [field]
                        }
                    }
                }
            });

            let errors = validate_schema_keys(&def);
            assert!(
                errors.iter().any(|e| e.contains("Unknown key")),
                "expected unknown key error for {key}, got: {errors:?}"
            );
        }
    }
}
