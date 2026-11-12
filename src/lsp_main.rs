//! Saved-file language service. stdout is reserved for framed JSON-RPC.
mod lsp;
fn main() {
    if let Err(error) = lsp::run() {
        eprintln!("mlc-lsp: {error}");
        std::process::exit(1);
    }
}
