#![forbid(unsafe_code)]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = std::env::args().nth(1);
    if command.as_deref() != Some("serve") {
        eprintln!("usage: stringer-mcp serve");
        std::process::exit(2);
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    stringer_mcp::serve_stdio().await
}
