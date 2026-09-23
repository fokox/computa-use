mod mcp;

use mcp::*;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use anyhow::Result;
use enigo::{Enigo, Keyboard, Mouse, Coordinate, Button, Direction, Key, Settings};
use uiautomation::{UIAutomation, UIElement};
use xcap::Monitor;
use image::GenericImageView;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::io::Cursor;

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < input.len() {
        let b1 = input[i];
        let b2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b3 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        
        let out1 = b1 >> 2;
        let out2 = ((b1 & 0b00000011) << 4) | (b2 >> 4);
        let out3 = ((b2 & 0b00001111) << 2) | (b3 >> 6);
        let out4 = b3 & 0b00111111;
        
        out.push(BASE64_CHARS[out1 as usize] as char);
        out.push(BASE64_CHARS[out2 as usize] as char);
        out.push(if i + 1 < input.len() { BASE64_CHARS[out3 as usize] as char } else { '=' });
        out.push(if i + 2 < input.len() { BASE64_CHARS[out4 as usize] as char } else { '=' });
        
        i += 3;
    }
    out
}

fn main() -> Result<()> {
    // Generate session directory
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let session_dir = format!("sessions/session_{}", now);
    fs::create_dir_all(&session_dir)?;

    // We use a synchronous loop for stdio reading to be simple and lightweight.
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

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
                            },
                            {
                                "name": "get_ui_tree",
                                "description": "Get the structural tree of UI elements on the screen.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            },
                            {
                                "name": "find_image_on_screen",
                                "description": "Finds a template image on the screen and returns its center coordinates.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "template_path": { "type": "string" }
                                    },
                                    "required": ["template_path"]
                                }
                            },
                            {
                                "name": "take_screenshot",
                                "description": "Captures the entire screen, saves it to a session-specific folder, and returns the image data directly.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            },
                            {
                                "name": "perform_actions",
                                "description": "Perform multiple computer navigation actions sequentially in one call. Actions can be mouse_move (x, y), mouse_click (button), keyboard_type (text), keyboard_press (key).",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "actions": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "action": { "type": "string", "description": "One of: mouse_move, mouse_click, keyboard_type, keyboard_press" },
                                                    "x": { "type": "integer" },
                                                    "y": { "type": "integer" },
                                                    "button": { "type": "string" },
                                                    "text": { "type": "string" },
                                                    "key": { "type": "string" }
                                                },
                                                "required": ["action"]
                                            }
                                        }
                                    },
                                    "required": ["actions"]
                                }
                            }
                        ]
                    }));
                }
                "tools/call" => {
                    let params = req.params.unwrap_or(Value::Null);
                    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                    let result = handle_tool_call(&mut enigo, tool_name, args, &session_dir);
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

fn handle_tool_call(enigo: &mut Enigo, name: &str, args: Value, session_dir: &str) -> Result<CallToolResult> {
    let mut is_error = false;
    let mut text = String::new();
    let mut extra_content = Vec::new();

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
            enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| anyhow::anyhow!("Mouse error: {}", e))?;
            text = format!("Moved mouse to {}, {}", x, y);
        }
        "mouse_click" => {
            let btn_str = args.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let btn = match btn_str {
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => Button::Left,
            };
            enigo.button(btn, Direction::Click).map_err(|e| anyhow::anyhow!("Mouse error: {}", e))?;
            text = format!("Clicked {} mouse button", btn_str);
        }
        "keyboard_type" => {
            let text_to_type = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            enigo.text(text_to_type).map_err(|e| anyhow::anyhow!("Keyboard error: {}", e))?;
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
                "media_play_pause" => Key::MediaPlayPause,
                "media_next_track" => Key::MediaNextTrack,
                _ => return Err(anyhow::anyhow!("Unsupported key: {}", key_str)),
            };
            enigo.key(key, Direction::Click).map_err(|e| anyhow::anyhow!("Keyboard error: {}", e))?;
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
                sys.physical_core_count(),
            );
        }
        "get_ui_tree" => {
            text = match get_ui_tree_impl() {
                Ok(t) => t,
                Err(e) => {
                    is_error = true;
                    format!("Error: {}", e)
                }
            };
        }
        "find_image_on_screen" => {
            let path = args.get("template_path").and_then(|v| v.as_str()).unwrap_or("");
            text = match find_template_impl(path) {
                Ok((x, y)) => format!("Template found at center coordinates: x={}, y={}", x, y),
                Err(e) => {
                    is_error = true;
                    format!("Error: {}", e)
                }
            };
        }
        "take_screenshot" => {
            let monitors = Monitor::all().map_err(|e| anyhow::anyhow!("Monitor error: {}", e))?;
            let monitor = monitors.first().ok_or_else(|| anyhow::anyhow!("No monitor found"))?;
            let image = monitor.capture_image().map_err(|e| anyhow::anyhow!("Capture error: {}", e))?;
            
            let file_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let file_path = format!("{}/screenshot_{}.png", session_dir, file_time);
            image.save(&file_path).map_err(|e| anyhow::anyhow!("Save error: {}", e))?;
            
            let mut buffer = Cursor::new(Vec::new());
            image.write_to(&mut buffer, image::ImageFormat::Png).map_err(|e| anyhow::anyhow!("Encode error: {}", e))?;
            let base64_data = encode_base64(&buffer.into_inner());

            text = format!("Saved screenshot to {}", file_path);
            extra_content.push(CallToolResultContent::Image { data: base64_data, mimeType: "image/png".to_string() });
        }
        "perform_actions" => {
            let mut results = Vec::new();
            if let Some(actions) = args.get("actions").and_then(|v| v.as_array()) {
                for act in actions {
                    let act_type = act.get("action").and_then(|v| v.as_str()).unwrap_or("");
                    match act_type {
                        "mouse_move" => {
                            let x = act.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let y = act.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            if let Err(e) = enigo.move_mouse(x, y, Coordinate::Abs) {
                                results.push(format!("Error mouse_move: {}", e));
                            } else {
                                results.push(format!("mouse_move({}, {})", x, y));
                            }
                        }
                        "mouse_click" => {
                            let btn_str = act.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                            let btn = match btn_str {
                                "right" => Button::Right,
                                "middle" => Button::Middle,
                                _ => Button::Left,
                            };
                            if let Err(e) = enigo.button(btn, Direction::Click) {
                                results.push(format!("Error mouse_click: {}", e));
                            } else {
                                results.push(format!("mouse_click({})", btn_str));
                            }
                        }
                        "keyboard_type" => {
                            let text_to_type = act.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            if let Err(e) = enigo.text(text_to_type) {
                                results.push(format!("Error keyboard_type: {}", e));
                            } else {
                                results.push(format!("keyboard_type(\"{}\")", text_to_type));
                            }
                        }
                        "keyboard_press" => {
                            let key_str = act.get("key").and_then(|v| v.as_str()).unwrap_or("");
                            let key = match key_str {
                                "return" => Key::Return,
                                "space" => Key::Space,
                                "escape" => Key::Escape,
                                "backspace" => Key::Backspace,
                                "tab" => Key::Tab,
                                "media_play_pause" => Key::MediaPlayPause,
                                "media_next_track" => Key::MediaNextTrack,
                                _ => Key::Return, // Default fallback
                            };
                            if let Err(e) = enigo.key(key, Direction::Click) {
                                results.push(format!("Error keyboard_press: {}", e));
                            } else {
                                results.push(format!("keyboard_press({})", key_str));
                            }
                        }
                        _ => {
                            results.push(format!("Unknown action: {}", act_type));
                        }
                    }
                }
                text = results.join("\n");
            } else {
                is_error = true;
                text = "Missing or invalid 'actions' array".to_string();
            }
        }
        _ => {
            is_error = true;
            text = format!("Tool {} not found", name);
        }
    }

    let mut content = vec![CallToolResultContent::Text { text }];
    content.extend(extra_content);

    Ok(CallToolResult {
        content,
        isError: is_error,
    })
}

fn get_ui_tree_impl() -> Result<String> {
    let automation = UIAutomation::new()?;
    let root = automation.get_root_element()?;
    Ok(print_tree(&automation, &root, 0, 3))
}

fn print_tree(automation: &UIAutomation, element: &UIElement, depth: usize, max_depth: usize) -> String {
    if depth > max_depth {
        return String::new();
    }
    let mut s = String::new();
    let name = element.get_name().unwrap_or_default();
    let class = element.get_classname().unwrap_or_default();
    let rect_str = if let Ok(rect) = element.get_bounding_rectangle() {
        format!("[{}, {}, {}, {}]", rect.get_left(), rect.get_top(), rect.get_right(), rect.get_bottom())
    } else {
        String::new()
    };
    
    let indent = "  ".repeat(depth);
    if !name.is_empty() || !class.is_empty() {
        s.push_str(&format!("{}{} (Class: {}) {}\n", indent, name, class, rect_str));
    }
    
    if let Ok(walker) = automation.get_control_view_walker() {
        if let Ok(child) = walker.get_first_child(element) {
            let mut curr = Some(child);
            while let Some(c) = curr {
                s.push_str(&print_tree(automation, &c, depth + 1, max_depth));
                curr = walker.get_next_sibling(&c).ok();
            }
        }
    }
    s
}

fn find_template_impl(template_path: &str) -> Result<(u32, u32)> {
    let monitors = Monitor::all().map_err(|e| anyhow::anyhow!("Monitor error: {}", e))?;
    let monitor = monitors.first().ok_or_else(|| anyhow::anyhow!("No monitor"))?;
    let monitor_image = monitor.capture_image().map_err(|e| anyhow::anyhow!("Capture error: {}", e))?;
    let template = image::open(template_path)?.to_rgba8();
    
    let (t_w, t_h) = template.dimensions();
    let (m_w, m_h) = monitor_image.dimensions();
    
    if t_w > m_w || t_h > m_h {
        return Err(anyhow::anyhow!("Template larger than monitor"));
    }
    
    for y in (0..=(m_h - t_h)).step_by(2) {
        for x in (0..=(m_w - t_w)).step_by(2) {
            let mut found = true;
            let points = [(0,0), (t_w-1, 0), (0, t_h-1), (t_w-1, t_h-1), (t_w/2, t_h/2)];
            for (tx, ty) in points {
                let m_pixel = monitor_image.get_pixel(x + tx, y + ty);
                let t_pixel = template.get_pixel(tx, ty);
                if m_pixel[0] != t_pixel[0] || m_pixel[1] != t_pixel[1] || m_pixel[2] != t_pixel[2] {
                    found = false;
                    break;
                }
            }
            if found {
                for ty in (0..t_h).step_by(3) {
                    for tx in (0..t_w).step_by(3) {
                        let m_pixel = monitor_image.get_pixel(x + tx, y + ty);
                        let t_pixel = template.get_pixel(tx, ty);
                        if (m_pixel[0] as i32 - t_pixel[0] as i32).abs() > 10 || 
                           (m_pixel[1] as i32 - t_pixel[1] as i32).abs() > 10 || 
                           (m_pixel[2] as i32 - t_pixel[2] as i32).abs() > 10 {
                            found = false;
                            break;
                        }
                    }
                    if !found { break; }
                }
                if found {
                    return Ok((x + t_w / 2, y + t_h / 2));
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Template not found"))
}
