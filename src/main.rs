use bittwiddle::{
    Message,
    expression::{self, IntegerWidth},
    model::{BitTwiddleModel, DisplayMode},
    update,
    view::view,
};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
        execute,
    },
    layout::Rect,
};
use std::io;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version") {
        println!(
            "{} version {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        println!("Author: {}", env!("CARGO_PKG_AUTHORS"));
        println!("License: {}", env!("CARGO_PKG_LICENSE"));
        return;
    }

    match parse_cli_args(&args) {
        CliParseResult::Help => print_help(),
        CliParseResult::Error(msg) => {
            eprintln!("bittwiddle: error: {}", msg);
            std::process::exit(1);
        }
        CliParseResult::Eval { width, expression } => {
            if let Err(e) = evaluate_expression(&expression, width) {
                eprintln!("bittwiddle: error: {}", e);
                std::process::exit(1);
            }
        }
        CliParseResult::Tui => {
            if let Err(e) = run_tui() {
                eprintln!("bittwiddle: error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

enum CliParseResult {
    Help,
    Error(String),
    Eval {
        width: IntegerWidth,
        expression: String,
    },
    Tui,
}

fn parse_cli_args(args: &[String]) -> CliParseResult {
    let mut expecting_width = false;
    let mut width = IntegerWidth::default();
    let mut expression_parts = Vec::new();

    for arg in args.iter().skip(1) {
        if expecting_width {
            match arg.as_str() {
                "8" => width = IntegerWidth::Eight,
                "16" => width = IntegerWidth::Sixteen,
                "32" => width = IntegerWidth::ThirtyTwo,
                "64" => width = IntegerWidth::SixtyFour,
                _ => {
                    return CliParseResult::Error(format!(
                        "invalid width: {} (expected 8, 16, 32, or 64)",
                        arg
                    ));
                }
            }
            expecting_width = false;
            continue;
        }

        match arg.as_str() {
            "--help" | "-h" => return CliParseResult::Help,
            "--width" | "-w" => expecting_width = true,
            other => expression_parts.push(other.to_string()),
        }
    }

    if expecting_width {
        return CliParseResult::Error("missing value for --width".into());
    }

    if expression_parts.is_empty() {
        if args.len() > 1 {
            return CliParseResult::Error("missing expression".into());
        }
        return CliParseResult::Tui;
    }

    let expression = expression_parts.join(" ");
    CliParseResult::Eval { width, expression }
}

fn print_help() {
    println!(
        r#"usage: bittwiddle [flags] <expression>

evaluate a bitwise expression and print the result

flags:
    -w, --width <n>    integer width: 8, 16, 32, or 64 (default: 64)
    -h, --help         show this help message
    --version          show version information

the result is printed in four formats (decimal, signed, hex, octal), one per line.

examples:
    bittwiddle 6 + 5
    bittwiddle --width 8 255 + 1"#
    );
}

fn evaluate_expression(expression: &str, width: IntegerWidth) -> Result<(), String> {
    let parsed = expression::parse_expr(width, 0, expression)?;
    let result = expression::compute_expr(width, parsed)?;
    for mode in [
        DisplayMode::Decimal,
        DisplayMode::Signed,
        DisplayMode::Hex,
        DisplayMode::Octal,
    ] {
        println!("{}", BitTwiddleModel::format_result(result, width, mode));
    }
    Ok(())
}

fn run_tui() -> Result<(), io::Error> {
    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;
    let mut model = BitTwiddleModel::default();

    while model.is_running() {
        terminal.draw(|f| view(&model, f))?;
        let frame_area = terminal.size()?.into();
        let mut msg = handle_event(frame_area)?;

        while let Some(m) = msg {
            msg = update(&mut model, m);
        }
    }

    let cleanup_result = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    cleanup_result
}

fn handle_event(frame_area: Rect) -> Result<Option<Message>, io::Error> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(Message::KeyPressed(key))),
        Event::Mouse(mouse) => Ok(Some(Message::MousePressed(mouse, frame_area))),
        _ => Ok(None),
    }
}
