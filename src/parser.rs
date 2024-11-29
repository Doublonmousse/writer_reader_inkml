use std::io;
use std::io::Read;
use xml::reader::{EventReader, XmlEvent as rXmlEvent};

use crate::context::ChannelType;
use crate::trace_data::TraceData;
use crate::xml_helpers::{get_id, get_ids};

pub fn parser<T: Read>(buf_file: T) -> io::Result<()> {
    let parser = EventReader::new(buf_file);
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
            _ => {}
        }
    }

    Ok(())
}
