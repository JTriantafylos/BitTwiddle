use bittwiddle::{Message, model::BitTwiddleModel, update, view::view};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
        execute,
    },
    layout::Rect,
};
use std::io;

fn handle_event(frame_area: Rect) -> Result<Option<Message>, io::Error> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(Message::KeyPressed(key))),
        Event::Mouse(mouse) => Ok(Some(Message::MousePressed(mouse, frame_area))),
        _ => Ok(None),
    }
}

fn main() -> Result<(), io::Error> {
    if std::env::args().any(|a| a == "--version") {
        println!(
            "{} version {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        println!("Author: {}", env!("CARGO_PKG_AUTHORS"));
        println!("License: {}", env!("CARGO_PKG_LICENSE"));
        return Ok(());
    }

    let result = (|| -> Result<(), io::Error> {
        let mut terminal = ratatui::init();
        execute!(io::stdout(), EnableMouseCapture)?;
        let mut model = BitTwiddleModel::default();

        while model.is_running() {
            terminal.draw(|f| view(&model, f))?;
            let frame_area = terminal.size()?.into();
            let mut current_msg = handle_event(frame_area)?;

            while let Some(msg) = current_msg {
                current_msg = update(&mut model, msg);
            }
        }

        Ok(())
    })();

    let cleanup_result = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result.and(cleanup_result)
}
