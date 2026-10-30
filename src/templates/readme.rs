//! README template for `ww-cli init` scaffolds.

pub fn app_readme(app_name: &str, auth_type: &str) -> String {
    let title = title_case(app_name);
    let auth_section = auth_section(auth_type);

    format!(
        r#"# {title} workflow integration

Workflow app for TODO: provider name and one-line summary.

## Overview

TODO: What this app does.

TODO: Who uses it (teams, customers, internal workflows).

TODO: Primary use cases (e.g. sync contacts, send notifications, poll for updates).

## Supported Features

### Triggers

| Key | Description |
|-----|-------------|
| `new_contact` | TODO: when this trigger fires |
| `new_file` | TODO: webhook trigger with dehydrated file sample |

### Actions

| Key | Description |
|-----|-------------|
| `create_contact` | TODO: what resource this creates |
| `upload_file` | TODO: file upload behavior |

### Searches

| Key | Description |
|-----|-------------|
| `find_contact` | TODO: how users look up records |

### Hydrators

| Key | Description |
|-----|-------------|
| `download_file` | TODO: lazy file fetch for downstream steps |

## Authentication

{auth_section}

## API Documentation

TODO: Link all API documentation used to build this app.

- TODO: OpenAPI / developer portal URL
- TODO: Auth guide URL
- TODO: Relevant endpoint reference pages

Base URL: `TODO: https://api.example.com`

## Implementation Notes

TODO: Document unusual decisions (pagination, rate limits, field mapping, dynamic fields, webhooks).

- Scaffold auth type: `{auth_type}`
- Example file handling: `new_file` trigger, `upload_file` create, `download_file` hydrator

## Known Limitations

TODO: Demo account limitations.

TODO: API limitations (rate limits, missing endpoints, unsupported fields).

TODO: Missing functionality not yet implemented in this app.
"#
    )
}

fn title_case(name: &str) -> String {
    name.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn auth_section(auth_type: &str) -> String {
    match auth_type {
        "api_key" => r"| Item | Details |
|------|---------|
| Auth type | `api_key` |
| Credential ownership | TODO: who creates and owns API keys |
| Required scopes | N/A |
| Callback URLs | N/A |
| Rotation process | TODO: how keys are rotated or revoked |

Implementation: [`src/authentication.ts`](src/authentication.ts)"
            .to_string(),
        "oauth2" => r"| Item | Details |
|------|---------|
| Auth type | `oauth2` |
| Credential ownership | TODO: who owns the OAuth app / client |
| Required scopes | TODO: list scopes |
| Callback URLs | TODO: redirect URI registered with the provider |
| Rotation process | TODO: refresh token / re-auth process |

Implementation: [`src/authentication.ts`](src/authentication.ts)"
            .to_string(),
        "basic_auth" => r"| Item | Details |
|------|---------|
| Auth type | `basic_auth` |
| Credential ownership | TODO: who provisions username/password |
| Required scopes | N/A |
| Callback URLs | N/A |
| Rotation process | TODO: password rotation process |

Implementation: [`src/authentication.ts`](src/authentication.ts)"
            .to_string(),
        "session" => r"| Item | Details |
|------|---------|
| Auth type | `session` |
| Credential ownership | TODO: who owns session credentials |
| Required scopes | N/A |
| Callback URLs | N/A |
| Rotation process | TODO: session expiry and renewal |

Implementation: [`src/authentication.ts`](src/authentication.ts)"
            .to_string(),
        "custom" => r"| Item | Details |
|------|---------|
| Auth type | `custom` |
| Credential ownership | TODO: who owns custom auth credentials |
| Required scopes | TODO: if applicable |
| Callback URLs | TODO: if applicable |
| Rotation process | TODO: how credentials are rotated |

Implementation: [`src/authentication.ts`](src/authentication.ts)"
            .to_string(),
        other => format!(
            "| Item | Details |\n|------|---------|\n| Auth type | `{other}` |\n| Credential ownership | TODO |\n| Required scopes | TODO |\n| Callback URLs | TODO |\n| Rotation process | TODO |\n\nImplementation: [`src/authentication.ts`](src/authentication.ts)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_includes_required_sections() {
        let readme = app_readme("my-app", "api_key");
        for heading in [
            "## Overview",
            "## Supported Features",
            "## Authentication",
            "## API Documentation",
            "## Implementation Notes",
            "## Known Limitations",
            "### Triggers",
            "### Actions",
            "### Searches",
        ] {
            assert!(readme.contains(heading), "missing section: {heading}");
        }
    }

    #[test]
    fn readme_prefills_auth_type() {
        let readme = app_readme("acme", "oauth2");
        assert!(readme.contains("Auth type | `oauth2`"));
        assert!(readme.contains("# Acme workflow integration"));
    }
}
