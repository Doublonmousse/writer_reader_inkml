use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use std::fs::File;
use std::io::BufReader;
use std::{f32::consts::PI, io};
use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent as rXmlEvent};
use xml::writer::{EmitterConfig, XmlEvent};

mod brushes;
mod context;
mod trace_data;

use brushes::Brush;
use context::{ChannelType, Context};
use trace_data::TraceData;

fn main() {
    //writer part
    //writer().unwrap();

    //parser stage
    parser().unwrap();
    writer().unwrap();
}

fn parser() -> io::Result<()> {
    let file = File::open("correct.xml")?;
    let file = BufReader::new(file);

    let parser = EventReader::new(file);
    let mut is_trace: bool = false;

    for e in parser {
        match e {
            Ok(rXmlEvent::StartElement {
                name, attributes, ..
            }) => {
                // we should dispatch on some local names
                match name.local_name.as_str() {
                    "context" => {
                        let id_context = get_id(attributes, String::from("id"));
                        println!("context id :{:?}", id_context);
                    }
                    "inkSource" => {
                        let id_source = get_id(attributes, String::from("id"));
                        println!("source id :{:?}", id_source);
                        // useful to start/end the parsing of a source (full context !)
                    }
                    "traceFormat" => {
                        println!("start of traceFormat");
                        // if we have no inkSource, this should init our context as well with a default inkSource id here
                    }
                    "channel" => {
                        let ids = get_ids(
                            attributes,
                            vec![
                                String::from("name"),
                                String::from("type"),
                                String::from("units"),
                            ],
                        );
                        println!("{:?}", ids);
                    }
                    "channelProperties" => {
                        println!("start of channel properties");
                    }
                    "channelProperty" => {
                        let ids = get_ids(
                            attributes,
                            vec![
                                String::from("channel"),
                                String::from("name"),
                                String::from("value"),
                                String::from("units"),
                            ],
                        );
                        println!("{:?}", ids);
                    }
                    "brush" => {
                        let brush_id = get_id(attributes, String::from("id"));
                        println!("brush id {:?}", brush_id);
                        // we have to register a brush (with some name of default otherwise)
                    }
                    "brushProperty" => {
                        let ids = get_ids(
                            attributes,
                            vec![
                                String::from("name"),
                                String::from("value"),
                                String::from("units"),
                            ],
                        );
                        println!("{:?}", ids);
                    }
                    "trace" => {
                        println!("start of trace");
                        is_trace = true;
                        // need to assign a context and a brush
                        // this will give the information on the type (int or float) of each channel
                        // and their number
                        // this will allow to read the trace context that follows
                        // and then populate to a stroke with a color and a width
                        let ids = get_ids(
                            attributes,
                            vec![String::from("contextRef"), String::from("brushRef")],
                        );
                        println!("{:?}", ids);
                    }
                    _ => {}
                }
            }
            Ok(rXmlEvent::EndElement { name }) => match name.local_name.as_str() {
                "definitions" => {
                    println!("\x1b[93mclosing definitions\x1b[0m");
                }
                "context" => {
                    println!("\x1b[93mclosing context\x1b[0m");
                }
                "inkSource" => {
                    println!("\x1b[93mclosing inkSource\x1b[0m");
                }
                "traceFormat" => {
                    println!("\x1b[93mclosing traceFormat\x1b[0m");
                }
                "channelProperties" => {
                    println!("\x1b[93mclosing channelProperties\x1b[0m");
                }
                "trace" => {
                    println!("\x1b[93mclosing trace\x1b[0m");
                    is_trace = false;
                }
                "brush" => {
                    println!("\x1b[93mclosing brush\x1b[0m");
                }
                _ => {}
            },
            Ok(rXmlEvent::Characters(string_out)) => {
                // we have to verify we are inside a trace
                if is_trace {
                    let ch_type_vec: Vec<ChannelType> = vec![
                        ChannelType::Integer,
                        ChannelType::Integer,
                        ChannelType::Integer,
                        ChannelType::Decimal,
                        ChannelType::Double,
                    ];
                    let mut trace_data = TraceData::from_channel_types(ch_type_vec);
                    match trace_data.parse_raw_data(string_out) {
                        Ok(()) => {}
                        Err(()) => {
                            eprintln!("Error: ");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    Ok(())
}

fn get_id(attributes: Vec<OwnedAttribute>, match_string: String) -> Option<String> {
    attributes
        .into_iter()
        .filter(|x| x.name.local_name == match_string)
        .map(|x| x.value)
        .next()
}

/// gets the attributes we asked for in that order
fn get_ids(attributes: Vec<OwnedAttribute>, match_string: Vec<String>) -> Vec<Option<String>> {
    match_string
        .into_iter()
        .map(|x| {
            attributes
                .clone()
                .into_iter()
                .filter(|attr| x == attr.name.local_name)
                .map(|x| x.value.clone())
                .next()
        })
        .collect()
}

fn writer() -> io::Result<()> {
    // let output = io::stdout();
    let mut out_v: Vec<u8> = vec![];
    let mut writer = EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(false)
        .create_writer(&mut out_v);

    // xmls : InkML
    writer
        .write(XmlEvent::start_element("ink").default_ns("http://www.w3.org/2003/InkML"))
        .unwrap();

    // definitions block
    // contains :
    // context/inksource/traceFormat
    //  - name of channels, encoding and units
    // context/inksource/channelProperties
    //  - more properties, resolution and units (if integer encoded, what's 1 in cm !)
    // brush list
    // - width, height, color, ignorePressure
    writer
        .write(XmlEvent::start_element("definitions"))
        .unwrap();

    let context = Context::default();
    context.write_context(&mut writer).unwrap();

    // collect brushes

    // for now one brush
    let brush = Brush::init(String::from("br1"), &context, (255, 255, 12), true, 0.2);
    // write brushes
    brush.write_brush(&mut writer).unwrap();

    writer.write(XmlEvent::end_element()).unwrap(); // end definitions

    // iterate over strokes
    //add trace element with some contextRef and brushRef
    // we also need to iterate on positions + convert with the correct
    // value (depending on resolution and units for source and end !)
    writer
        .write(
            XmlEvent::start_element("trace")
                .attr("contextRef", "#ctx0")
                .attr("brushRef", "#br1"),
        )
        .unwrap();

    // generate some data here
    let positions: Vec<(f32, f32)> = (1..10)
        .map(|x| {
            (
                (f32::sin(2.0 * PI * (x as f32) / 10.0) + 2.0) * 1000.0,
                (f32::cos(2.0 * PI * (x as f32) / 10.0) + 2.0) * 1000.0,
            )
        })
        .collect();

    let mut string_out = positions
        .into_iter()
        .fold(String::from("#"), |acc, (x, y)| {
            acc + &format!("{:.} {:.},", x.round(), y.round())
        });
    string_out = string_out[1..string_out.len() - 1].to_string();

    // for now this is very basic !
    // we should go through the
    // add our data
    writer.write(XmlEvent::characters(&string_out)).unwrap();

    writer.write(XmlEvent::end_element()).unwrap(); //end
    writer.write(XmlEvent::end_element()).unwrap(); // end ink

    // collect everything
    println!("Hello, {:?}", String::from_utf8(out_v.clone()));

    // copy to clipboard
    let mimetype = String::from("InkML Format");
    let content: Vec<ClipboardContent> = vec![ClipboardContent::Other(mimetype, out_v.to_owned())];
    let ctx = ClipboardContext::new().unwrap();
    let _ = ctx.set(content);

    Ok(())
}
