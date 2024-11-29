use crate::context::Context;
use std::io::Write;
use xml::writer::{Error, EventWriter, XmlEvent};

pub(crate) struct Brush {
    /// name for the brush
    /// <brush xml:id="name">
    name: String,
    /// RGB triplet
    color: (u8, u8, u8),
    // simplified version, the stroke width is
    // given as a positive float corresponding to the width in
    // mm
    stroke_width: f64,
    ignorepressure: bool,
}

/// We iterate over the strokes and construct a collection of brushes
/// so that we have the lowest number of brushes used
///
/// This means we have to create a mapping from a list of strokes to brushes
/// and create a growing collection of brush so that no one brush is repeated
/// twice
pub(crate) struct BrushCollection {
    brushes: Vec<Brush>,
}

impl Brush {
    pub fn init(
        name: String,
        context: &Context,
        color: (u8, u8, u8),
        ignorepressure: bool,
        stroke_width: f64,
    ) -> Brush {
        Brush {
            name: name,
            color: color,
            stroke_width: stroke_width,
            ignorepressure: !context.pressure_channel_exist() || ignorepressure,
        }
    }

    /// function to write the brush to the xml file
    pub fn write_brush<W: Write>(self, writer: &mut EventWriter<W>) -> Result<(), Error> {
        // add brush
        writer.write(XmlEvent::start_element("brush").attr("xml:id", &self.name))?;

        writer.write(
            XmlEvent::start_element("brushProperty")
                .attr("name", "width")
                .attr("value", &format!("{}", self.stroke_width))
                .attr("units", "mm"),
        )?;
        writer.write(XmlEvent::end_element())?;
        writer.write(
            XmlEvent::start_element("brushProperty")
                .attr("name", "height")
                .attr("value", &format!("{}", self.stroke_width))
                .attr("units", "mm"),
        )?;
        writer.write(XmlEvent::end_element())?;
        writer.write(
            XmlEvent::start_element("brushProperty")
                .attr("name", "color")
                .attr(
                    "value",
                    &format!(
                        "#{:02X}{:02X}{:02X}",
                        self.color.0, self.color.1, self.color.2
                    ),
                ),
        )?;
        writer.write(XmlEvent::end_element())?;
        // writer.write(
        //     XmlEvent::start_element("brushProperty")
        //         .attr("name", "ignorePressure")
        //         .attr("value", if self.ignorepressure { "1" } else { "0" }),
        // )?;
        // writer.write(XmlEvent::end_element())?;
        writer.write(XmlEvent::end_element())?; //close brush

        Ok(())
    }
}
