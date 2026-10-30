use clap::{Parser, Subcommand};

mod api;
mod bundler;
mod commands;
mod extractor;
mod generators;
mod templates;
mod utils;

use commands::*;

#[derive(Parser)]
#[command(name = "whippy-workflows-cli")]
#[command(about = "Whippy Workflows CLI for building custom integrations")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Login with your API key (stores in ~/.whippyrc)
    Login {
        /// API key (or will prompt if not provided)
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Logout (clears stored API key)
    Logout,
    /// Promote the current app from private to global
    MakeGlobal {
        /// API key (or set WHIPPY_API_KEY env var)
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Initialize a new custom integration
    Init {
        /// Name of the app
        name: String,
        /// Language (js, ts, typescript)
        #[arg(long, default_value = "typescript")]
        language: String,
        /// Authentication type for the generated app (api_key, oauth2, basic_auth, session, custom). Default: api_key.
        #[arg(long, default_value = "api_key")]
        auth: String,
    },
    /// Build the integration
    Build,
    /// Run the test suite (executes `npm test`)
    ///
    /// This command runs your integration's test suite. Make sure you have
    /// a test script defined in your package.json.
    ///
    /// Example:
    ///   whippy-workflows-cli test
    Test,
    /// Push the integration to Whippy
    ///
    /// Updates an existing app: it reads the app_id from .whippyapprc and pushes
    /// a new (or updated) version. If .whippyapprc is missing, push fails instead
    /// of silently creating a duplicate app. Use --create to explicitly register a
    /// new app (this writes .whippyapprc with the new app_id).
    Push {
        /// API key (or set WHIPPY_API_KEY env var)
        #[arg(long)]
        api_key: Option<String>,
        /// Allow pushing a version lower than the highest existing semver (overwrites older drafts)
        #[arg(long)]
        force: bool,
        /// Explicitly create a new app when .whippyapprc is missing (writes .whippyapprc with the new app_id)
        #[arg(long)]
        create: bool,
        /// With --create, proceed even if an app with the same name already exists in your organization
        #[arg(long, requires = "create")]
        allow_duplicate: bool,
    },
    /// Upload app logo (PNG) and optionally set brand color
    Logo {
        /// Path to logo PNG (default: assets/logo.png)
        #[arg(value_name = "FILE")]
        path: Option<std::path::PathBuf>,
        /// Brand color as 6-digit hex (e.g. #2563EB). Cannot be #FFFFFF.
        #[arg(long)]
        color: Option<String>,
        /// API key (or set WHIPPY_API_KEY env var)
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Validate the integration
    Validate,
    /// List all versions of an app
    Versions {
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
    },
    /// Promote a version to production
    Promote {
        /// Version to promote (e.g., "1.0.0" or "1.0.0-beta")
        version: String,
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
    },
    /// Deprecate a version
    Deprecate {
        /// Version to deprecate
        version: String,
        /// Deprecation date (ISO 8601, e.g. 2025-03-01T00:00:00Z). If omitted, defaults to now. The Cloudflare worker is removed after this date (daily cron).
        #[arg(long)]
        date: Option<String>,
        /// Reason for deprecation
        #[arg(long)]
        reason: Option<String>,
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
    },
    /// Migrate users from one version to another
    Migrate {
        /// Source version
        from_version: String,
        /// Target version
        to_version: String,
        /// Percentage of users to migrate (0-100, default: 100)
        #[arg(long, default_value = "100")]
        percentage: u8,
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
    },
    /// View logs for an app or version
    Logs {
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
        /// Version to view logs for
        #[arg(long)]
        version: Option<String>,
        /// Log type filter (http, bundle, console, audit)
        #[arg(long)]
        r#type: Option<String>,
        /// Number of log entries to show
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// Manage app-level secrets (`ww-cli secrets --help`)
    Secrets {
        #[command(subcommand)]
        command: secrets::SecretsCommand,
        /// App ID (or read from .whippyapprc)
        #[arg(long, global = true)]
        app_id: Option<String>,
        /// API key (or set WHIPPY_API_KEY / run login)
        #[arg(long, global = true)]
        api_key: Option<String>,
    },
    /// Invoke a function locally (for testing)
    ///
    /// You can use either short form (e.g., "create_contact") or full path
    /// (e.g., "creates.create_contact.operation.perform").
    ///
    /// If no function is provided, lists all available functions.
    ///
    /// Auth and test data:
    ///   Without --data: uses a sample bundle with placeholder auth (e.g. "test_value").
    ///   With --data <file>: use a JSON file with real auth_data and input_data so w.request()
    ///   sends real credentials. Copy test-data.example.json to test-data.json, fill in your
    ///   API keys, then: whippy-workflows-cli invoke create_contact --data test-data.json
    ///
    /// Examples:
    ///   whippy-workflows-cli invoke                    # List available functions
    ///   whippy-workflows-cli invoke create_contact     # Invoke with sample data
    ///   whippy-workflows-cli invoke create_contact --data test-data.json   # Invoke with real auth
    Invoke {
        /// Function name to invoke (short form like "create_contact" or full path). Omit to list available functions.
        function: Option<String>,
        /// JSON file with input_data and auth_data (use for real API keys). See test-data.example.json.
        #[arg(long)]
        data: Option<std::path::PathBuf>,
        /// App ID (or read from app.json)
        #[arg(long)]
        app_id: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login { api_key } => login::run(api_key),
        Commands::Logout => logout::run(),
        Commands::MakeGlobal { api_key } => app::make_global(api_key),
        Commands::Init {
            name,
            language,
            auth,
        } => init::run(name, language, auth),
        Commands::Build => build::run(),
        Commands::Test => test::run(),
        Commands::Push {
            api_key,
            force,
            create,
            allow_duplicate,
        } => push::run(api_key, force, create, allow_duplicate),
        Commands::Logo {
            path,
            color,
            api_key,
        } => logo::run(path, color, api_key),
        Commands::Validate => validate::run(),
        Commands::Versions { app_id } => versions::run(app_id),
        Commands::Promote { version, app_id } => promote::run(version, app_id),
        Commands::Deprecate {
            version,
            date,
            reason,
            app_id,
        } => deprecate::run(version, date, reason, app_id),
        Commands::Migrate {
            from_version,
            to_version,
            percentage,
            app_id,
        } => migrate::run(from_version, to_version, percentage, app_id),
        Commands::Logs {
            app_id,
            version,
            r#type,
            limit,
        } => logs::run(app_id, version, r#type, limit),
        Commands::Invoke {
            function,
            data,
            app_id,
        } => invoke::run(function, data, app_id),
        Commands::Secrets {
            command,
            app_id,
            api_key,
        } => secrets::run(command, app_id, api_key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_secrets_list() {
        let cli = Cli::parse_from(["ww-cli", "secrets", "list"]);
        assert!(matches!(
            cli.command,
            Commands::Secrets {
                command: secrets::SecretsCommand::List,
                app_id: None,
                api_key: None,
            }
        ));
    }

    #[test]
    fn parses_make_global_as_top_level_command() {
        let cli = Cli::parse_from(["ww-cli", "make-global"]);

        assert!(matches!(
            cli.command,
            Commands::MakeGlobal { api_key: None }
        ));
    }

    #[test]
    fn parses_push_without_create_flags() {
        let cli = Cli::parse_from(["ww-cli", "push"]);

        assert!(matches!(
            cli.command,
            Commands::Push {
                api_key: None,
                force: false,
                create: false,
                allow_duplicate: false,
            }
        ));
    }

    #[test]
    fn parses_push_create_flag() {
        let cli = Cli::parse_from(["ww-cli", "push", "--create"]);

        assert!(matches!(
            cli.command,
            Commands::Push {
                create: true,
                allow_duplicate: false,
                ..
            }
        ));
    }

    #[test]
    fn parses_push_create_allow_duplicate_flags() {
        let cli = Cli::parse_from(["ww-cli", "push", "--create", "--allow-duplicate"]);

        assert!(matches!(
            cli.command,
            Commands::Push {
                create: true,
                allow_duplicate: true,
                ..
            }
        ));
    }

    #[test]
    fn rejects_allow_duplicate_without_create() {
        // --allow-duplicate only makes sense together with --create; clap should reject
        // it on its own so the flag can't be silently ignored.
        let result = Cli::try_parse_from(["ww-cli", "push", "--allow-duplicate"]);
        assert!(result.is_err());
    }
}
