#[cfg(feature = "clipboard")]
use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};

use std::fs::File;
use std::io::BufReader;

mod brushes;
mod context;
mod parser;
mod trace_data;
mod traits;
mod writer;
mod xml_helpers;

use parser::parser;
use writer::writer;

fn main() {
    //parser stage
    let paths = vec![
        "test_files/onenote_multiple_contexts.xml",
        "test_files/correct.xml",
        "test_files/journal_output.xml",
        "test_files/10065.inkml",
        "test_files/highlighter_onenote.xml",
    ];

    for path in paths {
        let file = File::open(path).unwrap();
        let buf_file = BufReader::new(file);
        let result = parser(buf_file).unwrap();
        println!("result : {:?}", result);
    }

    // writer stage
    writer().unwrap();
}
