// Each trace data
// - refers to a context
// - refers to a Brush
// even if these are default values
// From the context we can define what the format of the data is

use crate::{context::ChannelType, traits::Writable};
use anyhow::anyhow;
use tracing::trace;
use xml::writer::XmlEvent;

/// polymorphic enum to hold the data from a trace before a resolution conversion
#[derive(Debug, Clone)]
pub enum ChannelData {
    Integer(Vec<i64>),
    Bool(Vec<bool>),
    Double(Vec<f64>),
}

impl ChannelData {
    pub(crate) fn cast_to_float(&self, scaling: f64) -> Vec<f64> {
        match self {
            ChannelData::Integer(int_vec) => int_vec.iter().map(|x| *x as f64 * scaling).collect(),
            ChannelData::Bool(bool_vec) => bool_vec
                .iter()
                .map(|x| (if *x { 1.0 } else { 0.0 }) * scaling)
                .collect(),
            ChannelData::Double(double_vec) => double_vec.iter().map(|x| x * scaling).collect(),
        }
    }
}

/// polymorhpic enum to hold the data from a point of the trace
/// Only used for holding the last element or difference (in order to calculate
/// 'x or "y)
#[derive(Debug, Clone)]
pub enum ChannelDataEl {
    Integer(i64),
    Double(f64),
    Bool,
}

impl ChannelDataEl {
    pub(crate) fn to_float(&self) -> f64 {
        match self {
            Self::Integer(integer) => *integer as f64,
            Self::Double(double) => *double,
            Self::Bool => 1.0,
        }
    }
}

impl From<ChannelDataEl> for String {
    fn from(value: ChannelDataEl) -> Self {
        match value {
            ChannelDataEl::Bool => String::from("1"),
            ChannelDataEl::Double(value) => format!("{value}"),
            ChannelDataEl::Integer(value) => format!("{value}"),
        }
    }
}

#[derive(Debug)]
/// Type to hold a formatted stroke data
/// - X as a float channel in cm unit
/// - Y as a float channel in cm unit
/// - F as a float channel in dev unit (from 0.0 to 1.0)
pub struct FormattedStroke {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub f: Vec<f64>,
}

impl Writable for FormattedStroke {
    fn write<W: std::io::Write>(
        &self,
        writer: &mut xml::EventWriter<W>,
    ) -> Result<(), xml::writer::Error> {
        // rem : we suppose that the context is the default with pressure one
        // So resolution of 1000 of 1/cm in integer and
        // F in dev unit between 0 and 32767

        let string_out =
            self.x
                .iter()
                .zip(&self.y)
                .zip(&self.f)
                .fold(String::from(""), |acc, ((x, y), f)| {
                    let x_int = (x * 1000.0) as i64;
                    let y_int = (y * 1000.0) as i64;
                    let f_int = (f * 32767.0) as u64;

                    acc + &format!("{x_int} {y_int} {f_int},")
                });

        writer.write(XmlEvent::characters(&string_out[0..string_out.len() - 1]))?;

        writer.write(XmlEvent::end_element())?;

        Ok(())
    }
}

/// Type of modifier
/// Used as a token before the corresponding value is parsed
#[derive(Debug, Clone, Copy)]
enum ValueModifier {
    Explicit,
    SingleDifference,
    DoubleDifference,
}

impl ChannelData {
    fn map_from_channel_type(ch_type: ChannelType) -> ChannelData {
        match ch_type {
            ChannelType::Integer => ChannelData::Integer(vec![]),
            ChannelType::Decimal => ChannelData::Double(vec![]),
            ChannelType::Bool => ChannelData::Bool(vec![]),
            ChannelType::Double => ChannelData::Double(vec![]),
        }
    }
}

pub struct TraceData {
    data: Vec<ChannelData>,
    last_value_modifiers: Vec<ValueModifier>,
    last_value_difference: Vec<ChannelDataEl>,
    /// the index of the channel we are currently parsing the data for
    index_channel: usize,
    /// accumulator for the value of the channel
    value_str: String,
    /// set to true on the first character of the num data of the channel
    is_value_found: bool,
    /// to switch to the new modifier if it's found before the numerical value
    /// Hence we are yet to have the value info to create the nextr LastValueModifier
    new_modifier: ValueModifier,
}

impl TraceData {
    pub fn data(&self) -> Vec<ChannelData> {
        self.data.clone()
    }

    pub fn from_channel_types(types: Vec<ChannelType>) -> TraceData {
        let num_channels = types.len();
        TraceData {
            data: types
                .clone()
                .into_iter()
                .map(ChannelData::map_from_channel_type)
                .collect(),
            last_value_difference: types.into_iter().map(|x| x.get_null_value()).collect(),
            last_value_modifiers: (1..=num_channels)
                .map(|_| ValueModifier::Explicit)
                .collect(),
            index_channel: 0,
            value_str: String::from(""),
            new_modifier: ValueModifier::Explicit,
            is_value_found: false,
        }
    }

    pub fn parse_raw_data(&mut self, line_str: String) -> anyhow::Result<()> {
        //line_str : ex '37'-40'1680'0'0
        // one element from the trace string after
        // splitting per ,
        for line in line_str.split(",") {
            // reset the variables
            self.index_channel = 0;
            self.is_value_found = false;

            let mut iterator = line.char_indices();

            // will store the modifier : updated if needed
            self.new_modifier = *self
                .last_value_modifiers
                .get(self.index_channel)
                .ok_or(anyhow!(""))?;
            while self.index_channel < self.last_value_modifiers.len() {
                match iterator.next() {
                    Some((_, next_char)) => {
                        match next_char {
                            ' ' | '\r' | '\n' | '\t' => {
                                if self.is_value_found {
                                    self.push_found_value()?;
                                }
                            }
                            '!' => {
                                self.new_modifier = ValueModifier::Explicit;
                                if self.is_value_found {
                                    self.push_found_value()?;
                                }
                            }
                            '\'' => {
                                self.new_modifier = ValueModifier::SingleDifference;
                                if self.is_value_found {
                                    self.push_found_value()?;
                                }
                            }
                            '\"' => {
                                self.new_modifier = ValueModifier::DoubleDifference;
                                if self.is_value_found {
                                    self.push_found_value()?;
                                }
                            }
                            '0'..='9' | '.' => {
                                self.is_value_found = true;
                                self.value_str.push(next_char);
                            }
                            '-' => {
                                // 0-12 is valid syntax !!
                                if self.is_value_found {
                                    // if two values are concatenated with no space in between
                                    // parse the value up till now
                                    self.push_found_value()?;
                                    self.is_value_found = true;

                                    // then restart
                                    self.new_modifier = *self
                                        .last_value_modifiers
                                        .get(self.index_channel)
                                        .ok_or(anyhow!("Could not find the last value modified for the current channel"))?;
                                    // we should verify the index here
                                    self.value_str.push(next_char);
                                } else {
                                    self.is_value_found = true;
                                    self.value_str.push(next_char);
                                }
                            }
                            'T' | 'F' => {
                                // for boolean traces
                                self.is_value_found = true;
                                self.value_str.push(next_char);
                                self.push_found_value()?;
                            }
                            _ => return Err(anyhow!("Unexpected char {next_char} found")),
                        }
                    }
                    None => {
                        // we expect to have situation like 0,
                        // hence we have None but we have parsed correctly
                        if self.is_value_found {
                            self.push_found_value()?;
                        } else {
                            return Err(anyhow!("Unexpected end. Expected more data before the end of the current trace"));
                            // we have exhausted the whole line before
                            // parsing all channel data ...
                            // Remark : needed so that we never loop forever
                        }
                    }
                }
            }

            trace!("verifying what's left is only spaces");

            // verify that the end of the line is all spaces
            // check that we have not more ignored data further down
            for (_, next_char) in iterator {
                match next_char {
                    ' ' | '\r' | '\n' | '\t' => {}
                    _ => {
                        return Err(anyhow!(
                            "char not expected {:?}, we only expected space-like elements",
                            next_char
                        )); //there was something left ...
                    }
                }
            }
            trace!("ok, this was only spaces");
        }

        for i in 0..self.data.len() {
            trace!("{:?}", self.data[i]);
        }
        Ok(())
    }

    fn push_found_value(&mut self) -> anyhow::Result<()> {
        // parse the value
        trace!(
            "End val, Value up till now {:?}, modifier {:?}, index : {:?}",
            self.value_str, self.new_modifier, self.index_channel
        );

        // push to the corresponding channel
        match &mut self
            .data
            .get_mut(self.index_channel)
            .ok_or(anyhow!("Could not find the current channel"))?
        {
            ChannelData::Integer(current) => {
                let parsed_value = self.value_str.parse::<i64>();
                trace!(
                    "parsed value : {:?} value str {:?}",
                    parsed_value, self.value_str
                );
                match parsed_value {
                    Ok(value) => match self.new_modifier {
                        ValueModifier::Explicit => {
                            current.push(value);
                        }
                        ValueModifier::SingleDifference => {
                            let previous = current.last().ok_or(anyhow!("could not find the previous value for the channel. 
                                                                                    This is unexpected as we found a single difference modifier, 
                                                                                    so the value is the previous one + the current values"))?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Integer(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Integer(last_difference + value);
                                    current.push(value + previous);
                                }
                                _ => {
                                    return Err(anyhow!(
                                        "The saved previous element for the channel is incorrect."
                                    ))
                                }
                            }
                        }
                        ValueModifier::DoubleDifference => {
                            let previous = current.last().ok_or(anyhow!("Could not find the previous value for the channel.
                                                                            This is unexpected as we found a double difference modifier
                                                                            so the value is calculated relative to the previous one"))?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Integer(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Integer(last_difference + value);
                                    current.push(value + previous + last_difference);
                                }
                                _ => {
                                    return Err(anyhow!(
                                        "The saved previous element for the channel is incorrect"
                                    ))
                                }
                            }
                        }
                    },
                    Err(e) => {
                        return Err(anyhow!("{e} : Could not parse the value as int"));
                    }
                }
            }
            ChannelData::Double(current) => {
                let parsed_value: Result<f64, std::num::ParseFloatError> =
                    self.value_str.parse::<f64>();
                trace!(
                 "parsed value : {:?} value str {:?}",
                    parsed_value, self.value_str
                );
                match parsed_value {
                    Ok(value) => match self.new_modifier {
                        ValueModifier::Explicit => {
                            current.push(value);
                        }
                        ValueModifier::SingleDifference => {
                            let previous = current.last().ok_or(anyhow!(
                                "could not find the previous value for the channel. 
                            This is unexpected as we found a single difference modifier, 
                            so the value is the previous one + the current values"
                            ))?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Double(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Double(last_difference + value);
                                    current.push(value + previous);
                                }
                                _ => {
                                    return Err(anyhow!(
                                        "The saved previous element for the channel is incorrect"
                                    ))
                                }
                            }
                        }
                        ValueModifier::DoubleDifference => {
                            let previous = current.last().ok_or(anyhow!(
                                "could not find the previous value for the channel. 
                            This is unexpected as we found a single difference modifier, 
                            so the value is the previous one + the current values"
                            ))?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Double(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Double(last_difference + value);
                                    current.push(value + previous + last_difference);
                                }
                                _ => {
                                    return Err(anyhow!(
                                        "The saved previous element for the channel is incorrect"
                                    ))
                                }
                            }
                        }
                    },
                    Err(e) => {
                        return Err(anyhow!("{e} : Could not parse to float"));
                    }
                }
            }
            ChannelData::Bool(current) => {
                trace!("value : {:?}", self.value_str);
                let parsed_value = match self.value_str.as_str() {
                    "T" => Ok(true),
                    "F" => Ok(false),
                    _ => Err(()),
                };
                trace!(
                    "parsed value : {:?} value str {:?}",
                    parsed_value, self.value_str
                );

                // boolean : will be true or false, not changing anything there
                // so effectively the corresponding index in the last_value_difference
                // element is unused
                match parsed_value {
                    Ok(bool_value) => {
                        current.push(bool_value);
                    }
                    Err(_) => {
                        return Err(anyhow!(
                            "Could not parse to bool the value {:?}",
                            self.value_str
                        ))
                    }
                }
            }
        }

        self.last_value_modifiers[self.index_channel] = self.new_modifier;
        self.value_str.clear();
        self.index_channel += 1;
        self.is_value_found = false;
        Ok(())
    }
}
