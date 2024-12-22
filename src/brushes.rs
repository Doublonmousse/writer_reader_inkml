use std::io::Write;
use std::{collections::HashMap, hash::Hash};
use xml::writer::{Error, EventWriter, XmlEvent};

use crate::traits::Writable;

#[derive(Debug, Clone)]
pub struct Brush {
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
    pub(crate) fn init_brush_with_id(id: &str) -> Brush {
        Brush {
            name: id.to_owned(),
            color: (0, 0, 0),
            stroke_width: 0.0,
            transparency: 0,
            ignorepressure: false,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct PositiveFiniteFloat {
    stroke_width: f64,
}

impl PositiveFiniteFloat {
    fn new(stroke_width: f64) -> PositiveFiniteFloat {
        PositiveFiniteFloat {
            stroke_width: if stroke_width.is_finite() {
                stroke_width
            } else {
                0.0
            },
        }
    }
}

impl Eq for PositiveFiniteFloat {} // works because we guarantee the value is finite

impl Hash for PositiveFiniteFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.stroke_width.to_bits().hash(state);
    }
}

/// Type alias that's used to check brush duplicates using a hashmap
/// - The first element is the (r,g,b) value
/// - The second element is the stroke width
/// - The third is whether or not pressure is ignored
/// - The last one is transparency
type BrushIndex = ((u8, u8, u8), PositiveFiniteFloat, bool, u8);

/// We iterate over the strokes and construct a collection of brushes
/// so that we have the lowest number of brushes used
///
/// This means we have to create a mapping from a list of strokes to brushes
/// and create a growing collection of brush so that no one brush is repeated
/// twice
#[derive(Default, Debug)]
pub(crate) struct BrushCollection {
    /// Brush collection (dictionnary on brush indexed by the brush id)
    brushes: HashMap<String, Brush>,
    /// Called with color, stroke width, ignorepressure and transparency, gives
    /// the id corresponding to this value
    duplicate_search: HashMap<BrushIndex, String>,
    /// Memorizes the brush id given for each call wanting to add a brush
    mapping: Vec<String>,
}

impl BrushCollection {
    pub(crate) fn add_brush(&mut self, brush: &Brush) {
        let duplicate_key = (
            brush.color,
            PositiveFiniteFloat::new(brush.stroke_width),
            brush.ignorepressure,
            brush.transparency,
        );

        match self.duplicate_search.get(&duplicate_key) {
            None => {
                // get the id
                let id = format!("br{}", self.brushes.len() + 1);
                self.mapping.push(id.clone());

                // push to duplicate search
                self.duplicate_search.insert(duplicate_key, id.clone());

                // push to brushes
                // edit the brush to take the new unique id
                let mut new_brush = brush.clone();
                new_brush.name = id.clone();
                self.brushes.insert(id, new_brush);
            }
            Some(id) => {
                self.mapping.push(id.clone());
            }
        }
    }

    pub(crate) fn brushes(&self) -> HashMap<String, Brush> {
        self.brushes.clone()
    }

    pub(crate) fn mapping(&self) -> Vec<String> {
        self.mapping.clone()
    }
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
            stroke_width,
            transparency,
            ignorepressure,
        }
    }
}

impl Writable for Brush {
    /// function to write the brush to the xml file
    fn write<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<(), Error> {
        // add brush
        writer.write(XmlEvent::start_element("brush").attr("xml:id", &self.name))?;

        writer.write(
            XmlEvent::start_element("brushProperty")
                .attr("name", "width")
                .attr("value", &format!("{}", self.stroke_width * 10.0))
                .attr("units", "cm"),
        )?;
        writer.write(XmlEvent::end_element())?;
        writer.write(
            XmlEvent::start_element("brushProperty")
                .attr("name", "height")
                .attr("value", &format!("{}", self.stroke_width * 10.0))
                .attr("units", "cm"),
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
        // transparency work but only with colors != 0,0,0
        if self.transparency > 0 && self.color != (0, 0, 0) {
            writer.write(
                XmlEvent::start_element("brushProperty")
                    .attr("name", "transparency")
                    .attr("value", &format!("{:?}", self.transparency)),
            )?;
            writer.write(XmlEvent::end_element())?;
        }

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
