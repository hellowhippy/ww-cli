//! Example hydrator, trigger, and create modules for default `ww-cli init` scaffolds.

pub fn hydrator_download_file(language: &str) -> String {
    if language == "typescript" {
        r#"import type { Whippy, WhippyBundle } from '../whippy.js';

const perform = async (w: Whippy, bundle: WhippyBundle) => {
  const response = await w.request({
    url: `https://example.com/files/${bundle.input_data.id}/download`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    raw: true,
  });

  if (response.status !== 200) {
    throw new Error(`Failed to download file: HTTP ${response.status}`);
  }

  const filename = bundle.input_data.filename || `file-${bundle.input_data.id}.pdf`;
  const url = await w.stashFile(response, {
    filename,
    contentType: response.contentType || 'application/pdf',
  });

  return {
    url,
    filename,
    content_type: response.contentType || 'application/pdf',
    size: response.data.byteLength,
  };
};

export const downloadFile = {
  key: 'download_file',
  noun: 'File',
  display: {
    label: 'Download File',
    description: 'Fetches a remote file and stashes it for downstream steps.',
  },
  operation: {
    input_fields: [
      { key: 'id', label: 'File ID', type: 'string', required: true },
      { key: 'filename', label: 'Filename', type: 'string', required: false },
    ],
    perform,
  },
};
"#
        .to_string()
    } else {
        r#"const perform = async (w, bundle) => {
  const response = await w.request({
    url: `https://example.com/files/${bundle.input_data.id}/download`,
    method: 'GET',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    raw: true,
  });

  if (response.status !== 200) {
    throw new Error(`Failed to download file: HTTP ${response.status}`);
  }

  const filename = bundle.input_data.filename || `file-${bundle.input_data.id}.pdf`;
  const url = await w.stashFile(response, {
    filename,
    contentType: response.contentType || 'application/pdf',
  });

  return {
    url,
    filename,
    content_type: response.contentType || 'application/pdf',
    size: response.data.byteLength,
  };
};

export const downloadFile = {
  key: 'download_file',
  noun: 'File',
  display: {
    label: 'Download File',
    description: 'Fetches a remote file and stashes it for downstream steps.',
  },
  operation: {
    input_fields: [
      { key: 'id', label: 'File ID', type: 'string', required: true },
      { key: 'filename', label: 'Filename', type: 'string', required: false },
    ],
    perform,
  },
};
"#
        .to_string()
    }
}

pub fn trigger_new_file(language: &str) -> String {
    if language == "typescript" {
        r#"import type { Whippy, WhippyBundle } from '../whippy.js';

const perform_subscribe = async (w: Whippy, bundle: WhippyBundle) => {
  const response = await w.request({
    url: 'https://api.example.com/webhooks',
    method: 'POST',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    body: {
      url: bundle.target_url,
      event: 'file.created',
      active: true,
    },
  });

  return {
    webhook_id: response.data.id,
    url: bundle.target_url,
  };
};

const perform_unsubscribe = async (w: Whippy, bundle: WhippyBundle) => {
  const webhook_id = bundle.subscribe_data?.webhook_id;

  if (webhook_id) {
    await w.request({
      url: `https://api.example.com/webhooks/${webhook_id}`,
      method: 'DELETE',
      headers: {
        Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
      },
    });
  }

  return {};
};

// Builder sample preview only (trigger_perform_list). Runtime webhook payloads pass through as-is.
const perform_list = async (w: Whippy, _bundle: WhippyBundle) => [
  {
    id: 'file-123',
    name: 'invoice.pdf',
    document: w.dehydrateFile('download_file', {
      id: 'file-123',
      filename: 'invoice.pdf',
    }),
  },
];

export const newFile = {
  key: 'new_file',
  noun: 'File',
  display: {
    label: 'New File',
    description: 'Triggers when a new file is uploaded via webhook. Webhook payloads pass through unchanged; perform_list shows lazy file pointers in the builder.',
  },
  operation: {
    type: 'webhook',
    input_fields: [],
    perform_subscribe,
    perform_unsubscribe,
    perform_list,
    sample: {
      id: 'file-123',
      name: 'invoice.pdf',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'document', label: 'Document', type: 'file' },
    ],
  },
};
"#
        .to_string()
    } else {
        r#"const perform_subscribe = async (w, bundle) => {
  const response = await w.request({
    url: 'https://api.example.com/webhooks',
    method: 'POST',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    body: {
      url: bundle.target_url,
      event: 'file.created',
      active: true,
    },
  });

  return {
    webhook_id: response.data.id,
    url: bundle.target_url,
  };
};

const perform_unsubscribe = async (w, bundle) => {
  const webhook_id = bundle.subscribe_data?.webhook_id;

  if (webhook_id) {
    await w.request({
      url: `https://api.example.com/webhooks/${webhook_id}`,
      method: 'DELETE',
      headers: {
        Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
      },
    });
  }

  return {};
};

const perform_list = async (w, _bundle) => [
  {
    id: 'file-123',
    name: 'invoice.pdf',
    document: w.dehydrateFile('download_file', {
      id: 'file-123',
      filename: 'invoice.pdf',
    }),
  },
];

export const newFile = {
  key: 'new_file',
  noun: 'File',
  display: {
    label: 'New File',
    description: 'Triggers when a new file is uploaded via webhook. Webhook payloads pass through unchanged; perform_list shows lazy file pointers in the builder.',
  },
  operation: {
    type: 'webhook',
    input_fields: [],
    perform_subscribe,
    perform_unsubscribe,
    perform_list,
    sample: {
      id: 'file-123',
      name: 'invoice.pdf',
    },
    output_fields: [
      { key: 'id', label: 'ID' },
      { key: 'name', label: 'Name' },
      { key: 'document', label: 'Document', type: 'file' },
    ],
  },
};
"#
        .to_string()
    }
}

pub fn create_upload_file(language: &str) -> String {
    if language == "typescript" {
        r#"import type { Whippy, WhippyBundle } from '../whippy.js';

const perform = async (w: Whippy, bundle: WhippyBundle) => {
  const file = bundle.input_data.file;
  const fileUrl = typeof file === 'string' ? file : file?.url;

  const response = await w.request({
    url: 'https://example.com/uploads',
    method: 'POST',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    multipart: [
      { name: 'title', type: 'text', value: bundle.input_data.title },
      {
        name: 'file',
        type: 'file',
        value: fileUrl,
        filename: file?.filename || 'upload.pdf',
      },
    ],
  });

  if (response.status !== 200 && response.status !== 201) {
    throw new Error(`Upload failed: HTTP ${response.status}`);
  }

  return response.data;
};

export const uploadFile = {
  key: 'upload_file',
  noun: 'File',
  display: {
    label: 'Upload File',
    description: 'Uploads a mapped file (from trigger or prior step) to the remote API.',
  },
  operation: {
    input_fields: [
      { key: 'title', label: 'Title', type: 'string', required: true },
      { key: 'file', label: 'File', type: 'file', required: true },
    ],
    perform,
    sample: {
      id: 'upload-1',
      title: 'Invoice',
    },
    output_fields: [
      { key: 'id', label: 'Upload ID' },
    ],
  },
};
"#
        .to_string()
    } else {
        r#"const perform = async (w, bundle) => {
  const file = bundle.input_data.file;
  const fileUrl = typeof file === 'string' ? file : file?.url;

  const response = await w.request({
    url: 'https://example.com/uploads',
    method: 'POST',
    headers: {
      Authorization: `Bearer ${bundle.auth_data.api_key ?? ''}`,
    },
    multipart: [
      { name: 'title', type: 'text', value: bundle.input_data.title },
      {
        name: 'file',
        type: 'file',
        value: fileUrl,
        filename: file?.filename || 'upload.pdf',
      },
    ],
  });

  if (response.status !== 200 && response.status !== 201) {
    throw new Error(`Upload failed: HTTP ${response.status}`);
  }

  return response.data;
};

export const uploadFile = {
  key: 'upload_file',
  noun: 'File',
  display: {
    label: 'Upload File',
    description: 'Uploads a mapped file (from trigger or prior step) to the remote API.',
  },
  operation: {
    input_fields: [
      { key: 'title', label: 'Title', type: 'string', required: true },
      { key: 'file', label: 'File', type: 'file', required: true },
    ],
    perform,
    sample: {
      id: 'upload-1',
      title: 'Invoice',
    },
    output_fields: [
      { key: 'id', label: 'Upload ID' },
    ],
  },
};
"#
        .to_string()
    }
}

pub fn test_data_example(auth_block: &str) -> String {
    format!(
        r#"{{
  "input_data": {{
    "name": "Jane Doe",
    "email": "jane@example.com",
    "query": "search term",
    "id": "file-123",
    "filename": "invoice.pdf",
    "title": "Invoice"
  }},
{auth_block},
  "meta": {{
    "workflow_id": "00000000-0000-0000-0000-000000000001",
    "workflow_version_id": "00000000-0000-0000-0000-000000000002",
    "app_version_id": "00000000-0000-0000-0000-000000000003",
    "app_action_type": "create",
    "function": "creates.upload_file.operation.perform",
    "execution_token": "local-dev-token"
  }},
  "target_url": "http://localhost:4003/api/v1/workflows/00000000-0000-0000-0000-000000000001/trigger/00000000-0000-0000-0000-000000000002"
}}
"#
    )
}
