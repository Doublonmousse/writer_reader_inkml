use std::io::Write;
use xml::writer::{Error, EventWriter};

pub(crate) trait Writable {
    fn write<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<(), Error>;
}
