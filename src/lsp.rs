mod analysis;
mod positions;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    io::{self, BufRead, Write},
};
pub const LEGEND: &[&str] = &[
    "namespace",
    "type",
    "class",
    "enum",
    "enumMember",
    "function",
    "method",
    "variable",
    "parameter",
    "property",
    "keyword",
    "string",
    "number",
    "comment",
    "operator",
    "typeParameter",
];
fn read(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                length = Some(value.trim().parse::<usize>().map_err(|e| e.to_string())?);
            }
        }
    }
    let length = length.ok_or("Missing Content-Length")?;
    if length > 32 * 1024 * 1024 {
        return Err("Message too large".into());
    }
    let mut data = vec![0; length];
    reader.read_exact(&mut data).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data)
        .map(Some)
        .map_err(|e| e.to_string())
}
fn send(writer: &mut impl Write, value: Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())
        .and_then(|_| writer.write_all(&bytes))
        .and_then(|_| writer.flush())
        .map_err(|e| e.to_string())
}
pub fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut snapshots = HashMap::<String, analysis::Snapshot>::new();
    let mut dirty = HashSet::<String>::new();
    let mut shutdown = false;
    let mut initialized = false;
    let mut refresh_tokens = false;
    let mut refresh_hints = false;
    let mut refresh_diagnostics = false;
    let mut refresh_id = 0u64;
    while let Some(message) = read(&mut reader)? {
        let id = message.get("id").cloned();
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        if message.get("method").is_none() {
            continue;
        }
        if method == "exit" {
            return if shutdown {
                Ok(())
            } else {
                Err("exit before shutdown".into())
            };
        }
        if method == "initialize" {
            initialized = true;
            refresh_diagnostics =
                params["capabilities"]["workspace"]["diagnostics"]["refreshSupport"]
                    .as_bool()
                    .unwrap_or(false);
            refresh_tokens =
                params["capabilities"]["workspace"]["semanticTokens"]["refreshSupport"]
                    .as_bool()
                    .unwrap_or(false);
            refresh_hints = params["capabilities"]["workspace"]["inlayHint"]["refreshSupport"]
                .as_bool()
                .unwrap_or(false);
            if let Some(id) = id {
                send(
                    &mut writer,
                    json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{"positionEncoding":"utf-16","textDocumentSync":{"openClose":true,"change":1,"save":true},"hoverProvider":true,"diagnosticProvider":{"identifier":"mlc","interFileDependencies":true,"workspaceDiagnostics":false},"definitionProvider":true,"inlayHintProvider":true,"semanticTokensProvider":{"legend":{"tokenTypes":LEGEND,"tokenModifiers":[]},"full":true}},"serverInfo":{"name":"mlc-lsp","version":env!("CARGO_PKG_VERSION")}}}),
                )?;
            }
            continue;
        }
        if method == "shutdown" {
            shutdown = true;
            if let Some(id) = id {
                send(&mut writer, json!({"jsonrpc":"2.0","id":id,"result":null}))?;
            }
            continue;
        }
        if method == "textDocument/didChange" {
            if let Some(uri) = params["textDocument"]["uri"].as_str() {
                dirty.insert(uri.into());
            }
            continue;
        }
        if matches!(method, "textDocument/didSave" | "textDocument/didClose") {
            snapshots.clear();
            for (enabled, method) in [
                (refresh_tokens, "workspace/semanticTokens/refresh"),
                (refresh_hints, "workspace/inlayHint/refresh"),
                (refresh_diagnostics, "workspace/diagnostic/refresh"),
            ] {
                if enabled {
                    refresh_id += 1;
                    send(
                        &mut writer,
                        json!({"jsonrpc":"2.0","id":format!("mlc-refresh-{refresh_id}"),"method":method}),
                    )?;
                }
            }
            if let Some(uri) = params["textDocument"]["uri"].as_str() {
                dirty.remove(uri);
            }
            continue;
        }
        if method == "textDocument/didOpen" {
            if let Some(uri) = params["textDocument"]["uri"].as_str() {
                if let Some(path) = positions::path(uri) {
                    let saved = std::fs::read_to_string(path).ok();
                    if saved.as_deref() != params["textDocument"]["text"].as_str() {
                        dirty.insert(uri.into());
                    }
                }
            }
            continue;
        }
        let Some(id) = id else {
            continue;
        };
        if !initialized || shutdown {
            send(
                &mut writer,
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32002,"message":"Server not initialized or shut down"}}),
            )?;
            continue;
        }
        if !matches!(
            method,
            "textDocument/diagnostic"
                | "textDocument/hover"
                | "textDocument/definition"
                | "textDocument/inlayHint"
                | "textDocument/semanticTokens/full"
        ) {
            send(
                &mut writer,
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not supported"}}),
            )?;
            continue;
        }
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let mut analysis_error = None;
        if !dirty.contains(uri) && !snapshots.get(uri).is_some_and(|s| s.fresh()) {
            snapshots.remove(uri);
            if let Some(path) = positions::path(uri) {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    analysis::analyze(&path)
                }))
                .unwrap_or_else(|_| Err("Semantic analysis failed".into()));
                match result {
                    Ok(snapshot) => {
                        snapshots.insert(uri.into(), snapshot);
                    }
                    Err(report) => analysis_error = Some(report),
                }
            }
        }
        let snapshot = snapshots
            .get(uri)
            .filter(|s| !dirty.contains(uri) && !s.affected(&dirty));
        let result = match snapshot {
            Some(snapshot) => snapshot.query(method, params),
            None => match method {
                "textDocument/diagnostic" => analysis_error
                    .as_ref()
                    .and_then(|r| {
                        positions::path(uri).map(|p| {
                            analysis::diagnostic_report(
                                r,
                                &p,
                                &std::fs::read_to_string(&p).unwrap_or_default(),
                            )
                        })
                    })
                    .unwrap_or_else(|| json!({"kind":"full","items":[]})),
                "textDocument/semanticTokens/full" => json!({"data":[]}),
                "textDocument/inlayHint" => json!([]),
                _ => Value::Null,
            },
        };
        send(
            &mut writer,
            json!({"jsonrpc":"2.0","id":id,"result":result}),
        )?;
    }
    Ok(())
}
