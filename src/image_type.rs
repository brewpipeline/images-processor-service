use crate::*;

use image::{DynamicImage, ImageFormat};
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    Normal,
    Thumbnail { nwidth: u32, nheight: u32 },
}

impl ImageType {
    pub fn name(&self, name: &String) -> String {
        format!("{name}_{self}")
    }
    pub fn file_name(&self, name: &String) -> String {
        format!("{name}.webp", name = self.name(name))
    }
    pub fn file_format(&self) -> ImageFormat {
        ImageFormat::WebP
    }
    pub fn extern_path(&self, name: &String) -> String {
        format!(
            "{EXTERN_LOCATION_IMAGES_STORAGE_PATH}{file_name}",
            file_name = self.file_name(&name)
        )
    }
    pub fn local_path(&self, name: &String) -> String {
        format!(
            "{LOCAL_IMAGES_STORAGE_PATH}{file_name}",
            file_name = self.file_name(&name)
        )
    }
    pub fn process_image(&self, image: DynamicImage) -> DynamicImage {
        match self {
            ImageType::Normal => image,
            ImageType::Thumbnail { nwidth, nheight } => image.thumbnail(*nwidth, *nheight),
        }
    }
}

impl fmt::Display for ImageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImageType::Normal => write!(f, "normal"),
            ImageType::Thumbnail { nwidth, nheight } => write!(f, "thumbnail_{nwidth}_{nheight}"),
        }
    }
}
