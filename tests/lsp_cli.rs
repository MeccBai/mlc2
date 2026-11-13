mod support;
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

fn hover_value(client: &mut Client, uri: &str, text: &str, needle: &str) -> String {
    let value = client.request(
        "textDocument/hover",
        json!({"textDocument":{"uri":uri},"position":position(text,needle)}),
    );
    value["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("Missing hover for {needle}: {value}"))
        .to_string()
}
#[test]
fn display_short_types_const_bindings_and_complete_function_signatures() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    fs::write(
        root.path().join("types.m2"),
        "export unit Point{pub x:i32;};",
    )
    .unwrap();
    let text = "import types;func magnitude(point:types::Point)->i32{return point.x;}func main()->i32{const fixed=50;val immutable=50;var p:types::Point=types::Point{1};var array:[types::Point:1]=[p];return magnitude(p)+fixed+immutable;}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    assert_eq!(hover_value(&mut client, &uri, text, "fixed="), "const i32");
    assert_eq!(hover_value(&mut client, &uri, text, "fixed+"), "const i32");
    assert_eq!(hover_value(&mut client, &uri, text, "immutable="), "i32");
    assert_eq!(hover_value(&mut client, &uri, text, "immutable;}"), "i32");
    assert_eq!(hover_value(&mut client, &uri, text, "p:types"), "Point");
    let array = hover_value(&mut client, &uri, text, "array:");
    assert!(
        array.contains("Point") && !array.contains("types::"),
        "{array}"
    );
    assert_eq!(
        hover_value(&mut client, &uri, text, "magnitude(point"),
        "func magnitude(point: Point) -> i32"
    );
    assert_eq!(
        hover_value(&mut client, &uri, text, "magnitude(p)"),
        "func magnitude(point: Point) -> i32"
    );
}
#[test]
fn semantic_errors_return_diagnostics_without_discarding_all_results() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "func helper()->i32{return 1;}func main()->i32{var good=50;return unknown;}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let diagnostics = client.request(
        "textDocument/diagnostic",
        json!({"textDocument":{"uri":uri}}),
    );
    assert_eq!(diagnostics["kind"], "full");
    assert!(
        diagnostics["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == 1),
        "{diagnostics}"
    );
    assert_eq!(hover_value(&mut client, &uri, text, "good="), "i32");
    assert_eq!(
        hover_value(&mut client, &uri, text, "helper()"),
        "func helper() -> i32"
    );
    let fixed = text.replace("unknown", "good");
    fs::write(&path, &fixed).unwrap();
    client.notify("textDocument/didSave", json!({"textDocument":{"uri":uri}}));
    let clean = client.request(
        "textDocument/diagnostic",
        json!({"textDocument":{"uri":uri}}),
    );
    assert!(
        !clean["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == 1),
        "{clean}"
    );
    fs::write(&path, "unit A{};unit A{};func main(){}").unwrap();
    client.notify("textDocument/didSave", json!({"textDocument":{"uri":uri}}));
    let duplicate = client.request(
        "textDocument/diagnostic",
        json!({"textDocument":{"uri":uri}}),
    );
    assert!(
        duplicate["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == 1),
        "{duplicate}"
    );
}
#[test]
fn println_uses_source_macro_parameter_names_and_signature() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    fs::write(root.path().join("Project.toml"),format!("[Project]\nName='lsp'\nVersion='0.1.0'\nOutputDir='build'\nTargets=[]\nLibDirs=['{}']\n",support::library().display())).unwrap();
    let text = "import std;using println=std::println;func main(){println!(\"value={}\",50);}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let hover = hover_value(&mut client, &uri, text, "println!(");
    assert!(hover.starts_with("macro func println!(str: str"), "{hover}");
    let hints=client.request("textDocument/inlayHint",json!({"textDocument":{"uri":uri},"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":text.len()}}}));
    assert!(
        hints
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["label"] == "str:"),
        "{hints}"
    );
    assert!(
        !hints.to_string().contains("__macro_arg") && !hints.to_string().contains("arg0:"),
        "{hints}"
    );
}

#[test]
fn semantic_tokens_distinguish_declarations_parameters_and_using_aliases() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "unit Point{pub x:i32;};enum Color{Red,Blue};union Number[i32,f64];using P=Point;using callback=distance;func distance(point:P)->i32{return point.x;}func<T> identity(item:T)->T{return item;}func main()->i32{var p=P{1};return callback(p);}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let result = client.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument":{"uri":uri}}),
    );
    let data = result["data"].as_array().unwrap();
    assert!(!data.is_empty(), "{result}");
    let legend = [
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
    let mut tokens = vec![];
    let (mut line, mut column) = (0u64, 0u64);
    for item in data.chunks_exact(5) {
        let delta = item[0].as_u64().unwrap();
        line += delta;
        column = if delta == 0 {
            column + item[1].as_u64().unwrap()
        } else {
            item[1].as_u64().unwrap()
        };
        tokens.push((
            line,
            column,
            item[2].as_u64().unwrap(),
            legend[item[3].as_u64().unwrap() as usize],
        ));
    }
    for (needle, expected) in [
        ("Point{", "class"),
        ("Color{", "enum"),
        ("Red,", "enumMember"),
        ("Number[", "type"),
        ("P=Point", "class"),
        ("callback=", "function"),
        ("distance(point", "function"),
        ("point:P", "parameter"),
        ("point.x", "parameter"),
        ("x:i32", "property"),
        ("identity(item", "function"),
        ("item:T", "parameter"),
        ("item;", "parameter"),
        ("T> identity", "typeParameter"),
        ("T)->T", "typeParameter"),
        ("callback(p)", "function"),
        ("p=P", "variable"),
        ("P{1}", "class"),
    ] {
        let position = position(text, needle);
        let token = tokens
            .iter()
            .find(|t| {
                t.0 == position["line"].as_u64().unwrap()
                    && t.1 == position["character"].as_u64().unwrap()
            })
            .unwrap_or_else(|| panic!("Missing token for {needle}: {result}"));
        assert_eq!(token.3, expected, "{needle}");
    }
}

#[test]
fn implicit_self_receivers_are_parameters_at_declarations_and_references() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "export unit Point{pub x:i32;};api Point::func sq(mut self)->i32{return self->x*self->x;}api Point::func get(self)->i32{return self->x;}api Point::func<T> templated(mut self,value:T)->i32{return self->x;}func main(){}";
    fs::write(&path, text).unwrap();
    let uri = uri(&path);
    let mut client = Client::new();
    let response = client.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument":{"uri":uri}}),
    );
    let data = response["data"].as_array().unwrap();
    assert!(!data.is_empty(), "{response}");
    let mut tokens = vec![];
    let (mut line, mut column) = (0u64, 0u64);
    for item in data.chunks_exact(5) {
        let delta = item[0].as_u64().unwrap();
        line += delta;
        column = if delta == 0 {
            column + item[1].as_u64().unwrap()
        } else {
            item[1].as_u64().unwrap()
        };
        tokens.push((line, column, item[3].as_u64().unwrap()));
    }
    for (offset, _) in text.match_indices("self") {
        let token = tokens
            .iter()
            .find(|t| t.0 == 0 && t.1 == offset as u64)
            .unwrap_or_else(|| panic!("Missing self token at {offset}"));
        assert_eq!(
            token.2, 8,
            "self at {offset} must be parameter, not variable"
        );
    }
    let reference = text.find("return self").unwrap() + 7;
    let definition = client.request(
        "textDocument/definition",
        json!({"textDocument":{"uri":uri},"position":{"line":0,"character":reference}}),
    );
    assert_eq!(
        definition["range"]["start"]["character"],
        text.find("self").unwrap() as u64
    );
}
