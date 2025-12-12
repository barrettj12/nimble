pub mod go;
pub use go::GoBuilder;

use crate::types::Image;

pub trait Builder {
    fn build(&self, path: &std::path::Path) -> Result<Image, Box<dyn std::error::Error>>;
}
