use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tauri::AppHandle;

#[derive(Debug, Serialize)]
pub struct SidecarRequest {
    pub command: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SidecarMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub step: String,
    #[serde(default)]
    pub percent: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

pub struct Sidecar {
    child: Child,
}

impl Sidecar {
    pub fn spawn(app: &AppHandle) -> Result<Self, String> {
        let child = if cfg!(debug_assertions) {
            Self::spawn_dev()?
        } else {
            Self::spawn_prod(app)?
        };

        log::info!("Sidecar spawned with PID {}", child.id());
        Ok(Self { child })
    }

    /// Dev: run Python source directly from the monorepo
    fn spawn_dev() -> Result<Child, String> {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project root: {e}"))?;

        log::info!("Dev sidecar from {}", project_root.display());

        Command::new("python")
            .arg(project_root.join("python/ai-pipeline/main.py"))
            .current_dir(&project_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar: {e}"))
    }

    /// Prod: run bundled PyInstaller binary from Tauri resource dir
    fn spawn_prod(app: &AppHandle) -> Result<Child, String> {
        use tauri::Manager;

        let bin_name = if cfg!(target_os = "windows") {
            "ai-pipeline.exe"
        } else {
            "ai-pipeline"
        };

        let bin_path = app
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {e}"))?
            .join("binaries")
            .join(bin_name);

        log::info!("Prod sidecar from {}", bin_path.display());

        Command::new(&bin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar at {}: {e}", bin_path.display()))
    }

    pub fn send_command(
        &mut self,
        request: &SidecarRequest,
        mut on_progress: impl FnMut(&SidecarMessage),
    ) -> Result<serde_json::Value, String> {
        log::info!("Sending command to sidecar: {}", request.command);

        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        let request_json = serde_json::to_string(request).map_err(|e| e.to_string())?;
        writeln!(stdin, "{}", request_json).map_err(|e| format!("Failed to write to sidecar stdin: {e}"))?;
        stdin.flush().map_err(|e| format!("Failed to flush sidecar stdin: {e}"))?;

        let stdout = self.child.stdout.as_mut().ok_or("No stdout")?;
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read sidecar stdout: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }

            log::debug!("Sidecar stdout: {}", line);

            let msg: SidecarMessage = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse sidecar message: {e}\nRaw line: {line}"))?;

            match msg.msg_type.as_str() {
                "progress" => on_progress(&msg),
                "result" => {
                    log::info!("Sidecar command '{}' completed successfully", request.command);
                    return Ok(msg.data.unwrap_or(serde_json::Value::Null));
                }
                "error" => {
                    let err_msg = msg.message.unwrap_or_else(|| "Unknown error".to_string());
                    log::error!("Sidecar command '{}' failed: {}", request.command, err_msg);
                    return Err(err_msg);
                }
                other => {
                    log::warn!("Unknown sidecar message type: {}", other);
                }
            }
        }

        // stdout closed — process likely crashed. Read stderr for details.
        let stderr_output = self.read_stderr();
        let exit_status = self.child.wait().ok().map(|s| format!("{s}"));

        let mut err = format!("Sidecar closed unexpectedly during '{}'", request.command);
        if let Some(status) = exit_status {
            err.push_str(&format!(" (exit: {status})"));
        }
        if !stderr_output.is_empty() {
            err.push_str(&format!("\n\nstderr:\n{stderr_output}"));
        }

        log::error!("{}", err);
        Err(err)
    }

    fn read_stderr(&mut self) -> String {
        let Some(stderr) = self.child.stderr.as_mut() else {
            return String::new();
        };
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        buf.trim().to_string()
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
