//! PDF image handling — embedding, extracting, and reading image XObjects.
//!
//! # Embedding images
//!
//! ```no_run
//! use folio_pdf::image::PdfImage;
//! use folio_pdf::cos::CosDoc;
//!
//! let mut doc = CosDoc::new();
//! let image = PdfImage::from_file("photo.jpg", &mut doc).unwrap();
//! // image.obj_id() gives you the indirect object reference to use in a content stream
//! ```

mod embed;
mod extract;
mod page_images;
mod xobject;

pub use embed::PdfImage;
pub use extract::extract_image;
pub use page_images::add_image_to_page;
pub use xobject::ImageXObject;
