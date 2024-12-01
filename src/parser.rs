use std::collections::HashMap;
use std::io::{Read, SeekFrom};
use std::num::ParseFloatError;
use xml::reader::{EventReader, XmlEvent as rXmlEvent};

use crate::brushes::{Brush, BrushCollection};
use crate::context::{Channel, ChannelKind, ChannelType, ChannelUnit, Context, ResolutionUnits};
use crate::trace_data::{ChannelData, TraceData};
use crate::xml_helpers::{get_id, get_ids, verify_channel_properties};

#[derive(Debug)]
enum ContextStartElement {
    TraceFormat,
    Context,
}

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
    start_context_element: Option<ContextStartElement>,
    current_brush_id: Option<String>,
    brushes: BrushCollection,
}

pub fn parser<T: Read>(
    buf_file: T,
) -> Result<
    (
        // maybe we should set a ParserResult apart for this
        Vec<(String, String, Vec<ChannelData>)>,
        HashMap<String, Context>,
        HashMap<String, Brush>,
    ),
    (),
> {
    let parser = EventReader::new(buf_file);
    let mut parser_context = ParserContext::default();

    let mut trace_collect: Vec<(String, String, Vec<ChannelData>)> = vec![];

    for xml_event in parser {
        match xml_event {
            Ok(rXmlEvent::StartElement {
                name, attributes, ..
            }) => {
                // we should dispatch on some local names
                match name.local_name.as_str() {
                    "context" => {
                        let id_context =
                            get_id(&attributes, String::from("id")).unwrap_or(String::from("ctx0"));
                        println!("context id :{:?}", id_context);

                        // create the empty context
                        if !parser_context.context.contains_key(&id_context) {
                            parser_context.context.insert(
                                id_context.clone(),
                                Context::create_empty(id_context.clone()),
                            );
                            parser_context.current_context_id = Some(id_context);
                            parser_context.start_context_element =
                                Some(ContextStartElement::Context);
                        } else {
                            return Err(());
                        }
                    }
                    "inkSource" => {
                        let id_source = get_id(&attributes, String::from("id"));
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
                            parser_context.start_context_element =
                                Some(ContextStartElement::TraceFormat);
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
                        // either the id exist or not
                        // if not fallback on a default value
                        let brush_id =
                            get_id(&attributes, String::from("id")).unwrap_or(String::from("br0"));
                        println!("brush id {:?}", brush_id);

                        parser_context.current_brush_id = Some(brush_id.clone());
                        if parser_context.brushes.brushes.contains_key(&brush_id) {
                            return Err(());
                            // we cannot have twice the same brush id
                        } else {
                            // we init the brush with default parameters
                            // this also allows the default parameter to serve as a fallback (except for the stroke width)
                            parser_context
                                .brushes
                                .brushes
                                .insert(brush_id.clone(), Brush::init_brush_with_id(&brush_id));
                        }
                    }
                    "brushProperty" => {
                        // we first check what property we have
                        let property_name_opt = get_id(&attributes, String::from("name"));

                        // get the current brush
                        let current_brush = match parser_context.current_brush_id {
                            None => return Err(()),
                            Some(ref key) => {
                                match parser_context.brushes.brushes.get_mut(&key.clone()) {
                                    Some(current) => current,
                                    None => return Err(()),
                                }
                            }
                        };

                        // TODO
                        // let value = get_ids(
                        //     attributes,
                        //     vec![
                        //         String::from("value"),
                        //         String::from("units"),
                        //     ],
                        // );

                        match property_name_opt {
                            Some(property_name) => {
                                match property_name.as_str() {
                                    "width" | "height" => {
                                        // as we don't have support for rectangular brushes
                                        // we increase the stroke width and take the max of both

                                        // we convert everything to mm here
                                        let in_unit =
                                            match get_id(&attributes, String::from("units")) {
                                                None => return Err(()),
                                                Some(unit_str) => {
                                                    match ChannelUnit::parse(&Some(unit_str)) {
                                                        Some(unit) => unit,
                                                        None => return Err(()),
                                                    }
                                                }
                                            };
                                        let value = match get_id(&attributes, String::from("value"))
                                        {
                                            None => return Err(()),
                                            Some(value_str) => {
                                                value_str.parse::<f64>().map_err(|_| ())?
                                            }
                                        };
                                        let stroke_width =
                                            in_unit.convert_to(ChannelUnit::mm, value)?;
                                        current_brush.stroke_width =
                                            current_brush.stroke_width.max(stroke_width);
                                    }
                                    "color" => {
                                        match get_id(&attributes, String::from("value")) {
                                            Some(color_string) => {
                                                // format : #{:02X}{:02X}{:02X} for RGB
                                                if color_string.len() == 7 {
                                                    println!(
                                                        "hello, matching color {:?}",
                                                        color_string
                                                    );
                                                    let r = u8::from_str_radix(
                                                        &color_string[1..=2],
                                                        16,
                                                    )
                                                    .map_err(|_| ())?;
                                                    let g = u8::from_str_radix(
                                                        &color_string[3..=4],
                                                        16,
                                                    )
                                                    .map_err(|_| ())?;
                                                    let b = u8::from_str_radix(
                                                        &color_string[5..=6],
                                                        16,
                                                    )
                                                    .map_err(|_| ())?;
                                                    current_brush.color = (r, g, b);
                                                } else {
                                                    return Err(());
                                                }
                                            }
                                            None => {
                                                return Err(());
                                            }
                                        }
                                    }
                                    "transparency" => {
                                        match get_id(&attributes, String::from("value")) {
                                            None => return Err(()),
                                            Some(value_str) => {
                                                current_brush.transparency =
                                                    value_str.parse::<u8>().map_err(|_| ())?;
                                            }
                                        }
                                    }
                                    "ignorePressure" => {
                                        let value = get_id(&attributes, String::from("value"));
                                        match value {
                                            Some(bool_str) => match bool_str.as_str() {
                                                "1" => {
                                                    current_brush.ignorepressure = true;
                                                }
                                                "0" => {
                                                    current_brush.ignorepressure = false;
                                                }
                                                _ => return Err(()),
                                            },
                                            None => return Err(()),
                                        }
                                    }
                                    _ => {
                                        // ignore
                                        println!("brush property ignored: {:?}", property_name);
                                    }
                                }
                            }
                            None => return Err(()),
                        }
                    }
                    "trace" => {
                        println!("start of trace");
                        parser_context.is_trace = true;
                        // need to assign a context and a brush
                        // this will give the information on the type (int or float) of each channel
                        // and their number
                        // this will allow to read the trace context that follows
                        // and then populate to a stroke with a color and a width (+ eventually transparency)
                        let ids = get_ids(
                            attributes,
                            vec![String::from("contextRef"), String::from("brushRef")],
                        );

                        parser_context.current_context_id = match &ids[0] {
                            Some(candidate) => Some(candidate.replace("#", "")),
                            None => Some(String::from("ctx0")),
                        };
                        // we will check inside the trace that the context exist or not

                        // we check the brush existence here
                        parser_context.current_brush_id = match &ids[1] {
                            Some(candidate_with_hash) => {
                                let candidate = candidate_with_hash.clone().replace("#", "");
                                if !parser_context.brushes.brushes.contains_key(&candidate) {
                                    return Err(());
                                }
                                Some(candidate)
                            }
                            None => {
                                // ok only if
                                // -zero brush exist : init of the default one latser
                                // - one brush only exist
                                // can we have no brush and need to define a default brush ? not the case for office inkml files .
                                match parser_context.brushes.brushes.len() {
                                    0 => None,
                                    1 => parser_context.brushes.brushes.keys().next().cloned(),
                                    _ => return Err(()),
                                }
                            }
                        };
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
                    parser_context.start_context_element = None;
                    println!("\x1b[93mclosing context\x1b[0m");
                }
                "inkSource" => {
                    println!("\x1b[93mclosing inkSource\x1b[0m");
                }
                "traceFormat" => {
                    if !matches!(
                        parser_context.start_context_element,
                        Some(ContextStartElement::TraceFormat)
                    ) {
                        parser_context.start_context_element = None;
                        parser_context.current_context_id = None;
                    }
                    println!("\x1b[93mclosing traceFormat\x1b[0m");
                }
                "channelProperties" => {
                    println!("\x1b[93mclosing channelProperties\x1b[0m");
                    println!("now the context is {:?}", parser_context.context);
                }
                "trace" => {
                    println!("\x1b[93mclosing trace\x1b[0m");
                    parser_context.is_trace = false;
                    parser_context.current_context_id = None;
                    parser_context.current_brush_id = None;
                }
                "brush" => {
                    println!("\x1b[93mclosing brush\x1b[0m");

                    // if no stroke width was given, give a min default value
                    match parser_context.current_brush_id {
                        None => return Err(()),
                        Some(current_key) => {
                            let current_brush =
                                match parser_context.brushes.brushes.get_mut(&current_key) {
                                    Some(brush) => brush,
                                    None => return Err(()),
                                };
                            if current_brush.stroke_width == 0.0 {
                                current_brush.stroke_width = 0.1;
                            }
                        }
                    }

                    parser_context.current_brush_id = None;
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

                    println!("start of trace char");

                    // init the trace data parser
                    let mut trace_data = TraceData::from_channel_types(ch_type_vec);
                    trace_data.parse_raw_data(string_out)?;

                    if (parser_context.current_brush_id.is_none())
                        && (parser_context.brushes.brushes.is_empty()
                            || parser_context
                                .brushes
                                .brushes
                                .contains_key(&String::from("br0")))
                    {
                        if parser_context.brushes.brushes.is_empty() {
                            // no brush was defined. We add a default brush
                            parser_context.brushes.brushes.insert(
                                String::from("br0"),
                                Brush::init(String::from("br0"), (255, 255, 255), true, 0, 0.1),
                            );
                        }
                        parser_context.current_brush_id = Some(String::from("br0"));
                    }

                    // collect output
                    trace_collect.push((
                        parser_context.current_context_id.unwrap(),
                        parser_context.current_brush_id.unwrap(),
                        trace_data.data(),
                    ));

                    parser_context.current_brush_id = None;
                    parser_context.current_context_id = None;
                }
            }
            Err(_) => return Err(()),
            _ => {}
        }
    }

    Ok((
        trace_collect,
        parser_context.context,
        parser_context.brushes.brushes,
    ))
    // check how to go from data to rnote
    // maybe do a test with the other .jiix and json data already collected
    // up till now
}
