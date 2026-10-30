use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub fn generate_helper_files(src_dir: &PathBuf) -> Result<()> {
    let whippy_dts = r#"// Type definitions for Whippy helper object
export interface WhippyMultipartPart {
  name: string;
  type?: 'text' | 'file';
  value?: any;
  file?: any;
  filename?: string;
  contentType?: string;
  content_type?: string;
}

export interface WhippyRequestOptions {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: any;
  params?: Record<string, string>;
  multipart?: WhippyMultipartPart[];
  /** When true, returns raw bytes in `data` as ArrayBuffer instead of parsing JSON/text. */
  raw?: boolean;
}

export interface WhippyRequestResponse {
  status: number;
  headers: Record<string, string>;
  data: any;
  contentType?: string;
}

export interface WhippyBundleMeta {
  workflow_id: string;
  workflow_version_id: string;
  app_version_id?: string;
  app_action_type?: string;
  function?: string;
  execution_token?: string;
  credential_id?: string;
  workflow_host?: string;
  is_loading_sample?: boolean;
  is_filling_dynamic_dropdown?: boolean;
  is_populating_dedupe?: boolean;
  page?: number;
  limit?: number;
  cursor?: string;
}

export interface WhippyBundle {
  input_data: Record<string, any>;
  auth_data: Record<string, any>;
  target_url?: string;
  meta: WhippyBundleMeta;
  subscribe_data?: Record<string, any>;
  app_secrets?: Record<string, string>;
}

export interface WhippyStashFileOptions {
  filename?: string;
  content_type?: string;
  contentType?: string;
}

export interface WhippyDehydrateOptions {
  /** Cache hint in seconds (default 300, max 86400). */
  cacheExpiration?: number;
}

export interface WhippyConsole {
  log: (...args: any[]) => void;
  warn: (...args: any[]) => void;
  error: (...args: any[]) => void;
}

export interface WhippyErrors {
  Error: new (message?: string) => Error;
  ExpiredAuthError: new (message?: string) => Error;
  RefreshAuthError: new (message?: string) => Error;
  HaltedError: new (message?: string) => Error;
  ThrottledError: new (message?: string) => Error;
}

export interface WhippyJSON {
  parse: (text: string) => any;
  stringify: (value: any) => string;
}

export interface Whippy {
  request: (options: WhippyRequestOptions) => Promise<WhippyRequestResponse>;
  /** Upload file bytes or URL to Whippy storage; returns a stashed URL string. */
  stashFile: (
    input: string | ArrayBuffer | Uint8Array | Blob | Promise<any> | WhippyRequestResponse,
    options?: WhippyStashFileOptions,
  ) => Promise<string>;
  /** Return a lazy file pointer for downstream mapping (platform hydrates at run time). */
  dehydrateFile: (
    hydratorKey: string,
    inputData?: Record<string, any>,
    options?: WhippyDehydrateOptions,
  ) => string;
  /** Alias for dehydrateFile. */
  dehydrate: (
    hydratorKey: string,
    inputData?: Record<string, any>,
    options?: WhippyDehydrateOptions,
  ) => string;
  console: WhippyConsole;
  errors: WhippyErrors;
  JSON: WhippyJSON;
}

export declare const w: Whippy;
"#;

    let whippy_js = r#"// Whippy helper object stub for local development
// The actual implementation is provided by the Worker wrapper at runtime

export const w = {
  request: async (options) => {
    console.warn('w.request() called - this is a stub. The real implementation will be available at runtime.');
    return {
      status: 200,
      headers: {},
      data: options?.raw ? new ArrayBuffer(0) : {},
    };
  },

  stashFile: async () => {
    console.warn('w.stashFile() called - this is a stub. The real implementation will be available at runtime.');
    return 'https://example.com/workflows/files/stub-id';
  },

  dehydrateFile: (hydratorKey, inputData = {}) => {
    console.warn('w.dehydrateFile() called - this is a stub.');
    return `whippy:dehydrate-file:stub:${hydratorKey}:${JSON.stringify(inputData)}`;
  },

  dehydrate: (hydratorKey, inputData = {}, options) =>
    w.dehydrateFile(hydratorKey, inputData, options),

  console: {
    log: (...args) => console.log('[user]', ...args),
    warn: (...args) => console.warn('[user]', ...args),
    error: (...args) => console.error('[user]', ...args),
  },

  errors: {
    Error: class extends Error {},
    ExpiredAuthError: class extends Error { name = 'ExpiredAuthError'; },
    RefreshAuthError: class extends Error { name = 'RefreshAuthError'; },
    HaltedError: class extends Error { name = 'HaltedError'; },
    ThrottledError: class extends Error { name = 'ThrottledError'; },
  },

  JSON: {
    parse: (str) => {
      try {
        return JSON.parse(str);
      } catch {
        return null;
      }
    },
    stringify: JSON.stringify,
  },
};
"#;

    fs::write(src_dir.join("whippy.d.ts"), whippy_dts)?;
    fs::write(src_dir.join("whippy.js"), whippy_js)?;

    Ok(())
}
