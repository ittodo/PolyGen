use std::fs;
use std::path::Path;
use std::process::Command;

fn get_polygen_path() -> String {
    // In bundled app, the binary is in the resources folder
    // During development, use the built binary from parent project
    if cfg!(debug_assertions) {
        // Development: use the binary built by cargo
        // Path is relative from gui/src-tauri/ to project root
        if cfg!(target_os = "windows") {
            "../../target/release/polygen.exe".to_string()
        } else {
            "../../target/release/polygen".to_string()
        }
    } else {
        // Production: bundled binary (in resources folder)
        "polygen".to_string()
    }
}

#[tauri::command]
pub fn run_generate(
    schema_path: String,
    lang: String,
    output_dir: String,
    templates_dir: Option<String>,
) -> Result<String, String> {
    let polygen = get_polygen_path();

    let mut cmd = Command::new(&polygen);
    cmd.arg("generate")
        .arg("--schema-path")
        .arg(&schema_path)
        .arg("--lang")
        .arg(&lang)
        .arg("--output-dir")
        .arg(&output_dir);

    if let Some(templates) = templates_dir {
        cmd.arg("--templates-dir").arg(&templates);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("Generated successfully. {}", stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("{}{}", stdout, stderr))
    }
}

#[tauri::command]
pub fn run_migrate(
    baseline_path: String,
    schema_path: String,
    output_dir: String,
) -> Result<String, String> {
    let polygen = get_polygen_path();

    let output = Command::new(&polygen)
        .arg("migrate")
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--schema-path")
        .arg(&schema_path)
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("Migration generated. {}", stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.to_string())
    }
}

#[tauri::command]
pub fn get_polygen_version() -> Result<String, String> {
    let polygen = get_polygen_path();

    let output = Command::new(&polygen)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    } else {
        Err("Failed to get version".to_string())
    }
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Parse import statements from a .poly file and return absolute paths
#[tauri::command]
pub fn parse_imports(file_path: String) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let base_dir = Path::new(&file_path)
        .parent()
        .ok_or("Invalid file path")?;

    let mut imports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            // Parse: import "path/to/file.poly";
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed.rfind('"') {
                    if start < end {
                        let import_path = &trimmed[start + 1..end];
                        let absolute_path = base_dir.join(import_path);
                        if absolute_path.exists() {
                            if let Some(path_str) = absolute_path.to_str() {
                                imports.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(imports)
}
