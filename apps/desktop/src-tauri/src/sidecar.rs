use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

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
    pub fn spawn() -> Result<Self, String> {
        let child = Command::new("python")
            .arg("python/ai-pipeline/main.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar: {e}"))?;

        Ok(Self { child })
    }

    pub fn send_command(
        &mut self,
        request: &SidecarRequest,
        mut on_progress: impl FnMut(&SidecarMessage),
    ) -> Result<serde_json::Value, String> {
        let stdin = self.child.stdin.as_mut().ok_or("No stdin")?;
        let request_json = serde_json::to_string(request).map_err(|e| e.to_string())?;
        writeln!(stdin, "{}", request_json).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;

        let stdout = self.child.stdout.as_mut().ok_or("No stdout")?;
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }

            let msg: SidecarMessage = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse sidecar message: {e}: {line}"))?;

            match msg.msg_type.as_str() {
                "progress" => on_progress(&msg),
                "result" => return Ok(msg.data.unwrap_or(serde_json::Value::Null)),
                "error" => return Err(msg.message.unwrap_or_else(|| "Unknown error".to_string())),
                _ => {}
            }
        }

        Err("Sidecar closed unexpectedly".to_string())
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
