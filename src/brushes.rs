use std::collections::HashMap;
use std::io::Write;
use xml::writer::{Error, EventWriter, XmlEvent};

#[derive(Debug)]
pub(crate) struct Brush {
    /// name for the brush
    /// ```html
    /// <brush xml:id="name">
    /// ```
    name: String,
    /// RGB triplet
    pub color: (u8, u8, u8),
    // simplified version, the stroke width is
    // given as a positive float corresponding to the width in
    // mm
    pub stroke_width: f64,
    pub ignorepressure: bool,
    pub transparency: u8,
}

impl Brush {
    pub(crate) fn init_brush_with_id(id: &String) -> Brush {
        Brush {
            name: id.clone(),
            color: (255, 255, 255),
            stroke_width: 0.0,
            transparency: 0,
            ignorepressure: true,
        }
    }
}

/// We iterate over the strokes and construct a collection of brushes
/// so that we have the lowest number of brushes used
///
/// This means we have to create a mapping from a list of strokes to brushes
/// and create a growing collection of brush so that no one brush is repeated
/// twice
///
/// For now this isn't very useful. Depending on what we add to this structure
/// we may have some more efficient ways to make sure a brush doesn't already exist
#[derive(Default, Debug)]
pub(crate) struct BrushCollection {
    pub brushes: HashMap<String, Brush>,
}

impl Brush {
    pub fn init(
        name: String,
        color: (u8, u8, u8),
        ignorepressure: bool,
        transparency: u8,
        stroke_width: f64,
    ) -> Brush {
        Brush {
            name,
            color,
            stroke_width: stroke_width,
            transparency: transparency,
            ignorepressure: ignorepressure,
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
        // transparency doesn't seem to work well on export
        // so we reserve the field for import only
        // if self.transparency > 0 {
        //     writer.write(
        //         XmlEvent::start_element("brushProperty")
        //             .attr("name", "transparency")
        //             .attr("value", &format!("{:?}", self.transparency)),
        //     )?;
        //     writer.write(XmlEvent::end_element())?;
        //     writer.write(
        //         XmlEvent::start_element("brushProperty")
        //             .attr("name", "tip")
        //             .attr("value", "rectangle"),
        //     )?;
        //     writer.write(XmlEvent::end_element())?;
        // }

        if self.ignorepressure {
            writer.write(
                XmlEvent::start_element("brushProperty")
                    .attr("name", "ignorePressure")
                    .attr("value", "1"),
            )?;
            writer.write(XmlEvent::end_element())?;
        }

        writer.write(XmlEvent::end_element())?; //close brush

        Ok(())
    }
}
