// Each trace data
// - refers to a context
// - refers to a Brush
// even if these are default values
// From the context we can define what the format of the data is

use crate::context::ChannelType;

/// polymorphic enum to hold the data from a trace before a resolution conversion
#[derive(Debug, Clone)]
pub enum ChannelData {
    Integer(Vec<i64>),
    Bool(Vec<bool>),
    Double(Vec<f64>),
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

    pub fn parse_raw_data(&mut self, line_str: String) -> Result<(), ()> {
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
                .ok_or(())?;
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
                                    self.push_found_value()?; //bug here ?
                                    self.is_value_found = true;

                                    // then restart
                                    self.new_modifier = *self
                                        .last_value_modifiers
                                        .get(self.index_channel)
                                        .ok_or(())?;
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
                            _ => return Err(()),
                        }
                    }
                    None => {
                        // we expect to have situation like 0,
                        // hence we have None but we have parsed correctly
                        if self.is_value_found {
                            self.push_found_value()?;
                        } else {
                            return Err(());
                            // we have exhausted the whole line before
                            // parsing all channel data ...
                            // Remark : needed so that we never loop forever
                        }
                    }
                }
            }

            //println!("verifying what's left is only spaces");

            // verify that the end of the line is all spaces
            // check that we have not more ignored data further down
            for (_, next_char) in iterator {
                match next_char {
                    ' ' | '\r' | '\n' | '\t' => {}
                    _ => {
                        println!("char not expected {:?}", next_char);
                        return Err(()); //there was something left ...
                    }
                }
            }
            // println!("ok, this was only spaces");
        }

        for i in 0..self.data.len() {
            println!("{:?}", self.data[i]);
        }
        Ok(())
    }

    fn push_found_value(&mut self) -> Result<(), ()> {
        // parse the value
        // debug trace
        // println!(
        //     "End val, Value up till now {:?}, modifier {:?}, index : {:?}",
        //     self.value_str, self.new_modifier, self.index_channel
        // );

        // push to the corresponding channel
        match &mut self.data.get_mut(self.index_channel).ok_or(())? {
            ChannelData::Integer(current) => {
                let parsed_value = self.value_str.parse::<i64>();
                // println!(
                //     "parsed value : {:?} value str {:?}",
                //     parsed_value, self.value_str
                // );
                match parsed_value {
                    Ok(value) => match self.new_modifier {
                        ValueModifier::Explicit => {
                            current.push(value);
                        }
                        ValueModifier::SingleDifference => {
                            let previous = current.last().ok_or(())?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Integer(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Integer(last_difference + value);
                                    current.push(value + previous);
                                }
                                _ => return Err(()),
                            }
                        }
                        ValueModifier::DoubleDifference => {
                            let previous = current.last().ok_or(())?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Integer(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Integer(last_difference + value);
                                    current.push(value + previous + last_difference);
                                }
                                _ => return Err(()),
                            }
                        }
                    },
                    Err(_) => {
                        return Err(());
                    }
                }
            }
            ChannelData::Double(current) => {
                let parsed_value: Result<f64, std::num::ParseFloatError> =
                    self.value_str.parse::<f64>();
                // println!(
                //     "parsed value : {:?} value str {:?}",
                //     parsed_value, self.value_str
                // );
                match parsed_value {
                    Ok(value) => match self.new_modifier {
                        ValueModifier::Explicit => {
                            current.push(value);
                        }
                        ValueModifier::SingleDifference => {
                            let previous = current.last().ok_or(())?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Double(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Double(last_difference + value);
                                    current.push(value + previous);
                                }
                                _ => return Err(()),
                            }
                        }
                        ValueModifier::DoubleDifference => {
                            let previous = current.last().ok_or(())?;
                            let last_difference_container =
                                self.last_value_difference[self.index_channel].clone();
                            match last_difference_container {
                                ChannelDataEl::Double(last_difference) => {
                                    self.last_value_difference[self.index_channel] =
                                        ChannelDataEl::Double(last_difference + value);
                                    current.push(value + previous + last_difference);
                                }
                                _ => return Err(()),
                            }
                        }
                    },
                    Err(_) => {
                        return Err(());
                    }
                }
            }
            ChannelData::Bool(current) => {
                println!("value : {:?}", self.value_str);
                let parsed_value = match self.value_str.as_str() {
                    "T" => Ok(true),
                    "F" => Ok(false),
                    _ => Err(()),
                };
                // println!(
                //     "parsed value : {:?} value str {:?}",
                //     parsed_value, self.value_str
                // );

                // boolean : will be true or false, not changing anything there
                // so effectively the corresponding index in the last_value_difference
                // element is unused
                match parsed_value {
                    Ok(bool_value) => {
                        current.push(bool_value);
                    }
                    Err(_) => return Err(()),
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
