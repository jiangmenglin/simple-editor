use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

pub struct Terminal;

impl Terminal {
    pub fn init() -> io::Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;
        Ok(())
    }

    pub fn restore() -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, event::DisableMouseCapture, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn size() -> io::Result<(u16, u16)> {
        let (cols, rows) = terminal::size()?;
        Ok((cols, rows))
    }

    pub fn read_event() -> io::Result<Event> {
        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                let ev = event::read()?;
                match &ev {
                    Event::Key(key_event) => {
                        if key_event.kind == KeyEventKind::Press {
                            return Ok(ev);
                        }
                    }
                    Event::Mouse(_) | Event::Resize(_, _) => {
                        return Ok(ev);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn read_key() -> io::Result<KeyEvent> {
        loop {
            if let Event::Key(key) = Self::read_event()? {
                return Ok(key);
            }
        }
    }

    pub fn clear_screen() -> io::Result<()> {
        execute!(io::stdout(), terminal::Clear(ClearType::All))?;
        Ok(())
    }

    pub fn move_cursor(row: u16, col: u16) -> io::Result<()> {
        execute!(io::stdout(), crossterm::cursor::MoveTo(col, row))?;
        Ok(())
    }

    pub fn flush() -> io::Result<()> {
        io::stdout().flush()
    }
}
