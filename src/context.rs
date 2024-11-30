// types for the whole program
// for writing we assume we'll use only 1 context
// but we use as many brushes as needed

use crate::trace_data::ChannelDataEl;
use std::io::Write;
use xml::writer::{Error, EventWriter, XmlEvent};

/// types of channel
/// For now we allow X,Y only
#[derive(Clone, PartialEq, Debug)]
#[allow(unused)]
pub(crate) enum ChannelKind {
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
}

impl ChannelKind {
    fn parse(name: &Option<String>) -> Result<ChannelKind, ()> {
        match name {
            Some(value) => match value.as_str() {
                "X" => Ok(ChannelKind::X),
                "Y" => Ok(ChannelKind::Y),
                "F" => Ok(ChannelKind::F),
                "OA" => Ok(ChannelKind::OA),
                "OE" => Ok(ChannelKind::OE),
                _ => Err(()),
            },
            None => Err(()),
        }
    }

    fn get_default_resolution_unit(&self) -> ResolutionUnits {
        match self {
            ChannelKind::X | ChannelKind::Y => ResolutionUnits::OneOverCm,
            ChannelKind::F => ResolutionUnits::OneOverDev,
            ChannelKind::OA | ChannelKind::OE => ResolutionUnits::OneOverDegree,
        }
    }

    fn get_default_unit(&self) -> ChannelUnit {
        match self {
            ChannelKind::X | ChannelKind::Y => ChannelUnit::cm,
            ChannelKind::F => ChannelUnit::dev,
            ChannelKind::OA | ChannelKind::OE => ChannelUnit::deg,
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
        }
    }
}

/// type used for the encoding
#[derive(Clone, Debug)]
#[allow(unused)]
pub(crate) enum ChannelType {
    Integer,
    Decimal,
    Double,
    Bool,
}

impl ChannelType {
    fn parse(name: &Option<String>) -> Result<ChannelType, ()> {
        match name {
            Some(value) => match value.as_str() {
                "integer" => Ok(ChannelType::Integer),
                "decimal" => Ok(ChannelType::Decimal),
                "double" => Ok(ChannelType::Double),
                "boolean" => Ok(ChannelType::Bool),
                _ => Err(()),
            },
            None => Err(()),
        }
    }
}

impl Default for ChannelType {
    fn default() -> Self {
        ChannelType::Decimal
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
enum ResolutionUnits {
    // 1/cm
    OneOverCm,
    // 1/mm
    OneOverMm,
    // 1/dev, dev device unit
    OneOverDev,
    // 1/deg, degree
    OneOverDegree,
}

impl From<ResolutionUnits> for String {
    fn from(value: ResolutionUnits) -> Self {
        match value {
            ResolutionUnits::OneOverCm => String::from("1/cm"),
            ResolutionUnits::OneOverMm => String::from("1/mm"),
            ResolutionUnits::OneOverDev => String::from("1/dev"),
            ResolutionUnits::OneOverDegree => String::from("1/deg"),
        }
    }
}

impl Default for ResolutionUnits {
    fn default() -> Self {
        ResolutionUnits::OneOverCm
    }
}

// TODO : use the full unit list
#[derive(Clone, Debug)]
#[allow(unused, non_camel_case_types)]
enum ChannelUnit {
    /// distance unit, `mm`
    mm,
    /// distance unit, `cm`
    cm,
    /// distance unit, `m`
    m,
    /// device ind unit
    dev,
    /// degree
    deg,
}

impl Default for ChannelUnit {
    fn default() -> Self {
        ChannelUnit::cm
    }
}

impl From<ChannelUnit> for String {
    fn from(value: ChannelUnit) -> Self {
        match value {
            ChannelUnit::mm => String::from("mm"),
            ChannelUnit::cm => String::from("cm"),
            ChannelUnit::m => String::from("m"),
            ChannelUnit::dev => String::from("dev"),
            ChannelUnit::deg => String::from("deg"),
        }
    }
}

impl ChannelUnit {
    fn parse(name: &Option<String>) -> Option<ChannelUnit> {
        match name {
            Some(value) => match value.as_str() {
                "mm" => Some(ChannelUnit::mm),
                "cm" => Some(ChannelUnit::cm),
                "m" => Some(ChannelUnit::m),
                "dev" => Some(ChannelUnit::dev),
                "deg" => Some(ChannelUnit::deg),
                _ => None,
            },
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Channel {
    kind: ChannelKind,
    types: ChannelType,
    // we are forcing this to u32 for now
    resolution_value: u32,
    unit_resolution: ResolutionUnits,
    unit_channel: ChannelUnit,
}

impl Channel {
    pub fn initialise_channel_from_name(
        kind_type_unit_v: Vec<Option<String>>,
    ) -> Result<Channel, ()> {
        let kind = &kind_type_unit_v[0];
        let channel_type = &kind_type_unit_v[1];
        let unit = &kind_type_unit_v[2];

        let channel_kind = ChannelKind::parse(&kind)?;

        Ok(Channel {
            kind: channel_kind.clone(),
            types: ChannelType::parse(&channel_type)?,
            // the rest is there as a default value, maybe we should also have a default value per kind ?
            resolution_value: 1000,
            unit_resolution: channel_kind.get_default_resolution_unit(),
            unit_channel: ChannelUnit::parse(unit).unwrap_or(channel_kind.get_default_unit()),
        })
    }
}

#[derive(Debug)]
pub(crate) struct Context {
    // name given to the context, name = ctx0 by default
    // refered by `contextRef="#ctx0" in the trace attr
    pub name: String,
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
                    resolution_value: 1000,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
                Channel {
                    kind: ChannelKind::Y,
                    types: ChannelType::Integer,
                    resolution_value: 1000,
                    unit_resolution: ResolutionUnits::OneOverCm,
                    unit_channel: ChannelUnit::cm,
                },
            ],
        }
    }
}

impl Context {
    pub fn create_empty(name: String) -> Context {
        Context {
            name: name,
            channel_list: vec![],
        }
    }

    pub fn pressure_channel_exist(&self) -> bool {
        self.channel_list
            .clone()
            .into_iter()
            .fold(false, |acc, x| acc || x.kind == ChannelKind::F)
    }

    pub fn write_context<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<(), Error> {
        // context block
        writer.write(XmlEvent::start_element("context").attr("xml:id", &self.name))?;

        // ink source
        writer
            .write(XmlEvent::start_element("inkSource").attr("xml:id", "inkSrc0"))
            .unwrap();

        // trace format
        writer
            .write(XmlEvent::start_element("traceFormat"))
            .unwrap();

        // iterate over channels
        for channel in &self.channel_list {
            writer.write(
                XmlEvent::start_element("channel")
                    .attr("name", &String::from(channel.kind.clone()))
                    .attr("type", &String::from(channel.types.clone()))
                    .attr("unit", &String::from(channel.unit_channel.clone())),
            )?;
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
