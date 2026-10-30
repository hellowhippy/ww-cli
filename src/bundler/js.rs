use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn bundle(source_dir: &PathBuf, build_dir: &PathBuf) -> Result<PathBuf> {
    // Ensure build directory exists
    fs::create_dir_all(build_dir)
        .with_context(|| format!("Failed to create build directory: {}", build_dir.display()))?;

    // Check for Node.js
    check_node_available()?;

    // Install dependencies if package.json exists
    let package_json_path = source_dir.join("package.json");
    if package_json_path.exists() {
        install_dependencies(source_dir)?;
    }

    // Bundle the code with esbuild
    let bundled_path = bundle_with_esbuild(source_dir, build_dir)?;

    Ok(bundled_path)
}

fn check_node_available() -> Result<()> {
    let output = Command::new("node").arg("--version").output().context(
        "Node.js is not installed or not in PATH. Please install Node.js to use the build command.",
    )?;

    if !output.status.success() {
        anyhow::bail!("Node.js check failed");
    }

    Ok(())
}

fn install_dependencies(source_dir: &PathBuf) -> Result<()> {
    println!("Installing dependencies...");

    // Check if esbuild is in package.json, if not add it
    let package_json_path = source_dir.join("package.json");
    if package_json_path.exists() {
        let content = std::fs::read_to_string(&package_json_path)?;
        let mut package_json: serde_json::Value = serde_json::from_str(&content)?;

        // Check if esbuild is already in devDependencies
        let needs_esbuild = !package_json["devDependencies"]
            .as_object()
            .and_then(|deps| deps.get("esbuild"))
            .is_some();

        if needs_esbuild {
            // Ensure devDependencies exists
            if !package_json["devDependencies"].is_object() {
                package_json["devDependencies"] = serde_json::json!({});
            }

            // Add esbuild
            if let Some(deps) = package_json["devDependencies"].as_object_mut() {
                deps.insert(
                    "esbuild".to_string(),
                    serde_json::Value::String("latest".to_string()),
                );
            }

            std::fs::write(
                &package_json_path,
                serde_json::to_string_pretty(&package_json)?,
            )?;
            println!("  Added esbuild to devDependencies");
        }
    }

    let status = Command::new("npm")
        .arg("install")
        .current_dir(source_dir)
        .status()
        .context("Failed to run npm install")?;

    if !status.success() {
        anyhow::bail!("npm install failed");
    }

    println!("✓ Dependencies installed");
    Ok(())
}

fn bundle_with_esbuild(source_dir: &PathBuf, build_dir: &PathBuf) -> Result<PathBuf> {
    println!("Bundling code with esbuild...");

    // Check if esbuild is available (either globally or in node_modules)
    let (esbuild_cmd, use_npx) = find_esbuild(source_dir)?;

    let index_path = source_dir.join("src").join("index.ts");
    let index_js_path = source_dir.join("src").join("index.js");

    let entry_point = if index_path.exists() {
        index_path
    } else if index_js_path.exists() {
        index_js_path
    } else {
        anyhow::bail!("No index.ts or index.js found in src/ directory");
    };

    let output_path = build_dir.join("app.js");

    // Use absolute paths for reliability
    // Ensure build directory exists first
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create build directory")?;
    }

    let output_abs = output_path
        .canonicalize()
        .or_else(|_| {
            // If canonicalize fails, use the path as-is (it will be created)
            Ok::<PathBuf, std::io::Error>(output_path.clone())
        })?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid output path encoding"))?
        .to_string();

    let entry_abs = entry_point
        .canonicalize()
        .with_context(|| {
            format!(
                "Failed to canonicalize entry point: {}",
                entry_point.display()
            )
        })?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid entry path encoding"))?
        .to_string();

    // Run esbuild
    // When using npx, we need to pass --yes to npx, then esbuild, then esbuild args
    let mut cmd = if use_npx {
        Command::new("npx")
    } else {
        let mut c = Command::new(&esbuild_cmd);
        // For direct esbuild, add entry point first
        c.arg(&entry_abs);
        c
    };

    if use_npx {
        cmd.arg("--yes").arg("esbuild").arg(&entry_abs);
    }

    // Use --outfile=path format (with equals sign) which is more reliable
    let outfile_flag = format!("--outfile={}", output_abs);

    cmd.arg("--bundle")
        .arg("--format=esm")
        .arg("--platform=neutral")
        .arg("--main-fields=main,module")
        .arg("--target=es2022")
        .arg(&outfile_flag)
        .arg("--external:fs") // fs is not available in Workers
        .arg("--external:path")
        .arg("--external:os")
        // `module` is Node-only; some libs (e.g. quickjs-emscripten mjs variant)
        // dynamic-import it behind a `process.versions.node` guard. esbuild
        // still resolves it statically, so mark external to keep the bundle
        // from failing. The branch is dead code in CF Workers.
        .arg("--external:module")
        // CF Workers' `nodejs_compat` flag polyfills `process`, so libraries
        // that detect Node via `typeof process.versions.node == "string"`
        // wrongly take the Node code path (and then fail because
        // `import.meta.url` is undefined in workerd). Force the detection to
        // be undefined so esbuild tree-shakes the Node branch entirely.
        .arg("--define:process.versions.node=undefined")
        // Real Cloudflare Workers expose `self.importScripts` (so the
        // Emscripten "web worker" branch fires) but `self.location` is
        // undefined — the asmjs/wasm glue then crashes reading
        // `self.location.href`. Local `wrangler dev` happens to provide a
        // location, hiding the bug. Inject a tiny polyfill at the top of the
        // bundle so `self.location.href` is always defined before any
        // Emscripten init runs.
        .arg("--banner:js=if(typeof self!=='undefined'&&!self.location){try{Object.defineProperty(self,'location',{value:{href:''},configurable:true});}catch(_){}}");

    // If TypeScript, add TypeScript support
    if entry_point.extension().and_then(|e| e.to_str()) == Some("ts") {
        cmd.arg("--loader:.ts=ts");
    }

    let output = cmd.current_dir(source_dir).output().with_context(|| {
        format!("Failed to run esbuild. Make sure esbuild is installed (npm install -D esbuild)")
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("esbuild stderr: {}", stderr);
        eprintln!("esbuild stdout: {}", stdout);
        anyhow::bail!("esbuild bundling failed: {}", stderr);
    }

    println!("✓ Code bundled successfully");
    Ok(output_path)
}

fn find_esbuild(source_dir: &PathBuf) -> Result<(String, bool)> {
    // First check for local esbuild in node_modules
    let local_esbuild = source_dir.join("node_modules").join(".bin").join("esbuild");

    if local_esbuild.exists() {
        return Ok((local_esbuild.to_str().unwrap().to_string(), false));
    }

    // Check for global esbuild
    let output = Command::new("which").arg("esbuild").output();

    if let Ok(output) = output {
        if output.status.success() {
            let path = String::from_utf8(output.stdout)?.trim().to_string();
            return Ok((path, false));
        }
    }

    // Try npx esbuild
    let output = Command::new("npx")
        .arg("--yes")
        .arg("esbuild")
        .arg("--version")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Ok(("esbuild".to_string(), true));
        }
    }

    anyhow::bail!("esbuild not found. Please install it: npm install -D esbuild")
}
