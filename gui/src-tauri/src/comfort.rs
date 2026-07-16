//! Bridge to the bundled JUCE Voice Comfort engine.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ComfortStatus {
    pub available: bool,
    pub running: bool,
    pub input_db: f32,
    pub output_db: f32,
    pub de_ess_reduction_db: f32,
    pub soften: f32,
    pub fullness: f32,
    pub de_ess: f32,
    pub compression: f32,
    pub gain_db: f32,
    pub bypass: bool,
    pub shortcut_available: bool,
    pub shortcut_running: bool,
    pub shortcut_error: String,
    pub error: String,
}

impl Default for ComfortStatus {
    fn default() -> Self {
        Self {
            available: cfg!(target_os = "macos"),
            running: false,
            input_db: -80.0,
            output_db: -80.0,
            de_ess_reduction_db: 0.0,
            soften: 0.78,
            fullness: 0.53,
            de_ess: 0.68,
            compression: 0.58,
            gain_db: -0.5,
            bypass: false,
            shortcut_available: cfg!(target_os = "macos"),
            shortcut_running: false,
            shortcut_error: String::new(),
            error: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComfortParameters {
    pub soften: f32,
    pub fullness: f32,
    pub de_ess: f32,
    pub compression: f32,
    pub gain_db: f32,
    pub bypass: bool,
}

struct EngineProcess {
    child: Child,
    stdin: ChildStdin,
}

pub struct ComfortEngine {
    process: Mutex<Option<EngineProcess>>,
    status: Arc<Mutex<ComfortStatus>>,
}

impl Default for ComfortEngine {
    fn default() -> Self {
        Self {
            process: Mutex::new(None),
            status: Arc::new(Mutex::new(ComfortStatus::default())),
        }
    }
}

impl ComfortEngine {
    fn executable() -> PathBuf {
        if let Ok(current) = std::env::current_exe() {
            if let Some(parent) = current.parent() {
                let bundled = parent.join("voice-comfort-engine");
                if bundled.exists() {
                    return bundled;
                }
            }
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("bin/voice-comfort-engine-aarch64-apple-darwin")
    }

    fn ensure_process(&self) -> Result<(), String> {
        let mut process = self.process.lock().map_err(|_| "音频引擎状态已损坏")?;
        if let Some(active) = process.as_mut() {
            if active.child.try_wait().map_err(|e| e.to_string())?.is_none() {
                return Ok(());
            }
            *process = None;
        }

        let executable = Self::executable();
        if !executable.exists() {
            return Err(format!("未找到 Voice Comfort 引擎：{}", executable.display()));
        }
        let mut child = Command::new(&executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("无法启动 Voice Comfort 引擎：{e}"))?;
        let stdin = child.stdin.take().ok_or("无法连接音频引擎输入")?;
        let stdout = child.stdout.take().ok_or("无法连接音频引擎状态")?;
        let status = self.status.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Ok(mut next) = serde_json::from_str::<ComfortStatus>(&line) else {
                    continue;
                };
                next.available = true;
                if let Ok(mut current) = status.lock() {
                    *current = next;
                }
            }
            if let Ok(mut current) = status.lock() {
                current.running = false;
                if current.error.is_empty() {
                    current.error = "Voice Comfort 引擎已停止".into();
                }
            }
        });
        *process = Some(EngineProcess { child, stdin });
        Ok(())
    }

    fn send(&self, value: &serde_json::Value) -> Result<(), String> {
        self.ensure_process()?;
        let mut process = self.process.lock().map_err(|_| "音频引擎状态已损坏")?;
        let active = process.as_mut().ok_or("音频引擎未启动")?;
        serde_json::to_writer(&mut active.stdin, value).map_err(|e| e.to_string())?;
        active.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        active.stdin.flush().map_err(|e| e.to_string())
    }

    fn start(&self) -> Result<(), String> {
        self.send(&serde_json::json!({ "action": "start" }))
    }

    fn stop(&self) -> Result<(), String> {
        self.send(&serde_json::json!({ "action": "stop" }))
    }

    fn set(&self, parameters: ComfortParameters) -> Result<(), String> {
        self.send(&serde_json::json!({
            "action": "set",
            "soften": parameters.soften,
            "fullness": parameters.fullness,
            "deEss": parameters.de_ess,
            "compression": parameters.compression,
            "outputDb": parameters.gain_db,
            "bypass": parameters.bypass,
        }))
    }

    fn status(&self) -> ComfortStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn receiver_shortcut_start(&self) -> Result<(), String> {
        self.send(&serde_json::json!({ "action": "shortcut_start" }))
    }

    fn receiver_shortcut_stop(&self) -> Result<(), String> {
        self.send(&serde_json::json!({ "action": "shortcut_stop" }))
    }
}

#[tauri::command]
pub fn comfort_status(engine: State<'_, ComfortEngine>) -> ComfortStatus {
    engine.status()
}

#[tauri::command]
pub fn comfort_start(engine: State<'_, ComfortEngine>) -> Result<(), String> {
    engine.start()
}

#[tauri::command]
pub fn comfort_stop(engine: State<'_, ComfortEngine>) -> Result<(), String> {
    engine.stop()
}

#[tauri::command]
pub fn comfort_set(
    parameters: ComfortParameters,
    engine: State<'_, ComfortEngine>,
) -> Result<(), String> {
    engine.set(parameters)
}

#[tauri::command]
pub fn receiver_shortcut_start(engine: State<'_, ComfortEngine>) -> Result<(), String> {
    engine.receiver_shortcut_start()
}

#[tauri::command]
pub fn receiver_shortcut_stop(engine: State<'_, ComfortEngine>) -> Result<(), String> {
    engine.receiver_shortcut_stop()
}
