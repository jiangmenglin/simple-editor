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
        execute!(stdout, terminal::EnterAlternateScreen)?;
        Ok(())
    }

    pub fn restore() -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn size() -> io::Result<(u16, u16)> {
        let (cols, rows) = terminal::size()?;
        Ok((cols, rows))
    }

    pub fn read_key() -> io::Result<KeyEvent> {
        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.kind == KeyEventKind::Press {
                        return Ok(key_event);
                    }
                }
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
