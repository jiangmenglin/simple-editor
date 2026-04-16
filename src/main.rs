mod editor;
mod row;
mod terminal;
mod find;

use std::env;
use std::io;

use editor::Editor;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).cloned();

    let mut editor = Editor::new(filename)?;
    editor.run()
}
