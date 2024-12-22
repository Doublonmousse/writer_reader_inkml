use crate::brushes::BrushCollection;
use crate::context::Context;
use crate::traits::Writable;
use crate::{brushes::Brush, trace_data::FormattedStroke};
#[cfg(feature = "clipboard")]
use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use xml::writer::{EmitterConfig, XmlEvent};

pub fn writer(stroke_data: Vec<(FormattedStroke, Brush)>) -> anyhow::Result<Vec<u8>> {
    // create brushes
    let mut brush_collection = BrushCollection::default();
    for (_, brush) in &stroke_data {
        brush_collection.add_brush(brush);
    }

    let mut out_v: Vec<u8> = vec![];
    let mut writer = EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(false)
        .create_writer(&mut out_v);

    // xmls : InkML
    writer.write(XmlEvent::start_element("ink").default_ns("http://www.w3.org/2003/InkML"))?;

    // definitions block
    // contains :
    // context/inksource/traceFormat
    //  - name of channels, encoding and units
    // context/inksource/channelProperties
    //  - more properties, resolution and units (if integer encoded, what's 1 in cm !)
    // brush list
    // - width, height, color, ignorePressure
    writer.write(XmlEvent::start_element("definitions"))?;

    let context = Context::default_with_pressure();
    context.write(&mut writer)?;

    // write all brushes
    for (_, brush) in brush_collection.brushes() {
        brush.write(&mut writer)?;
    }
    writer.write(XmlEvent::end_element())?; // end definitions

    // iterate over strokes
    //add trace element with some contextRef and brushRef
    // we also need to iterate on positions + convert with the correct
    // value (depending on resolution and units for source and end !)

    for ((formatted_stroke, _), brush_id) in stroke_data.into_iter().zip(brush_collection.mapping())
    {
        // we are using the NEW brush id here
        writer.write(
            XmlEvent::start_element("trace")
                .attr("contextRef", format!("#{}", context.name).as_str())
                .attr("brushRef", format!("#{}", brush_id).as_str()),
        )?;

        formatted_stroke.write(&mut writer)?;
    }

    writer.write(XmlEvent::end_element())?; // end ink

    // copy to clipboard (for testing purposes only)
    #[cfg(feature = "clipboard")]
    {
        let mimetype = String::from("InkML Format");
        let content: Vec<ClipboardContent> =
            vec![ClipboardContent::Other(mimetype, out_v.to_owned())];
        let ctx = ClipboardContext::new()?;
        let _ = ctx.set(content);
    }
    Ok(out_v)
}
