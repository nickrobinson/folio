//! PDF bookmarks (document outline).

use folio_core::{FolioError, Result};
use folio_cos::{CosDoc, ObjectId, PdfObject};

/// A PDF bookmark (outline item).
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Object ID of this bookmark.
    id: ObjectId,
    /// The bookmark's title.
    title: String,
    /// Reference to the destination or action.
    dest: Option<PdfObject>,
    action: Option<PdfObject>,
    /// Child/sibling/parent references.
    first_child: Option<ObjectId>,
    last_child: Option<ObjectId>,
    next: Option<ObjectId>,
    prev: Option<ObjectId>,
    parent: Option<ObjectId>,
    /// Number of visible descendants (negative = closed).
    count: i64,
    /// Display flags: 1=italic, 2=bold.
    flags: i32,
    /// Color (RGB, 0-1 range).
    color: Option<[f64; 3]>,
}

impl Bookmark {
    /// Load a bookmark from a document.
    pub fn load(obj_num: u32, doc: &mut CosDoc) -> Result<Self> {
        let obj = doc
            .get_object(obj_num)?
            .ok_or_else(|| FolioError::InvalidObject(format!("Bookmark {} not found", obj_num)))?
            .clone();

        let dict = obj
            .as_dict()
            .ok_or_else(|| FolioError::InvalidObject("Bookmark is not a dict".into()))?;

        let title = dict
            .get(b"Title".as_slice())
            .and_then(|o| o.as_str())
            .map(decode_text)
            .unwrap_or_default();

        Ok(Self {
            id: ObjectId::new(obj_num, 0),
            title,
            dest: dict.get(b"Dest".as_slice()).cloned(),
            action: dict.get(b"A".as_slice()).cloned(),
            first_child: dict.get(b"First".as_slice()).and_then(|o| o.as_reference()),
            last_child: dict.get(b"Last".as_slice()).and_then(|o| o.as_reference()),
            next: dict.get(b"Next".as_slice()).and_then(|o| o.as_reference()),
            prev: dict.get(b"Prev".as_slice()).and_then(|o| o.as_reference()),
            parent: dict
                .get(b"Parent".as_slice())
                .and_then(|o| o.as_reference()),
            count: dict
                .get(b"Count".as_slice())
                .and_then(|o| o.as_i64())
                .unwrap_or(0),
            flags: dict
                .get(b"F".as_slice())
                .and_then(|o| o.as_i64())
                .unwrap_or(0) as i32,
            color: dict.get(b"C".as_slice()).and_then(|o| {
                let arr = o.as_array()?;
                if arr.len() >= 3 {
                    Some([arr[0].as_f64()?, arr[1].as_f64()?, arr[2].as_f64()?])
                } else {
                    None
                }
            }),
        })
    }

    /// Get the bookmark title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the object ID.
    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// Whether this bookmark is open (children visible).
    pub fn is_open(&self) -> bool {
        self.count > 0
    }

    /// Whether this bookmark is italic.
    pub fn is_italic(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Whether this bookmark is bold.
    pub fn is_bold(&self) -> bool {
        self.flags & 2 != 0
    }

    /// Get the bookmark color (RGB).
    pub fn color(&self) -> Option<[f64; 3]> {
        self.color
    }

    /// Get the destination object (if any).
    pub fn destination(&self) -> Option<&PdfObject> {
        self.dest.as_ref()
    }

    /// Get the action object (if any).
    pub fn action(&self) -> Option<&PdfObject> {
        self.action.as_ref()
    }

    /// Get the first child bookmark ID.
    pub fn first_child(&self) -> Option<ObjectId> {
        self.first_child
    }

    /// Get the next sibling bookmark ID.
    pub fn next(&self) -> Option<ObjectId> {
        self.next
    }

    /// Get the previous sibling bookmark ID.
    pub fn prev(&self) -> Option<ObjectId> {
        self.prev
    }

    /// Get the parent bookmark ID.
    pub fn parent(&self) -> Option<ObjectId> {
        self.parent
    }

    /// Check if this bookmark has children.
    pub fn has_children(&self) -> bool {
        self.first_child.is_some()
    }

    /// Get all bookmarks in the document as a flat list (depth-first).
    pub fn get_all(doc: &mut CosDoc) -> Result<Vec<(Bookmark, u32)>> {
        let catalog_ref = doc
            .trailer()
            .get(b"Root".as_slice())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| FolioError::InvalidObject("No /Root".into()))?;

        let catalog = doc
            .get_object(catalog_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Catalog not found".into()))?
            .clone();

        let outlines_ref = match catalog.dict_get(b"Outlines") {
            Some(PdfObject::Reference(id)) => *id,
            _ => return Ok(Vec::new()),
        };

        let outlines = doc
            .get_object(outlines_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Outlines not found".into()))?
            .clone();

        let first = match outlines.dict_get(b"First") {
            Some(PdfObject::Reference(id)) => *id,
            _ => return Ok(Vec::new()),
        };

        let mut result = Vec::new();
        Self::collect_bookmarks(first, doc, 0, &mut result)?;
        Ok(result)
    }

    fn collect_bookmarks(
        id: ObjectId,
        doc: &mut CosDoc,
        depth: u32,
        result: &mut Vec<(Bookmark, u32)>,
    ) -> Result<()> {
        let bm = Bookmark::load(id.num, doc)?;
        let next = bm.next;
        let first_child = bm.first_child;
        result.push((bm, depth));

        // Recurse into children
        if let Some(child_id) = first_child {
            Self::collect_bookmarks(child_id, doc, depth + 1, result)?;
        }

        // Continue to next sibling
        if let Some(next_id) = next {
            Self::collect_bookmarks(next_id, doc, depth, result)?;
        }

        Ok(())
    }
}

fn decode_text(data: &[u8]) -> String {
    if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        let mut chars = Vec::new();
        let mut i = 2;
        while i + 1 < data.len() {
            chars.push(((data[i] as u16) << 8) | (data[i + 1] as u16));
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    } else {
        String::from_utf8_lossy(data).into_owned()
    }
}
