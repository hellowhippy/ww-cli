use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::generators::js_helper;
use crate::utils::language;

const VALID_AUTH_TYPES: [&str; 5] = ["api_key", "oauth2", "basic_auth", "session", "custom"];

fn normalize_auth(auth: &str) -> Result<String> {
    let normalized = match auth.to_lowercase().as_str() {
        "api_key" | "apikey" => "api_key",
        "oauth2" | "oauth" | "oauth_2" => "oauth2",
        "basic_auth" | "basic" => "basic_auth",
        "session" => "session",
        "custom" => "custom",
        _ => {
            anyhow::bail!(
                "Unsupported auth type: '{}'. Supported: {}",
                auth,
                VALID_AUTH_TYPES.join(", ")
            );
        }
    };
    Ok(normalized.to_string())
}

pub fn run(app_name: String, language: String, auth: String) -> Result<()> {
    let lang = language::normalize(&language)?;
    let auth_type = normalize_auth(&auth)?;

    // Check if directory already exists
    let app_dir = PathBuf::from(&app_name);
    if app_dir.exists() {
        anyhow::bail!(
            "Directory '{}' already exists. Please choose a different name or remove the existing directory.",
            app_name
        );
    }

    // Create directory structure
    fs::create_dir_all(&app_dir)
        .with_context(|| format!("Failed to create directory: {}", app_name))?;

    let src_dir = app_dir.join("src");
    fs::create_dir_all(&src_dir.join("triggers"))?;
    fs::create_dir_all(&src_dir.join("creates"))?;
    fs::create_dir_all(&src_dir.join("searches"))?;
    fs::create_dir_all(&src_dir.join("hydrators"))?;

    // Generate language-specific files
    match lang.as_str() {
        "js" | "ts" | "typescript" => {
            generate_js_ts_files(&app_dir, &app_name, &lang, &auth_type)?;
        }
        "python" => {
            anyhow::bail!("Python support coming soon");
        }
        _ => {
            anyhow::bail!(
                "Unsupported language: {}. Supported languages: typescript, javascript",
                lang
            );
        }
    }

    println!(
        "✓ Created app: {} (language: {}, auth: {})",
        app_name, lang, auth_type
    );
    println!("  Directory: {}", app_dir.display());
    println!("\nNext steps:");
    println!("  1. cd {}", app_name);
    println!("  2. Fill in README.md (overview, auth, API docs, limitations)");
    println!("  3. Edit src/index.* to adjust authentication or actions");
    println!("  4. Run 'whippy-workflows-cli build' when ready");
    println!("  5. For local testing: copy test-data.example.json to test-data.json, fill in auth_data, then run 'whippy-workflows-cli invoke <function> --data test-data.json'");
    println!("     File examples: invoke triggers.new_file.operation.perform_list, download_file, or upload_file");

    Ok(())
}

/// Returns the source code for the OAuth2 authentication module.
/// Exports the full auth object as default; index just references it. No .env, snake_case.
fn get_oauth2_authentication_module(is_typescript: bool) -> &'static str {
    if is_typescript {
        r#"// Replace with your provider's base URL (no .env — use constants).
const BASE_URL = 'https://your-provider.com';

const get_access_token = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/access-token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      code: bundle.input_data?.code ?? '',
      redirect_uri: bundle.input_data?.redirect_uri ?? '',
      client_id: bundle.auth_data?.client_id ?? '',
      client_secret: bundle.auth_data?.client_secret ?? '',
      grant_type: 'authorization_code',
    }).toString(),
  });

  if (response.status !== 200) {
    throw new Error('Unable to fetch access token: ' + JSON.stringify(response.data));
  }

  const result = response.data as Record<string, unknown>;
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token,
    expires_in: result.expires_in,
  };
};

const refresh_access_token = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/refresh-token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      refresh_token: bundle.auth_data?.refresh_token ?? '',
      client_id: bundle.auth_data?.client_id ?? '',
      client_secret: bundle.auth_data?.client_secret ?? '',
      grant_type: 'refresh_token',
    }).toString(),
  });

  if (response.status !== 200) {
    throw new Error('Unable to fetch access token: ' + JSON.stringify(response.data));
  }

  const result = response.data as Record<string, unknown>;
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token ?? bundle.auth_data?.refresh_token,
    expires_in: result.expires_in,
  };
};

const test = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.access_token ?? ''}`,
    },
  });

  if (response.status === 401) {
    throw new Error('The access token you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'oauth2',
  oauth2_config: {
    authorize_url: 'https://your-provider.com/oauth/authorize',
    client_id: 'your-client-id',
    client_secret: 'your-client-secret',
    scopes: 'read,write',
  },
  get_access_token,
  refresh_access_token,
  test,
};
"#
    } else {
        r#"// Replace with your provider's base URL (no .env — use constants).
const BASE_URL = 'https://your-provider.com';

const get_access_token = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/access-token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      code: bundle.input_data?.code ?? '',
      redirect_uri: bundle.input_data?.redirect_uri ?? '',
      client_id: bundle.auth_data?.client_id ?? '',
      client_secret: bundle.auth_data?.client_secret ?? '',
      grant_type: 'authorization_code',
    }).toString(),
  });

  if (response.status !== 200) {
    throw new Error('Unable to fetch access token: ' + JSON.stringify(response.data));
  }

  const result = response.data;
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token,
    expires_in: result.expires_in,
  };
};

const refresh_access_token = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/refresh-token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      refresh_token: bundle.auth_data?.refresh_token ?? '',
      client_id: bundle.auth_data?.client_id ?? '',
      client_secret: bundle.auth_data?.client_secret ?? '',
      grant_type: 'refresh_token',
    }).toString(),
  });

  if (response.status !== 200) {
    throw new Error('Unable to fetch access token: ' + JSON.stringify(response.data));
  }

  const result = response.data;
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token ?? bundle.auth_data?.refresh_token,
    expires_in: result.expires_in,
  };
};

const test = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.access_token ?? ''}`,
    },
  });

  if (response.status === 401) {
    throw new Error('The access token you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'oauth2',
  oauth2_config: {
    authorize_url: 'https://your-provider.com/oauth/authorize',
    client_id: 'your-client-id',
    client_secret: 'your-client-secret',
    scopes: 'read,write',
  },
  get_access_token,
  refresh_access_token,
  test,
};
"#
    }
}

fn get_test_data_auth(auth_type: &str) -> &'static str {
    match auth_type {
        "api_key" => {
            r#"  "auth_data": {
    "api_key": "your-api-key-here"
  }"#
        }
        "oauth2" => {
            r#"  "auth_data": {
    "access_token": "your-access-token",
    "refresh_token": "your-refresh-token"
  }"#
        }
        "basic_auth" => {
            r#"  "auth_data": {
    "username": "your-username",
    "password": "your-password"
  }"#
        }
        "session" => {
            r#"  "auth_data": {
    "client_id": "your-client-id",
    "client_secret": "your-client-secret",
    "access_token": "your-access-token"
  }"#
        }
        "custom" => {
            r#"  "auth_data": {
    "custom_token": "your-custom-token"
  }"#
        }
        _ => get_test_data_auth("api_key"),
    }
}

fn generate_authentication_module(auth_type: &str, language: &str) -> String {
    let is_typescript = language == "typescript";
    match auth_type {
        "oauth2" => get_oauth2_authentication_module(is_typescript).to_string(),
        "api_key" => get_api_key_authentication_module(is_typescript).to_string(),
        "basic_auth" => get_basic_auth_authentication_module(is_typescript).to_string(),
        "session" => get_session_authentication_module(is_typescript).to_string(),
        "custom" => get_custom_authentication_module(is_typescript).to_string(),
        _ => get_api_key_authentication_module(is_typescript).to_string(),
    }
}

/// API Key auth: single field for the key. Optional test to verify credentials.
fn get_api_key_authentication_module(is_typescript: bool) -> &'static str {
    if is_typescript {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.api_key ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The API key you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'api_key',
  fields: [
    { key: 'api_key', label: 'API Key', type: 'string', required: true },
  ],
  test,
};
"#
    } else {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.api_key ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The API key you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'api_key',
  fields: [
    { key: 'api_key', label: 'API Key', type: 'string', required: true },
  ],
  test,
};
"#
    }
}

/// Basic auth: username/password. No fields; platform prompts for them.
fn get_basic_auth_authentication_module(is_typescript: bool) -> &'static str {
    if is_typescript {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w: any, bundle: any) => {
  const username = bundle.auth_data?.username ?? '';
  const password = bundle.auth_data?.password ?? '';
  const credentials = btoa(`${username}:${password}`);
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Basic ${credentials}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The username and/or password you supplied is incorrect');
  }
  return response.data;
};

export default {
  type: 'basic_auth',
  fields: [
    { key: 'username', label: 'Username', type: 'string', required: true },
    { key: 'password', label: 'Password', type: 'password', required: true },
  ],
  test,
};
"#
    } else {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w, bundle) => {
  const username = bundle.auth_data?.username ?? '';
  const password = bundle.auth_data?.password ?? '';
  const credentials = btoa(`${username}:${password}`);
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Basic ${credentials}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The username and/or password you supplied is incorrect');
  }
  return response.data;
};

export default {
  type: 'basic_auth',
  fields: [
    { key: 'username', label: 'Username', type: 'string', required: true },
    { key: 'password', label: 'Password', type: 'password', required: true },
  ],
  test,
};
"#
    }
}

/// Session auth: exchange credentials (client_id, client_secret) for access_token via session_config.perform.
fn get_session_authentication_module(is_typescript: bool) -> &'static str {
    if is_typescript {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

// get_session_key must return a map with at least access_token. The platform merges
// this into the credential. Adapt the response parsing to your provider's token endpoint format.
const get_session_key = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: {
      client_id: bundle.auth_data?.client_id,
      client_secret: bundle.auth_data?.client_secret,
      grant_type: 'client_credentials',
    },
  });
  if (response.status !== 200) {
    throw new Error('Failed to exchange credentials for access token');
  }
  return { access_token: response.data?.access_token };
};

const test = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.access_token ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The session credentials are not valid');
  }
  return response.data;
};

export default {
  type: 'session',
  fields: [
    { key: 'client_id', label: 'Client ID', type: 'string', required: true },
    { key: 'client_secret', label: 'Client Secret', type: 'password', required: true },
  ],
  session_config: { perform: 'authentication.get_session_key' },
  get_session_key,
  test,
};
"#
    } else {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

// get_session_key must return a map with at least access_token. The platform merges
// this into the credential. Adapt the response parsing to your provider's token endpoint format.
const get_session_key = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/oauth/token`,
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: {
      client_id: bundle.auth_data?.client_id,
      client_secret: bundle.auth_data?.client_secret,
      grant_type: 'client_credentials',
    },
  });
  if (response.status !== 200) {
    throw new Error('Failed to exchange credentials for access token');
  }
  return { access_token: response.data?.access_token };
};

const test = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.access_token ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The session credentials are not valid');
  }
  return response.data;
};

export default {
  type: 'session',
  fields: [
    { key: 'client_id', label: 'Client ID', type: 'string', required: true },
    { key: 'client_secret', label: 'Client Secret', type: 'password', required: true },
  ],
  session_config: { perform: 'authentication.get_session_key' },
  get_session_key,
  test,
};
"#
    }
}

/// Custom auth: define your own fields.
fn get_custom_authentication_module(is_typescript: bool) -> &'static str {
    if is_typescript {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w: any, bundle: any) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.custom_token ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The custom token you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'custom',
  fields: [
    { key: 'custom_token', label: 'Custom Token', type: 'string', required: true },
  ],
  test,
};
"#
    } else {
        r#"// Replace with your provider's base URL.
const BASE_URL = 'https://your-provider.com';

const test = async (w, bundle) => {
  const response = await w.request({
    url: `${BASE_URL}/me`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data?.custom_token ?? ''}`,
    },
  });
  if (response.status === 401) {
    throw new Error('The custom token you supplied is not valid');
  }
  return response.data;
};

export default {
  type: 'custom',
  fields: [
    { key: 'custom_token', label: 'Custom Token', type: 'string', required: true },
  ],
  test,
};
"#
    }
}

fn generate_js_ts_files(
    app_dir: &PathBuf,
    app_name: &str,
    language: &str,
    auth_type: &str,
) -> Result<()> {
    let src_dir = app_dir.join("src");

    // Generate whippy.d.ts and whippy.js
    js_helper::generate_helper_files(&src_dir)?;

    // Generate package.json
    let package_json = serde_json::json!({
        "name": app_name,
        "version": "1.0.0",
        "type": "module",
        "scripts": {
            "build": "echo 'Use whippy-workflows-cli build'"
        }
    });

    let package_json_path = app_dir.join("package.json");
    fs::write(
        &package_json_path,
        serde_json::to_string_pretty(&package_json)?,
    )?;

    let extension = if language == "typescript" { "ts" } else { "js" };

    // Generate example trigger action (webhook type)
    let example_trigger = if language == "typescript" {
        r#"import { w } from '../whippy.js';

// Subscribe to webhook - called when workflow is published
const perform_subscribe = async (w: any, bundle: any) => {
  // bundle.target_url contains the webhook URL to register
  const response = await w.request({
    url: 'https://api.example.com/webhooks',
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${bundle.auth_data.api_key}`,
    },
    body: {
      url: bundle.target_url,
      event: 'contact.created',
      active: true,
    },
  });
  
  // Return data that will be stored and passed to unsubscribe
  return {
    webhook_id: response.data.id,
    url: bundle.target_url,
  };
};

// Unsubscribe from webhook - called when workflow is unpublished or deleted
const perform_unsubscribe = async (w: any, bundle: any) => {
  // bundle.subscribe_data contains the data returned from perform_subscribe
  const webhook_id = bundle.subscribe_data.webhook_id;
  
  await w.request({
    url: `https://api.example.com/webhooks/${webhook_id}`,
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${bundle.auth_data.api_key}`,
    },
  });
  
  return {};
};

// Handle incoming webhook event - called when webhook is received
const perform = async (w: any, bundle: any) => {
  // bundle.cleaned_request contains the webhook payload
  // Return the data that will be used as input to the workflow
  return bundle.cleaned_request;
};

export const newContact = {
  key: 'new_contact',
  noun: 'Contact',
  display: {
    label: 'New Contact',
    description: 'Triggers when a new contact is created via webhook.',
  },
  operation: {
    type: 'webhook',
    input_fields: [],
    perform_subscribe,
    perform_unsubscribe,
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
      created_at: '2024-01-01T00:00:00Z',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
      { key: 'created_at', label: 'Created At' },
    ],
  },
};
"#
    } else {
        r#"import { w } from '../whippy.js';

// Subscribe to webhook - called when workflow is published
const perform_subscribe = async (w, bundle) => {
  // bundle.target_url contains the webhook URL to register
  const response = await w.request({
    url: 'https://api.example.com/webhooks',
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${bundle.auth_data.api_key}`,
    },
    body: {
      url: bundle.target_url,
      event: 'contact.created',
      active: true,
    },
  });
  
  // Return data that will be stored and passed to unsubscribe
  return {
    webhook_id: response.data.id,
    url: bundle.target_url,
  };
};

// Unsubscribe from webhook - called when workflow is unpublished or deleted
const perform_unsubscribe = async (w, bundle) => {
  // bundle.subscribe_data contains the data returned from perform_subscribe
  const webhook_id = bundle.subscribe_data.webhook_id;
  
  await w.request({
    url: `https://api.example.com/webhooks/${webhook_id}`,
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${bundle.auth_data.api_key}`,
    },
  });
  
  return {};
};

// Handle incoming webhook event - called when webhook is received
const perform = async (w, bundle) => {
  // bundle.cleaned_request contains the webhook payload
  // Return the data that will be used as input to the workflow
  return bundle.cleaned_request;
};

export const newContact = {
  key: 'new_contact',
  noun: 'Contact',
  display: {
    label: 'New Contact',
    description: 'Triggers when a new contact is created via webhook.',
  },
  operation: {
    type: 'webhook',
    input_fields: [],
    perform_subscribe,
    perform_unsubscribe,
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
      created_at: '2024-01-01T00:00:00Z',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
      { key: 'created_at', label: 'Created At' },
    ],
  },
};
"#
    };

    let trigger_path = src_dir
        .join("triggers")
        .join(format!("new_contact.{}", extension));
    fs::write(&trigger_path, example_trigger)?;

    // Generate example create action
    let example_create = if language == "typescript" {
        r#"import { w } from '../whippy.js';

const perform = async (w: any, bundle: any) => {
  const response = await w.request({
    url: 'https://api.example.com/contacts',
    method: 'POST',
    body: {
      name: bundle.input_data.name,
      email: bundle.input_data.email,
    },
  });
  
  return response.data;
};

export const createContact = {
  key: 'create_contact',
  noun: 'Contact',
  display: {
    label: 'Create Contact',
    description: 'Creates a new contact in your CRM.',
  },
  operation: {
    input_fields: [
      {
        key: 'name',
        label: 'Name',
        type: 'string',
        required: true,
      },
      {
        key: 'email',
        label: 'Email',
        type: 'string',
        required: true,
      },
    ],
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
    ],
  },
};
"#
    } else {
        r#"import { w } from '../whippy.js';

const perform = async (w, bundle) => {
  const response = await w.request({
    url: 'https://api.example.com/contacts',
    method: 'POST',
    body: {
      name: bundle.input_data.name,
      email: bundle.input_data.email,
    },
  });
  
  return response.data;
};

export const createContact = {
  key: 'create_contact',
  noun: 'Contact',
  display: {
    label: 'Create Contact',
    description: 'Creates a new contact in your CRM.',
  },
  operation: {
    input_fields: [
      {
        key: 'name',
        label: 'Name',
        type: 'string',
        required: true,
      },
      {
        key: 'email',
        label: 'Email',
        type: 'string',
        required: true,
      },
    ],
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
    ],
  },
};
"#
    };

    let create_path = src_dir
        .join("creates")
        .join(format!("create_contact.{}", extension));
    fs::write(&create_path, example_create)?;

    // Generate example search action
    let example_search = if language == "typescript" {
        r#"import { w } from '../whippy.js';

const perform = async (w: any, bundle: any) => {
  const response = await w.request({
    url: 'https://api.example.com/contacts/search',
    method: 'GET',
    params: {
      q: bundle.input_data.query,
    },
  });
  
  return response.data;
};

export const findContact = {
  key: 'find_contact',
  noun: 'Contact',
  display: {
    label: 'Find Contact',
    description: 'Searches for a contact by name or email.',
  },
  operation: {
    input_fields: [
      {
        key: 'query',
        label: 'Search Query',
        type: 'string',
        required: true,
      },
    ],
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
    ],
  },
};
"#
    } else {
        r#"import { w } from '../whippy.js';

const perform = async (w, bundle) => {
  const response = await w.request({
    url: 'https://api.example.com/contacts/search',
    method: 'GET',
    params: {
      q: bundle.input_data.query,
    },
  });
  
  return response.data;
};

export const findContact = {
  key: 'find_contact',
  noun: 'Contact',
  display: {
    label: 'Find Contact',
    description: 'Searches for a contact by name or email.',
  },
  operation: {
    input_fields: [
      {
        key: 'query',
        label: 'Search Query',
        type: 'string',
        required: true,
      },
    ],
    perform,
    sample: {
      id: '123',
      name: 'Jane Doe',
      email: 'jane@example.com',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'email', label: 'Email' },
    ],
  },
};
"#
    };

    let search_path = src_dir
        .join("searches")
        .join(format!("find_contact.{}", extension));
    fs::write(&search_path, example_search)?;

    fs::write(
        src_dir.join(format!("hydrators/download_file.{}", extension)),
        crate::templates::file_examples::hydrator_download_file(language),
    )?;
    fs::write(
        src_dir.join(format!("triggers/new_file.{}", extension)),
        crate::templates::file_examples::trigger_new_file(language),
    )?;
    fs::write(
        src_dir.join(format!("creates/upload_file.{}", extension)),
        crate::templates::file_examples::create_upload_file(language),
    )?;

    // Generate authentication module for all auth types (exports full auth object; index references it)
    let auth_module = generate_authentication_module(auth_type, language);
    let auth_module_path = src_dir.join(format!("authentication.{}", extension));
    fs::write(&auth_module_path, auth_module)?;

    // Generate index.ts/js - main entry point that exports all actions
    let index_body = format!(
        r#"import {{ newContact }} from './triggers/new_contact.js';
import {{ newFile }} from './triggers/new_file.js';
import {{ createContact }} from './creates/create_contact.js';
import {{ uploadFile }} from './creates/upload_file.js';
import {{ findContact }} from './searches/find_contact.js';
import {{ downloadFile }} from './hydrators/download_file.js';
import authentication from './authentication.js';

export default {{
  // Version will be extracted from package.json during build
  version: '1.0.0',
  platform_version: '1.0.0',
  authentication,
  hydrators: {{
    [downloadFile.key]: downloadFile,
  }},
  triggers: {{
    [newContact.key]: newContact,
    [newFile.key]: newFile,
  }},
  creates: {{
    [createContact.key]: createContact,
    [uploadFile.key]: uploadFile,
  }},
  searches: {{
    [findContact.key]: findContact,
  }},
}};
"#
    );

    fs::write(&src_dir.join(format!("index.{}", extension)), index_body)?;

    // Generate example test data file for invoke (copy to test-data.json and fill in real auth)
    let test_data_auth = get_test_data_auth(auth_type);
    let test_data_example = crate::templates::file_examples::test_data_example(test_data_auth);
    let test_data_path = app_dir.join("test-data.example.json");
    fs::write(&test_data_path, test_data_example)
        .with_context(|| format!("Failed to write {}", test_data_path.display()))?;

    let readme_path = app_dir.join("README.md");
    let readme = crate::templates::readme::app_readme(app_name, auth_type);
    fs::write(&readme_path, readme)
        .with_context(|| format!("Failed to write {}", readme_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn init_scaffold_includes_file_handling_examples() {
        let app_dir = unique_temp_dir("ww-init-test");
        let app_name = app_dir.to_string_lossy().into_owned();

        run(
            app_name.clone(),
            "typescript".to_string(),
            "api_key".to_string(),
        )
        .expect("init should succeed");

        let src = app_dir.join("src");
        assert!(src.join("hydrators/download_file.ts").is_file());
        assert!(src.join("triggers/new_file.ts").is_file());
        assert!(src.join("creates/upload_file.ts").is_file());
        assert!(src.join("authentication.ts").is_file());

        let index = fs::read_to_string(src.join("index.ts")).expect("read index.ts");
        assert!(index.contains("hydrators:"));
        assert!(index.contains("downloadFile"));
        assert!(!index.contains("w.dehydrateFile"));

        let new_file = fs::read_to_string(src.join("triggers/new_file.ts")).expect("read new_file");
        assert!(new_file.contains("type: 'webhook'"));
        assert!(new_file.contains("perform_subscribe"));
        assert!(new_file.contains("perform_list"));
        assert!(!new_file.contains("const perform ="));
        assert!(new_file.contains("dehydrateFile"));

        let download =
            fs::read_to_string(src.join("hydrators/download_file.ts")).expect("read download");
        assert!(download.contains("stashFile"));
        assert!(download.contains("raw: true"));

        let test_data =
            fs::read_to_string(app_dir.join("test-data.example.json")).expect("read test data");
        assert!(test_data.contains("\"meta\""));
        assert!(test_data.contains("target_url"));

        let readme = fs::read_to_string(app_dir.join("README.md")).expect("read README.md");
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
            assert!(readme.contains(heading), "README missing section: {heading}");
        }
        assert!(readme.contains("Auth type | `api_key`"));

        fs::remove_dir_all(&app_dir).ok();
    }
}
