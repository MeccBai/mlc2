use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
}
impl Client {
    fn new() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mlc-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child,
            input,
            output,
            next: 1,
        };
        let result = client.request("initialize", json!({"capabilities":{}}));
        assert_eq!(result["capabilities"]["hoverProvider"], true);
        client
    }
    fn send(&mut self, value: Value) {
        let data = serde_json::to_vec(&value).unwrap();
        write!(self.input, "Content-Length: {}\r\n\r\n", data.len()).unwrap();
        self.input.write_all(&data).unwrap();
        self.input.flush().unwrap();
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let mut length = 0;
        loop {
            let mut line = String::new();
            assert!(self.output.read_line(&mut line).unwrap() > 0, "LSP exited");
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut bytes = vec![0; length];
        self.output.read_exact(&mut bytes).unwrap();
        let response: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response["id"], id);
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }
    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc":"2.0","method":method,"params":params}));
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn uri(path: &std::path::Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}
fn position(text: &str, name: &str) -> Value {
    let index = text.find(name).unwrap();
    let prefix = &text[..index];
    let start = prefix.rfind('\n').map_or(0, |i| i + 1);
    json!({"line":prefix.bytes().filter(|b|*b==b'\n').count(),"character":prefix[start..].encode_utf16().count()})
}
#[test]
fn saved_source_provides_hover_hints_definition_and_tokens() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    fs::write(root.path().join("types.m2"),"export unit Pair{pub value:i32;};export enum Color{Red,Blue};export union Number[i32,f64];export func add(left:i32,right:i32)->i32{return left+right;}").unwrap();
    let text = "import types;\nfunc main()->i32 {\n var pair:types::Pair=types::Pair{1};\n var color:types::Color=types::Color::Red;\n var n:types::Number=types::Number{1};\n var answer=types::add(20,22);\n return answer;\n}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let hover = client.request(
        "textDocument/hover",
        json!({"textDocument":{"uri":uri},"position":position(text,"answer=")}),
    );
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("i32"),
        "{hover}"
    );
    for name in ["Pair=", "Color=", "Number=", "add("] {
        let definition = client.request(
            "textDocument/definition",
            json!({"textDocument":{"uri":uri},"position":position(text,name)}),
        );
        assert!(
            definition["uri"].as_str().unwrap().ends_with("types.m2"),
            "{name}: {definition}"
        );
    }
    let hints=client.request("textDocument/inlayHint",json!({"textDocument":{"uri":uri},"range":{"start":{"line":0,"character":0},"end":{"line":10,"character":0}}}));
    assert!(
        hints
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["label"] == "left:"),
        "{hints}"
    );
    assert!(
        hints
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["label"] == "right:"),
        "{hints}"
    );
    let tokens = client.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument":{"uri":uri}}),
    );
    assert!(!tokens["data"].as_array().unwrap().is_empty());
    assert!(!root.path().join("build/ctfe").exists());
    assert_eq!(client.request("shutdown", Value::Null), Value::Null);
    client.notify("exit", Value::Null);
    assert!(client.child.wait().unwrap().success());
}
#[test]
fn dirty_and_invalid_files_return_no_analysis_then_recover_on_save() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "func main()->i32{var answer=42;return answer;}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let params = json!({"textDocument":{"uri":uri},"position":position(text,"answer=")});
    let mut client = Client::new();
    assert!(
        client
            .request("textDocument/hover", params.clone())
            .is_object()
    );
    client.notify(
        "textDocument/didChange",
        json!({"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"broken"}]}),
    );
    assert!(
        client
            .request("textDocument/hover", params.clone())
            .is_null()
    );
    client.notify("textDocument/didSave", json!({"textDocument":{"uri":uri}}));
    assert!(
        client
            .request("textDocument/hover", params.clone())
            .is_object()
    );
    for text in ["func main(){var x=;}", "func main(){\u{0001}}"] {
        fs::write(&path, text).unwrap();
        client.notify("textDocument/didSave", json!({"textDocument":{"uri":uri}}));
        assert!(
            client
                .request("textDocument/hover", params.clone())
                .is_null()
        );
    }
    fs::write(&path, text).unwrap();
    client.notify("textDocument/didSave", json!({"textDocument":{"uri":uri}}));
    assert!(client.request("textDocument/hover", params).is_object());
}

#[test]
fn editor_does_not_execute_ctfe_and_todo_fills_expected_types() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "const func spin()->i32{while(true){}return 1;}func missing()->f64{return todo();}func main()->i32{const answer=spin();return answer;}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let hover = client.request(
        "textDocument/hover",
        json!({"textDocument":{"uri":uri},"position":position(text,"answer=")}),
    );
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("i32"),
        "{hover}"
    );
    assert!(!root.path().join("build/ctfe").exists());
}

#[test]
fn utf16_positions_and_percent_encoded_paths_match_source_tokens() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("项目 😀");
    fs::create_dir(&dir).unwrap();
    let path = dir.join("main.m2");
    let text = "func main()->i32{/*😀*/var answer=42;return answer;}";
    fs::write(&path, text).unwrap();
    let raw = uri(&path);
    let mut uri = String::new();
    for b in raw.bytes() {
        if b.is_ascii_alphanumeric() || b"/:._-~".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    let mut client = Client::new();
    let hover = client.request(
        "textDocument/hover",
        json!({"textDocument":{"uri":uri},"position":position(text,"answer=")}),
    );
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("i32"),
        "{hover}"
    );
}
