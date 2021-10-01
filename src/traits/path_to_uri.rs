use std::path::Path;

/// This trait allows for easy conversion of a path to a URI
pub trait PathToURI {
    fn to_uri(&self) -> String;
}

impl PathToURI for Path {
    /// Returns `self` as a URI. Panics in case of an error.
    fn to_uri(&self) -> String {
        glib::filename_to_uri(self, None)
            .expect("Error converting path to URI")
            .to_string()
    }
}
