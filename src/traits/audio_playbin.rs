use anyhow::{Context, Result};
use glib::FlagsClass;
use glib::ObjectExt;
use gst::Element;

/// This trait provides a `disable_video()` helper method for
/// GStreamer `playbin` elements.
pub trait AudioPlaybin {
    fn disable_video(&self) -> Result<()>;
}

impl AudioPlaybin for Element {
    fn disable_video(&self) -> Result<()> {
        let flags = self.property("flags")?;
        let flags_class = FlagsClass::new(flags.type_()).context("FlagsClass")?;
        let flags = flags_class
            .builder_with_value(flags)
            .context("FlagsBuilder")?
            .unset_by_nick("video")
            .build()
            .context("Flags.Build")?;
        self.set_property_from_value("flags", &flags)?;
        Ok(())
    }
}
