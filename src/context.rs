// types for the whole program
// for writing we assume we'll use only 1 context
// but we use as many brushes as needed

use crate::trace_data::ChannelDataEl;
use crate::traits::Writable;
use anyhow::anyhow;
use std::io::Write;
use xml::writer::{Error, EventWriter, XmlEvent};

/// types of channel
/// We will only use X,Y,F for the most part
/// TODO : use the full channel list from the inkml spec
#[derive(Clone, PartialEq, Debug)]
#[allow(unused)]
pub enum ChannelKind {
    /// X coordinates, left to right
    X,
    /// Y coordinates, high to bottom
    Y,
    /// F : force/pressure
    F,
    /// azimuth angle of the pen
    OA,
    /// elevation angle of the pen
    OE,
    OTx,
    OTy,
}

impl ChannelKind {
    pub(crate) fn parse(name: &Option<String>) -> anyhow::Result<ChannelKind> {
        match name {
            Some(value) => match value.as_str() {
                "X" => Ok(ChannelKind::X),
                "Y" => Ok(ChannelKind::Y),
                "F" => Ok(ChannelKind::F),
                "OA" => Ok(ChannelKind::OA),
                "OE" => Ok(ChannelKind::OE),
                "OTx" => Ok(ChannelKind::OTx),
                "OTy" => Ok(ChannelKind::OTy),
                _ => Err(anyhow!("the channel kind {:?} was not found. It is either not implemented or incorrect", value)),
            },
            None => Err(anyhow!("an empty string was given")),
        }
    }

    fn get_default_resolution_unit(&self) -> ResolutionUnits {
        match self {
            ChannelKind::X | ChannelKind::Y => ResolutionUnits::OneOverCm,
            ChannelKind::F => ResolutionUnits::OneOverDev,
            ChannelKind::OA | ChannelKind::OE | ChannelKind::OTx | ChannelKind::OTy => {
                ResolutionUnits::OneOverDegree
            }
        }
    }

    fn get_default_unit(&self) -> ChannelUnit {
        match self {
            ChannelKind::X | ChannelKind::Y => ChannelUnit::cm,
            ChannelKind::F => ChannelUnit::dev,
            ChannelKind::OA | ChannelKind::OE | ChannelKind::OTx | ChannelKind::OTy => {
                ChannelUnit::deg
            }
        }
    }
}

impl From<ChannelKind> for String {
    fn from(value: ChannelKind) -> Self {
        match value {
            ChannelKind::X => String::from("X"),
            ChannelKind::Y => String::from("Y"),
            ChannelKind::F => String::from("F"),
            ChannelKind::OA => String::from("OA"),
            ChannelKind::OE => String::from("OF"),
            ChannelKind::OTx => String::from("OTx"),
            ChannelKind::OTy => String::from("OTy"),
        }
    }
}

/// type used for the encoding
#[derive(Clone, Debug)]
#[allow(unused)]
#[derive(Default)]
pub enum ChannelType {
    Integer,
    #[default]
    Decimal,
    Double,
    Bool,
}

impl ChannelType {
    pub(crate) fn parse(name: &Option<String>) -> anyhow::Result<ChannelType> {
        match name {
            Some(value) => match value.as_str() {
                "integer" => Ok(ChannelType::Integer),
                "decimal" => Ok(ChannelType::Decimal),
                "double" => Ok(ChannelType::Double),
                "boolean" => Ok(ChannelType::Bool),
                _ => Err(anyhow!("the channel type {value} is not part of the types accepted : integer, decimal, double or boolean")),
            },
            None => Err(anyhow!("ChannelType:parse was given a None")),
        }
    }

    fn get_max_value(&self, max_val: &Option<String>) -> Option<ChannelDataEl> {
        match max_val {
            None => None,
            Some(max_parsed_str) => {
                // match depending on the type
                match self {
                    ChannelType::Integer => max_parsed_str
                        .parse::<i64>()
                        .map(ChannelDataEl::Integer)
                        .ok(),
                    ChannelType::Double | ChannelType::Decimal => max_parsed_str
                        .parse::<f64>()
                        .map(ChannelDataEl::Double)
                        .ok(),
                    _ => None,
                }
            }
        }
    }
}

impl From<ChannelType> for String {
    fn from(value: ChannelType) -> Self {
        match value {
            ChannelType::Integer => String::from("integer"),
            ChannelType::Decimal => String::from("decimal"),
            ChannelType::Double => String::from("double"),
            ChannelType::Bool => String::from("bool"),
        }
    }
}

impl ChannelType {
    pub fn get_null_value(self: &ChannelType) -> ChannelDataEl {
        match self {
            ChannelType::Integer => ChannelDataEl::Integer(0),
            ChannelType::Decimal => ChannelDataEl::Double(0.0),
            ChannelType::Bool => ChannelDataEl::Bool,
            ChannelType::Double => ChannelDataEl::Double(0.0),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
#[derive(Default)]
pub enum ResolutionUnits {
    // 1/cm
    #[default]
    OneOverCm,
    // 1/mm
    OneOverMm,
    // 1/dev, dev device unit
    OneOverDev,
    // 1/deg, degree
    OneOverDegree,
    // 1/himetric
    OneOverHimetric,
}

impl From<ResolutionUnits> for String {
    fn from(value: ResolutionUnits) -> Self {
        match value {
            ResolutionUnits::OneOverCm => String::from("1/cm"),
            ResolutionUnits::OneOverMm => String::from("1/mm"),
            ResolutionUnits::OneOverDev => String::from("1/dev"),
            ResolutionUnits::OneOverDegree => String::from("1/deg"),
            ResolutionUnits::OneOverHimetric => String::from("1/himetric"),
        }
    }
}

impl ResolutionUnits {
    pub fn parse(name: &Option<String>) -> anyhow::Result<ResolutionUnits> {
        match name {
            Some(value) => match value.as_str() {
                "1/cm" => Ok(ResolutionUnits::OneOverCm),
                "1/mm" => Ok(ResolutionUnits::OneOverMm),
                "1/dev" => Ok(ResolutionUnits::OneOverDev),
                "1/deg" => Ok(ResolutionUnits::OneOverDegree),
                "1/himetric" => Ok(ResolutionUnits::OneOverHimetric),
                _ => Err(
                    anyhow!("Could not find a `ResolutionUnits` matching {value}. 
                            It either is incorrect of this unit is not implemented 
                            (1/cm, 1/mm, 1/dev, 1/deg, and 1/himetric are the ones currently implemented)"),
                ),
            },
            None => Err(anyhow!("ResolutionUnits::parse was given a None, aborting")),
        }
    }
}

// TODO : use the full unit list from the inkml spec
#[derive(Clone, Debug)]
#[allow(unused, non_camel_case_types)]
#[derive(Default)]
pub(crate) enum ChannelUnit {
    /// distance unit, `mm`
    mm,
    /// distance unit, `cm`
    #[default]
    cm,
    /// distance unit, `m`
    m,
    /// device ind unit
    dev,
    /// degree
    deg,
    /// himetric
    himetric,
}

impl From<ChannelUnit> for String {
    fn from(value: ChannelUnit) -> Self {
        match value {
            ChannelUnit::mm => String::from("mm"),
            ChannelUnit::cm => String::from("cm"),
            ChannelUnit::m => String::from("m"),
            ChannelUnit::dev => String::from("dev"),
            ChannelUnit::deg => String::from("deg"),
            ChannelUnit::himetric => String::from("himetric"),
        }
    }
}

impl ChannelUnit {
    pub(crate) fn parse(name: &Option<String>) -> Option<ChannelUnit> {
        match name {
            Some(value) => match value.as_str() {
                "mm" => Some(ChannelUnit::mm),
                "cm" => Some(ChannelUnit::cm),
                "m" => Some(ChannelUnit::m),
                "dev" => Some(ChannelUnit::dev),
                "deg" => Some(ChannelUnit::deg),
                "himetric" => Some(ChannelUnit::himetric),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn convert_to(
        &self,
        output_unit: ChannelUnit,
        input_value: f64,
    ) -> anyhow::Result<f64> {
        // pretty horrible, better to use a table/matrix with conversion values ?
        match (self, output_unit) {
            (ChannelUnit::mm, ChannelUnit::mm) => Ok(input_value),
            (ChannelUnit::mm, ChannelUnit::cm) => Ok(input_value * 1e-1),
            (ChannelUnit::mm, ChannelUnit::m) => Ok(input_value * 1e-3),
            (ChannelUnit::cm, ChannelUnit::mm) => Ok(input_value * 1e1),
            (ChannelUnit::cm, ChannelUnit::cm) => Ok(input_value),
            (ChannelUnit::cm, ChannelUnit::m) => Ok(input_value * 1e-2),
            (ChannelUnit::m, ChannelUnit::mm) => Ok(input_value * 1e3),
            (ChannelUnit::m, ChannelUnit::cm) => Ok(input_value * 1e2),
            (ChannelUnit::m, ChannelUnit::m) => Ok(input_value),
            (ChannelUnit::deg, ChannelUnit::deg) => Ok(input_value),
            (ChannelUnit::dev, ChannelUnit::dev) => Ok(input_value),
            (ChannelUnit::himetric, ChannelUnit::cm) => Ok(input_value * 1e-3),
            (ChannelUnit::himetric, ChannelUnit::mm) => Ok(input_value * 1e-2),
            (ChannelUnit::himetric, ChannelUnit::m) => Ok(input_value * 1e-5),
            (input, output) => Err(anyhow!(
                "Could not convert from {:?} to {:?}. Is the conversion valid ? 
                (For example, converting deg to meters, or dev to another unit)",
                input,
                output
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub kind: ChannelKind,
    pub types: ChannelType,
    pub resolution_value: f64,
    max_value: Option<ChannelDataEl>,
    pub unit_resolution: ResolutionUnits,
    unit_channel: ChannelUnit,
}

impl Channel {
    pub fn initialise_channel_from_name(
        kind_type_unit_v: Vec<Option<String>>,
    ) -> anyhow::Result<Channel> {
        let channel_type = &kind_type_unit_v[1];
        let unit = &kind_type_unit_v[2];

        let channel_kind = ChannelKind::parse(&kind_type_unit_v[0])?;
        let types = ChannelType::parse(channel_type)?;

        // we are parsing the max value
        // useful for the F channel (where the mapping in 0-1 is done through the max value)
        // For the F channel, if we have a dev unit, the max value will be used for the mapping instead
        Ok(Channel {
            kind: channel_kind.clone(),
            types: types.clone(),
            resolution_value: 1000.0,
            max_value: types.get_max_value(&kind_type_unit_v[3]),
            unit_resolution: channel_kind.get_default_resolution_unit(),
            unit_channel: ChannelUnit::parse(unit).unwrap_or(channel_kind.get_default_unit()),
        })
    }

    pub fn get_scaling(&self) -> f64 {
        if self.max_value.is_some() && self.kind == ChannelKind::F {
            // exception for F
            1.0 / self.max_value.as_ref().unwrap().to_float()
        } else {
            let ratio = match self.unit_resolution {
                ResolutionUnits::OneOverCm => 1.0,
                ResolutionUnits::OneOverMm => 0.1,
                ResolutionUnits::OneOverDegree => 1.0,
                ResolutionUnits::OneOverDev => 1.0,
                ResolutionUnits::OneOverHimetric => 1.0 / 1000.0,
            };
            ratio * (1.0 / self.resolution_value)
        }
    }
}

#[derive(Debug)]
pub struct Context {
    // name given to the context, name = ctx0 by default
    // refered by `contextRef="#ctx0" in the trace attr
    pub name: String,
    /// vector of channels
    /// Remark : we NEED the order to be preserved as the order here
    /// also corresponds to the order in which traces are built
    pub channel_list: Vec<Channel>,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            name: String::from("ctx0"),
            channel_list: vec![
                Channel {
                    kind: ChannelKind::X,
                    types: ChannelType::Integer,
                    resolution_value: 1000.0,
                    max_value: None,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
                Channel {
                    kind: ChannelKind::Y,
                    types: ChannelType::Integer,
                    resolution_value: 1000.0,
                    max_value: None,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
            ],
        }
    }
}

impl Context {
    pub(crate) fn default_with_pressure() -> Context {
        Context {
            name: String::from("ctx0"),
            channel_list: vec![
                Channel {
                    kind: ChannelKind::X,
                    types: ChannelType::Integer,
                    resolution_value: 1000.0,
                    max_value: None,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
                Channel {
                    kind: ChannelKind::Y,
                    types: ChannelType::Integer,
                    resolution_value: 1000.0,
                    max_value: None,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
                Channel {
                    kind: ChannelKind::F,
                    types: ChannelType::Integer,
                    resolution_value: 0.0,
                    max_value: Some(ChannelDataEl::Integer(32767)),
                    unit_resolution: ResolutionUnits::OneOverDev,
                    unit_channel: ChannelUnit::dev,
                },
            ],
        }
    }

    pub fn create_empty(name: String) -> Context {
        Context {
            name,
            channel_list: vec![],
        }
    }

    pub fn channel_exists(&self, channel_kind: ChannelKind) -> Option<usize> {
        self.channel_list
            .clone()
            .into_iter()
            .enumerate()
            .find(|(_, x)| x.kind == channel_kind)
            .map(|(index, _)| index)
    }
}

impl Writable for Context {
    fn write<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<(), Error> {
        // context block
        writer.write(XmlEvent::start_element("context").attr("xml:id", &self.name))?;

        // ink source
        writer.write(XmlEvent::start_element("inkSource").attr("xml:id", "inkSrc0"))?;

        // trace format
        writer.write(XmlEvent::start_element("traceFormat"))?;

        // iterate over channels
        for channel in &self.channel_list {
            if channel.max_value.is_some() {
                writer.write(
                    XmlEvent::start_element("channel")
                        .attr("name", &String::from(channel.kind.clone()))
                        .attr("type", &String::from(channel.types.clone()))
                        .attr(
                            "max",
                            &String::from(channel.max_value.as_ref().unwrap().clone()),
                        )
                        .attr("unit", &String::from(channel.unit_channel.clone())),
                )?;
            } else {
                writer.write(
                    XmlEvent::start_element("channel")
                        .attr("name", &String::from(channel.kind.clone()))
                        .attr("type", &String::from(channel.types.clone()))
                        .attr("unit", &String::from(channel.unit_channel.clone())),
                )?;
            }
            writer.write(XmlEvent::end_element())?;
        }
        writer.write(XmlEvent::end_element())?; // end trace format

        // channelProperties :
        writer.write(XmlEvent::start_element("channelProperties"))?;

        for channel in &self.channel_list {
            writer.write(
                XmlEvent::start_element("channelProperty")
                    .attr("channel", &String::from(channel.kind.clone()))
                    .attr("name", "resolution")
                    .attr("value", &format!("{:?}", channel.resolution_value))
                    .attr("units", &String::from(channel.unit_resolution.clone())),
            )?;
            writer.write(XmlEvent::end_element())?;
        }
        writer.write(XmlEvent::end_element())?; // end channelProperties
        writer.write(XmlEvent::end_element())?; // end ink source
        writer.write(XmlEvent::end_element())?; // end context
        Ok(())
    }
}
