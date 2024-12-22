use brushes::Brush;
use std::fs::File;
use std::io::BufReader;
use trace_data::FormattedStroke;
mod brushes;
mod context;
mod parser;
mod trace_data;
mod traits;
mod writer;
mod xml_helpers;

use parser::{parse_formatted, parser};
use tracing::trace;
#[cfg(feature = "tracer")]
use tracing_subscriber;
use writer::writer;

fn main() {
    #[cfg(feature = "tracer")]
    tracing_subscriber::fmt::init();
    //parser stage
    let paths = vec![
        "test_files/onenote_multiple_contexts.xml",
        "test_files/correct.xml",
        "test_files/journal_output.xml",
        "test_files/10065.inkml",
        "test_files/highlighter_onenote.xml",
        "test_files/onenote_web.xml",
    ];

    for path in paths {
        let file = File::open(path).unwrap();
        let buf_file = BufReader::new(file);
        let result = parser(buf_file).unwrap();
        trace!("result : {:?}", result);

        // test with parser_formatted
        let file = File::open(path).unwrap();
        let buf_file = BufReader::new(file);
        let result_formatted = parse_formatted(buf_file).unwrap();
        trace!("result: {:?}", result_formatted);
    }

    // writer stage
    let data = vec![(
        FormattedStroke {
            x: vec![0.0, 1.0],
            y: vec![0.0, 1.0],
            f: vec![0.0, 1.0],
        },
        Brush::init(String::from("hello"), (0, 1, 0), true, 150, 10.0),
    )];
    writer(data).unwrap();
}
