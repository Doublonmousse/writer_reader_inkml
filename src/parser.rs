use std::collections::HashMap;
use std::io::Read;
use xml::reader::{EventReader, XmlEvent as rXmlEvent};

use crate::brushes::BrushCollection;
use crate::context::{Channel, ChannelKind, ChannelType, Context, ResolutionUnits};
use crate::trace_data::TraceData;
use crate::xml_helpers::{get_id, get_ids, verify_channel_properties};

#[derive(Default, Debug)]
struct ParserContext {
    /// keeps trace of whether we are inside of a trace element
    is_trace: bool,
    /// stores the context(s) of the inkml file.
    /// Using a conditional to mark whether we encountered a context
    /// We suppose that there is always a context, even if this is only
    /// a `traceFormat` tag (in that case there would be only one context !)
    context: HashMap<String, Context>,
    current_context_id: Option<String>,
    current_brush_id: Option<String>,
    brushes: BrushCollection,
}

pub fn parser<T: Read>(buf_file: T) -> Result<(), ()> {
    let parser = EventReader::new(buf_file);
    let mut parser_context = ParserContext::default();

    for xml_event in parser {
        match xml_event {
            Ok(rXmlEvent::StartElement {
                name, attributes, ..
            }) => {
                // we should dispatch on some local names
                match name.local_name.as_str() {
                    "context" => {
                        let id_context =
                            get_id(attributes, String::from("id")).unwrap_or(String::from("ctx0"));
                        println!("context id :{:?}", id_context);

                        // create the empty context
                        if !parser_context.context.contains_key(&id_context) {
                            parser_context.context.insert(
                                id_context.clone(),
                                Context::create_empty(id_context.clone()),
                            );
                            parser_context.current_context_id = Some(id_context);
                        } else {
                            return Err(());
                        }
                    }
                    "inkSource" => {
                        let id_source = get_id(attributes, String::from("id"));
                        println!("source id :{:?}", id_source);
                        // useful to start/end the parsing of a source (full context !)
                        // though there are cases where only the trace format can exist
                    }
                    "traceFormat" => {
                        println!("start of traceFormat");
                        // if we have no inkSource, this should init our context as well with a default inkSource id here
                        if parser_context.context.is_empty() {
                            // create a new context with a default name
                            parser_context.context.insert(
                                String::from("ctx0"),
                                Context::create_empty(String::from("ctx0")),
                            );
                            parser_context.current_context_id = Some(String::from("ctx0"));
                        }

                        println!("here is the current context: {:?}", parser_context.context);
                    }
                    "channel" => {
                        let ids = get_ids(
                            attributes,
                            vec![
                                String::from("name"),
                                String::from("type"),
                                String::from("units"), // can be optional
                                String::from("max"),
                            ],
                        );
                        // add the channels to the CURRENT context
                        println!("{:?}", ids);
                        if let Some(ref current_context) = parser_context.current_context_id {
                            parser_context
                                .context
                                .get_mut(current_context)
                                .ok_or(())?
                                .channel_list
                                .push(Channel::initialise_channel_from_name(ids)?);
                        }
                    }
                    "channelProperties" => {
                        println!("start of channel properties");
                    }
                    "channelProperty" => {
                        // inside of a context, the channelProperty gives additional info on the scaling of elements
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

                        if verify_channel_properties(&ids)
                            && parser_context.current_context_id.is_some()
                            && parser_context
                                .context
                                .contains_key(&parser_context.current_context_id.clone().unwrap())
                        {
                            // get the current context
                            let current_context = parser_context
                                .context
                                .get_mut(&parser_context.current_context_id.clone().unwrap())
                                .unwrap();

                            let channel_kind = ChannelKind::parse(&ids[0])?;
                            let resolution_units = ResolutionUnits::parse(&ids[3])?;
                            let value = &ids[2].clone().unwrap().parse::<f64>();
                            if value.is_err() {
                                return Err(());
                            }

                            // find the index
                            let index = current_context.channel_list.iter().enumerate().fold(
                                Err(()),
                                |acc, (index, channel_el)| {
                                    if channel_el.kind == channel_kind {
                                        Ok(index)
                                    } else {
                                        acc
                                    }
                                },
                            )?;
                            let channel_to_update =
                                current_context.channel_list.get_mut(index).unwrap();
                            channel_to_update.resolution_value = value.clone().unwrap();
                            channel_to_update.unit_resolution = resolution_units;
                        }
                    }
                    "brush" => {
                        let brush_id = get_id(attributes, String::from("id"));
                        println!("brush id {:?}", brush_id);
                        // we have to register a brush (with some name of default otherwise)
                        // also need to init the current brush id (and check for duplicate)
                        // NEXT STEP
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
                        // NEXT STEP 2
                    }
                    "trace" => {
                        println!("start of trace");
                        parser_context.is_trace = true;
                        // need to assign a context and a brush
                        // this will give the information on the type (int or float) of each channel
                        // and their number
                        // this will allow to read the trace context that follows
                        // and then populate to a stroke with a color and a width
                        let ids = get_ids(
                            attributes,
                            vec![String::from("contextRef"), String::from("brushRef")],
                        );

                        parser_context.current_context_id = match &ids[0] {
                            Some(candidate) => Some(candidate.replace("#", "")),
                            None => Some(String::from("ctx0s")),
                        };
                        // we will check inside the trace that the context exist or not
                        parser_context.current_brush_id = match &ids[1] {
                            Some(candidate) => Some(candidate.replace("#", "")),
                            None => None,
                        };
                        // for the brush, for now not checked here, we should do that here though
                        // TODO
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
                    parser_context.current_context_id = None;
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
                    println!("now the context is {:?}", parser_context.context);
                }
                "trace" => {
                    println!("\x1b[93mclosing trace\x1b[0m");
                    parser_context.is_trace = false;
                }
                "brush" => {
                    println!("\x1b[93mclosing brush\x1b[0m");
                }
                _ => {}
            },
            Ok(rXmlEvent::Characters(string_out)) => {
                // we have to verify we are inside a trace
                if parser_context.is_trace {
                    // get the ChannelType from the current context
                    let ch_type_vec = match parser_context.current_context_id {
                        Some(ref key) => match parser_context.context.get(&key.clone()) {
                            Some(current_context) => current_context
                                .channel_list
                                .iter()
                                .map(|x| x.types.clone())
                                .collect::<Vec<ChannelType>>(),
                            None => return Err(()),
                        },
                        None => return Err(()),
                    };
                    // init the trace data parser
                    let mut trace_data = TraceData::from_channel_types(ch_type_vec);
                    trace_data.parse_raw_data(string_out)?;
                }
            }
            Err(_) => return Err(()),
            _ => {}
        }
    }
    Ok(())
}
