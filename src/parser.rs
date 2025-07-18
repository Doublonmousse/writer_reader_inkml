use anyhow::anyhow;
use std::collections::HashMap;
use std::io::Read;
use xml::reader::{EventReader, XmlEvent as rXmlEvent};

use crate::brushes::Brush;
use crate::context::{Channel, ChannelKind, ChannelType, ChannelUnit, Context, ResolutionUnits};
use crate::trace_data::FormattedStroke;
use crate::trace_data::{ChannelData, TraceData};
use crate::xml_helpers::{get_id, get_ids, verify_channel_properties};
use tracing::{debug,trace};

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
    brushes: HashMap<String, Brush>,
}

#[derive(Debug)]
pub struct ParserResult {
    /// Each element contains
    /// - The name of the context
    /// - The name of the brush
    /// - The (raw) channel data associated with it.
    ///   In particular this raw channel data
    ///     - Keeps the same order as the one given in the trace
    ///     - Keeps the same type (integer, boolean or double) as the
    ///       one given in the trace definition
    context_brush_data_vec: Vec<(String, String, Vec<ChannelData>)>,
    context_dict: HashMap<String, Context>,
    context_brush: HashMap<String, Brush>,
}

/// This function returns the raw data from the trace
/// Hence all supported channels with their origin types are
/// returned, with corresponding resolution, brush properties and so on
pub fn parser<T: Read>(buf_file: T) -> anyhow::Result<ParserResult> {
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
                        debug!("context id :{:?}", id_context);

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
                            return Err(anyhow!("could not create the context"));
                        }
                    }
                    "inkSource" => {
                        let id_source = get_id(&attributes, String::from("id"));
                        debug!("source id :{:?}", id_source);
                        // useful to start/end the parsing of a source (full context !)
                        // though there are cases where only the trace format can exist
                    }
                    "traceFormat" => {
                        debug!("start of traceFormat");
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
                        debug!("here is the current context: {:?}", parser_context.context);
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
                        debug!("{:?}", ids);
                        if let Some(ref current_context) = parser_context.current_context_id {
                            parser_context
                                .context
                                .get_mut(current_context)
                                .ok_or(anyhow!("Could not add the channel to the current context, as it was not found"))?
                                .channel_list
                                .push(Channel::initialise_channel_from_name(ids)?);
                        }
                    }
                    "channelProperties" => {
                        debug!("start of channel properties");
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
                        debug!("{:?}", ids);

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
                                return Err(anyhow!("ParseFloatError: could not parse the value property to a float"));
                            }

                            // find the index
                            let index = match current_context.channel_list.iter().enumerate().fold(None,
                                |acc, (index, channel_el)| {
                                    if channel_el.kind == channel_kind {
                                        Some(index)
                                    } else {
                                        acc
                                    }}) {
                                Some(index) => index,
                                None => {
                                    return Err(anyhow!("Could not find the channel in the list. Searching for {:?}, not present in the list of channels: {:?}", channel_kind,
                                    current_context.channel_list.iter().map(|x| x.kind.clone())))
                                }
                            };

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
                        debug!("brush id {:?}", brush_id);

                        parser_context.current_brush_id = Some(brush_id.clone());
                        if parser_context.brushes.contains_key(&brush_id) {
                            return Err(anyhow!(
                                "DuplicateKeyError : We cannot have twice the same brush"
                            ));
                            // we cannot have twice the same brush id
                        } else {
                            // we init the brush with default parameters
                            // this also allows the default parameter to serve as a fallback (except for the stroke width)
                            parser_context
                                .brushes
                                .insert(brush_id.clone(), Brush::init_brush_with_id(&brush_id));
                        }
                    }
                    "brushProperty" => {
                        // we first check what property we have
                        let property_name_opt = get_id(&attributes, String::from("name"));

                        // get the current brush
                        let current_brush = match parser_context.current_brush_id {
                            None => return Err(anyhow!("Trying to set properties of the current brush but there is no current brush")),
                            Some(ref key) => match parser_context.brushes.get_mut(&key.clone()) {
                                Some(current) => current,
                                None => return Err(anyhow!("could not find the current brush using the current key")),
                            },
                        };

                        match property_name_opt {
                            Some(property_name) => {
                                match property_name.as_str() {
                                    "width" | "height" => {
                                        // as we don't have support for rectangular brushes
                                        // we increase the stroke width and take the max of both

                                        // we convert everything to mm here
                                        let in_unit = match get_id(
                                            &attributes,
                                            String::from("units"),
                                        ) {
                                            None => {
                                                return Err(anyhow!(
                                                    "No unit was found for the brush property {:?}",
                                                    property_name.as_str()
                                                ))
                                            }
                                            Some(unit_str) => {
                                                match ChannelUnit::parse(&Some(unit_str.clone())) {
                                                        Some(unit) => unit,
                                                        None => return Err(anyhow!("Could not find a ChannelUnit matching {:?}", unit_str)),
                                                    }
                                            }
                                        };
                                        let value = match get_id(&attributes, String::from("value"))
                                        {
                                            None => {
                                                return Err(anyhow!(
                                                "No value was given to set the {:?} of the brush",
                                                property_name
                                            ))
                                            }
                                            Some(value_str) => {
                                                value_str.parse::<f64>().map_err(|_| {
                                                    anyhow!("Could not parse {value_str} to f64")
                                                })?
                                            }
                                        };
                                        let stroke_width =
                                            in_unit.convert_to(ChannelUnit::cm, value)?;
                                        current_brush.stroke_width_cm =
                                            current_brush.stroke_width_cm.max(stroke_width);
                                    }
                                    "color" => {
                                        match get_id(&attributes, String::from("value")) {
                                            Some(color_string) => {
                                                // format : #{:02X}{:02X}{:02X} for RGB
                                                if color_string.len() == 7 {
                                                    debug!("Matching color {:?}", color_string);
                                                    let r = u8::from_str_radix(
                                                        &color_string[1..=2],
                                                        16,
                                                    )
                                                    .map_err(|_| {
                                                        anyhow!("Failed to parse {color_string}")
                                                    })?;
                                                    let g = u8::from_str_radix(
                                                        &color_string[3..=4],
                                                        16,
                                                    )
                                                    .map_err(|_| {
                                                        anyhow!("Failed to parse {color_string}")
                                                    })?;
                                                    let b = u8::from_str_radix(
                                                        &color_string[5..=6],
                                                        16,
                                                    )
                                                    .map_err(|_| {
                                                        anyhow!("Failed to parse {color_string}")
                                                    })?;
                                                    current_brush.color = (r, g, b);
                                                } else {
                                                    return Err(anyhow!("Unexpected length for the color string, expected 7, found {}",color_string.len()));
                                                }
                                            }
                                            None => {
                                                return Err(anyhow!(
                                                    "No color was found in the color property"
                                                ));
                                            }
                                        }
                                    }
                                    "transparency" => {
                                        match get_id(&attributes, String::from("value")) {
                                            None => return Err(anyhow!("No transparency value was given in the transparency property")),
                                            Some(value_str) => {
                                                // workaround to make it work with
                                                // this https://devblogs.microsoft.com/microsoft365dev/onenote-ink-beta-apis/
                                                // with transparency between 0 and 256 !!
                                                current_brush.transparency = value_str
                                                    .parse::<u16>()
                                                    .map_err(|_| anyhow!("Failed to parse {value_str} to an integer"))?
                                                    .clamp(0, u8::MAX.into())
                                                    as u8;
                                            }
                                        }
                                    }
                                    "ignorePressure" => {
                                        let value = get_id(&attributes, String::from("value"));
                                        match value {
                                            Some(bool_str) => match bool_str.as_str() {
                                                "1" | "true" => {
                                                    current_brush.ignorepressure = true;
                                                }
                                                "0" | "false" => {
                                                    current_brush.ignorepressure = false;
                                                }
                                                _ => return Err(anyhow!("Unexpected value for the boolean, expected 1,0,true of false, found {bool_str}")),
                                            },
                                            None => return Err(anyhow!("No value was found to set the transparency")),
                                        }
                                    }
                                    _ => {
                                        // ignore
                                        debug!("brush property ignored: {:?}", property_name);
                                    }
                                }
                            }
                            None => {
                                return Err(anyhow!(
                                "No property was given to be changed, empty BrushProperty element"
                            ))
                            }
                        }
                    }
                    "trace" => {
                        trace!("start of trace");
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
                                if !parser_context.brushes.contains_key(&candidate) {
                                    return Err(anyhow!("The trace refers to the Brush {candidate} but it was not found.
                                                        The parser expects trace to refer to brushes that are defined before them in the inkml file"));
                                }
                                Some(candidate)
                            }
                            None => {
                                // ok only if
                                // - zero brush exist : init of the default one latser
                                // - one brush only exist
                                // can we have no brush and need to define a default brush ? not the case for office inkml files .
                                match parser_context.brushes.len() {
                                    0 => None,
                                    1 => parser_context.brushes.keys().next().cloned(),
                                    _ => return Err(anyhow!("Tried to give a default brush to the current trace as no reference was given,
                                                            But this association was ambiguous (more than one brush available)")),
                                }
                            }
                        };
                    }
                    _ => {}
                }
            }
            Ok(rXmlEvent::EndElement { name }) => {
                match name.local_name.as_str() {
                    "definitions" => {
                        debug!("\x1b[93mclosing definitions\x1b[0m");
                    }
                    "context" => {
                        parser_context.current_context_id = None;
                        parser_context.start_context_element = None;
                        debug!("\x1b[93mclosing context\x1b[0m");
                    }
                    "inkSource" => {
                        debug!("\x1b[93mclosing inkSource\x1b[0m");
                    }
                    "traceFormat" => {
                        if !matches!(
                            parser_context.start_context_element,
                            Some(ContextStartElement::TraceFormat)
                        ) {
                            parser_context.start_context_element = None;
                            parser_context.current_context_id = None;
                        }
                        trace!("\x1b[93mclosing traceFormat\x1b[0m");
                    }
                    "channelProperties" => {
                        debug!("\x1b[93mclosing channelProperties\x1b[0m");
                        debug!("now the context is {:?}", parser_context.context);
                    }
                    "trace" => {
                        trace!("\x1b[93mclosing trace\x1b[0m");
                        parser_context.is_trace = false;
                        parser_context.current_context_id = None;
                        parser_context.current_brush_id = None;
                    }
                    "brush" => {
                        debug!("\x1b[93mclosing brush\x1b[0m");

                        // if no stroke width was given, give a min default value
                        match parser_context.current_brush_id {
                        None => return Err(anyhow!("Closing element for a brush but it was never opened, malformed file")),
                        Some(current_key) => {
                            let current_brush = match parser_context.brushes.get_mut(&current_key) {
                                Some(brush) => brush,
                                None => return Err(anyhow!("Cannot find the brush with its (supposedly) key in the dictionnary")),
                            };
                            if current_brush.stroke_width_cm == 0.0 {
                                current_brush.stroke_width_cm = 0.1;
                            }
                        }
                    }

                        parser_context.current_brush_id = None;
                    }
                    _ => {}
                }
            }
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
                            None => {
                                return Err(anyhow!(
                                "Trace data was started but couldn't find its associated context"
                            ))
                            }
                        },
                        None => {
                            return Err(anyhow!(
                            "Text data is only expected inside of a trace but no trace was opened"
                        ))
                        }
                    };

                    trace!("start of trace char");

                    // init the trace data parser
                    let mut trace_data = TraceData::from_channel_types(ch_type_vec);
                    trace_data.parse_raw_data(string_out)?;

                    if (parser_context.current_brush_id.is_none())
                        && (parser_context.brushes.is_empty()
                            || parser_context.brushes.contains_key(&String::from("br0")))
                    {
                        if parser_context.brushes.is_empty() {
                            // no brush was defined. We add a default brush
                            parser_context.brushes.insert(
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
            Err(e) => return Err(anyhow!("Failed to parse xml element : {e}")),
            _ => {}
        }
    }

    Ok(ParserResult {
        context_brush_data_vec: trace_collect,
        context_dict: parser_context.context,
        context_brush: parser_context.brushes,
    })
}

/// This function formats the output of the parser
/// for an easier use.
/// We return an iterator over strokes where the X,Y and F
/// channels are returned as floats with X and Y being in cm unit
/// and F between 0 and 1 (and 1.0 if F is missing), with the associated brush
pub fn parse_formatted<T: Read>(buf_file: T) -> anyhow::Result<Vec<(FormattedStroke, Brush)>> {
    let mut formatted_result: Vec<(FormattedStroke, Brush)> = vec![];
    let ParserResult {
        context_brush_data_vec: strokes,
        context_dict,
        context_brush: brushes_dict,
    } = parser(buf_file)?;

    // iterate over results
    for (context_str, brush_str, stroke) in strokes {
        let context = context_dict
            .get(&context_str)
            .ok_or_else(|| anyhow!("Could not find the context"))?;
        let brush = brushes_dict
            .get(&brush_str)
            .ok_or_else(|| anyhow!("Could not find the brush"))?
            .clone();

        // verify X, Y exist
        let (x_idx, y_idx) = (
            context.channel_exists(ChannelKind::X),
            context.channel_exists(ChannelKind::Y),
        );
        let f_idx = context.channel_exists(ChannelKind::F);

        if x_idx.is_some() && y_idx.is_some() {
            // calculate scalings
            let x_ratio = context
                .channel_list
                .get(x_idx.unwrap())
                .unwrap()
                .get_scaling();
            let y_ratio = context
                .channel_list
                .get(x_idx.unwrap())
                .unwrap()
                .get_scaling();

            formatted_result.push((
                FormattedStroke {
                    x: stroke.get(x_idx.unwrap()).unwrap().cast_to_float(x_ratio),
                    y: stroke.get(y_idx.unwrap()).unwrap().cast_to_float(y_ratio),
                    f: if f_idx.is_some() {
                        let f_ratio = context
                            .channel_list
                            .get(f_idx.unwrap())
                            .unwrap()
                            .get_scaling();
                        stroke.get(f_idx.unwrap()).unwrap().cast_to_float(f_ratio)
                    } else {
                        stroke
                            .get(x_idx.unwrap())
                            .unwrap()
                            .cast_to_float(1.0)
                            .into_iter()
                            .map(|_| 1.0)
                            .collect()
                    },
                },
                brush,
            ));
        }
    }

    Ok(formatted_result)
}
