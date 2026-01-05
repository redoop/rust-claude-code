use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rust-claude-code")]
#[command(about = "A Rust implementation of Claude Code CLI", long_about = None)]
struct Args {
    /// Input file to process
    #[arg(short, long)]
    input: Option<String>,
}

fn main() {
    let args = Args::parse();

    println!("Rust Claude Code - Starting...");

    if let Some(input) = args.input {
        println!("Processing input file: {}", input);
    }
}
