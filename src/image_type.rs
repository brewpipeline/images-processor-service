use crate::*;

use image::{DynamicImage, ImageFormat, ImageResult};
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum ImageType {
    Normal,
    Thumbnail { nwidth: u32, nheight: u32 },
}

impl ImageType {
    pub fn name(&self, name: &String) -> String {
        format!("{name}_{self}")
    }
    pub fn file_name_and_format(&self, name: &String) -> (String, ImageFormat) {
        (
            format!("{name}.webp", name = self.name(name)),
            ImageFormat::WebP,
        )
    }
    pub fn extern_location(&self, name: &String) -> String {
        format!(
            "{EXTERN_LOCATION_IMAGES_STORAGE_PATH}{file_name}",
            file_name = self.file_name_and_format(&name).0
        )
    }
    pub fn local_location(&self, name: &String) -> String {
        format!(
            "{LOCAL_IMAGES_STORAGE_PATH}{file_name}",
            file_name = self.file_name_and_format(&name).0
        )
    }
    pub fn is_local_image_exists(&self, name: &String) -> bool {
        Path::new(&self.local_location(name)).exists()
    }
    pub fn process_and_save_with_name(
        &self,
        name: &String,
        image: DynamicImage,
    ) -> ImageResult<()> {
        let image = match self {
            ImageType::Normal => image,
            ImageType::Thumbnail { nwidth, nheight } => image.thumbnail(*nwidth, *nheight),
        };
        let (file_name, format) = self.file_name_and_format(name);
        image.save_with_format(format!("{LOCAL_IMAGES_STORAGE_PATH}{file_name}"), format)
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
