use anyhow::Result;

pub fn run(
    _from_version: String,
    _to_version: String,
    _percentage: u8,
    _app_id: Option<String>,
) -> Result<()> {
    println!(
        "Migrate command is not yet implemented.\n\
         The backend does not yet expose a migration endpoint.\n\
         Use 'whippy-workflows-cli versions' to list versions and 'whippy-workflows-cli promote' to promote a version."
    );
    Ok(())
}
