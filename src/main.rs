mod mcp;

use mcp::*;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use anyhow::Result;
use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};

fn main() -> Result<()> {
    // We use a synchronous loop for stdio reading to be simple and lightweight.
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut enigo = Enigo::new();


    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
            let id = req.id.clone().unwrap_or(Value::Null);
            let mut response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: None,
                error: None,
            };

            match req.method.as_str() {
                "initialize" => {
                    response.result = Some(json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "computa-use",
                            "version": "0.1.0"
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }));
                }
                "tools/list" => {
                    response.result = Some(json!({
                        "tools": [
                            {
                                "name": "execute_command",
                                "description": "Execute a shell command (PowerShell).",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "command": { "type": "string" }
                                    },
                                    "required": ["command"]
                                }
                            },
                            {
                                "name": "mouse_move",
                                "description": "Move the mouse to absolute coordinates.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "x": { "type": "integer" },
                                        "y": { "type": "integer" }
                                    },
                                    "required": ["x", "y"]
                                }
                            },
                            {
                                "name": "mouse_click",
                                "description": "Click a mouse button (left, right, middle).",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "button": { "type": "string" }
                                    },
                                    "required": ["button"]
                                }
                            },
                            {
                                "name": "keyboard_type",
                                "description": "Type a string of text.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "text": { "type": "string" }
                                    },
                                    "required": ["text"]
                                }
                            },
                            {
                                "name": "keyboard_press",
                                "description": "Press a specific key (e.g., return, space, escape, backspace, tab, media_play_pause, media_next_track).",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "key": { "type": "string" }
                                    },
                                    "required": ["key"]
                                }
                            },
                            {
                                "name": "get_system_metrics",
                                "description": "Get CPU and RAM metrics.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            }
                        ]
                    }));
                }
                "tools/call" => {
                    let params = req.params.unwrap_or(Value::Null);
                    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                    let result = handle_tool_call(&mut enigo, tool_name, args);
                    match result {
                        Ok(res) => response.result = Some(json!(res)),
                        Err(e) => {
                            response.result = Some(json!(CallToolResult {
                                content: vec![CallToolResultContent::Text { text: format!("Error: {}", e) }],
                                isError: true,
                            }));
                        }
                    }
                }
                "notifications/initialized" => {
                    continue;
                }
                _ => {
                    // Default to acknowledging unknown methods to not crash clients
                    if req.id.is_some() {
                        response.result = Some(json!({}));
                    }
                }
            }

            if req.id.is_some() {
                let response_str = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", response_str)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

fn handle_tool_call(enigo: &mut Enigo, name: &str, args: Value) -> Result<CallToolResult> {
    let mut is_error = false;
    let mut text = String::new();

    match name {
        "execute_command" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let output = std::process::Command::new("powershell")
                .arg("-Command")
                .arg(cmd)
                .output()?;
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() {
                is_error = true;
            }
            text = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout_str, stderr_str);
        }
        "mouse_move" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            enigo.mouse_move_to(x, y);
            text = format!("Moved mouse to {}, {}", x, y);
        }
        "mouse_click" => {
            let btn_str = args.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let btn = match btn_str {
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            enigo.mouse_click(btn);
            text = format!("Clicked {} mouse button", btn_str);
        }
        "keyboard_type" => {
            let text_to_type = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            enigo.key_sequence(text_to_type);
            text = format!("Typed text");
        }
        "keyboard_press" => {
            let key_str = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let key = match key_str {
                "return" => Key::Return,
                "space" => Key::Space,
                "escape" => Key::Escape,
                "backspace" => Key::Backspace,
                "tab" => Key::Tab,
                _ => return Err(anyhow::anyhow!("Unsupported key: {}", key_str)),
            };
            enigo.key_click(key);
            text = format!("Pressed key {}", key_str);
        }
        "get_system_metrics" => {
            use sysinfo::{System, Networks, Disks};
            let mut sys = System::new_all();
            sys.refresh_all();
            
            text = format!(
                "Total RAM: {} Bytes\nUsed RAM: {} Bytes\nTotal Swap: {} Bytes\nUsed Swap: {} Bytes\nCore Count: {:?}",
                sys.total_memory(),
                sys.used_memory(),
                sys.total_swap(),
                sys.used_swap(),
                sysinfo::System::physical_core_count(),
            );
        }
        _ => {
            is_error = true;
            text = format!("Tool {} not found", name);
        }
    }

    Ok(CallToolResult {
        content: vec![CallToolResultContent::Text { text }],
        isError: is_error,
    })
}
