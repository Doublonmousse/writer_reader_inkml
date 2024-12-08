use std::{f32::consts::PI, io};
use xml::writer::{EmitterConfig, XmlEvent};
use crate::context::Context;
use crate::traits::Writable;
use crate::brushes::Brush;

pub fn writer() -> io::Result<()> {
    // let output = io::stdout();
    let mut out_v: Vec<u8> = vec![];
    let mut writer = EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(false)
        .create_writer(&mut out_v);

    // xmls : InkML
    writer
        .write(XmlEvent::start_element("ink").default_ns("http://www.w3.org/2003/InkML"))
        .unwrap();

    // definitions block
    // contains :
    // context/inksource/traceFormat
    //  - name of channels, encoding and units
    // context/inksource/channelProperties
    //  - more properties, resolution and units (if integer encoded, what's 1 in cm !)
    // brush list
    // - width, height, color, ignorePressure
    writer
        .write(XmlEvent::start_element("definitions"))
        .unwrap();

    let context = Context::default();
    context.write(&mut writer).unwrap();

    // collect brushes

    // for now one brush
    let brush = Brush::init(
        String::from("br1"),
        (255, 255, 12),
        !&context.pressure_channel_exist(),
        125,
        0.2,
    );
    // write brushes
    brush.write(&mut writer).unwrap();

    writer.write(XmlEvent::end_element()).unwrap(); // end definitions

    // iterate over strokes
    //add trace element with some contextRef and brushRef
    // we also need to iterate on positions + convert with the correct
    // value (depending on resolution and units for source and end !)
    writer
        .write(
            XmlEvent::start_element("trace")
                .attr("contextRef", "#ctx0")
                .attr("brushRef", "#br1"),
        )
        .unwrap();

    // generate some data here
    let positions: Vec<(f32, f32)> = (1..10)
        .map(|x| {
            (
                (f32::sin(2.0 * PI * (x as f32) / 10.0) + 2.0) * 1000.0,
                (f32::cos(2.0 * PI * (x as f32) / 10.0) + 2.0) * 1000.0,
            )
        })
        .collect();

    let mut string_out = positions
        .into_iter()
        .fold(String::from("#"), |acc, (x, y)| {
            acc + &format!("{:.} {:.},", x.round(), y.round())
        });
    string_out = string_out[1..string_out.len() - 1].to_string();

    // for now this is very basic !
    // we should go through the strokes to
    // add our data
    writer.write(XmlEvent::characters(&string_out)).unwrap();

    writer.write(XmlEvent::end_element()).unwrap(); //end
    writer.write(XmlEvent::end_element()).unwrap(); // end ink

    // collect everything
    println!("Hello, {:?}", String::from_utf8(out_v.clone()));

    // copy to clipboard
    #[cfg(feature = "clipboard")]
    {
        let mimetype = String::from("InkML Format");
        let content: Vec<ClipboardContent> =
            vec![ClipboardContent::Other(mimetype, out_v.to_owned())];
        let ctx = ClipboardContext::new().unwrap();
        let _ = ctx.set(content);
    }
    Ok(())
}
