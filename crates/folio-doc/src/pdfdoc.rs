//! PdfDoc — high-level PDF document.

use folio_core::{FolioError, Rect, Result};
use folio_cos::{CosDoc, ObjectId, PdfObject};
use indexmap::IndexMap;

use crate::info::DocInfo;
use crate::page::Page;

/// A high-level PDF document.
///
/// This wraps `CosDoc` and provides PDF-specific operations like
/// page management, metadata access, and save.
pub struct PdfDoc {
    cos: CosDoc,
}

impl PdfDoc {
    /// Create a new empty PDF document.
    pub fn new() -> Result<Self> {
        let mut cos = CosDoc::new();

        // Create Pages dict
        let mut pages_dict = IndexMap::new();
        pages_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Pages".to_vec()));
        pages_dict.insert(b"Kids".to_vec(), PdfObject::Array(vec![]));
        pages_dict.insert(b"Count".to_vec(), PdfObject::Integer(0));
        let pages_id = cos.create_indirect(PdfObject::Dict(pages_dict));

        // Create Catalog
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Catalog".to_vec()));
        catalog_dict.insert(b"Pages".to_vec(), PdfObject::Reference(pages_id));
        let catalog_id = cos.create_indirect(PdfObject::Dict(catalog_dict));

        // Set trailer Root
        cos.trailer_mut()
            .insert(b"Root".to_vec(), PdfObject::Reference(catalog_id));

        Ok(Self { cos })
    }

    /// Open a PDF document from a file path.
    pub fn open(path: &str) -> Result<Self> {
        let cos = CosDoc::open_file(path)?;
        Ok(Self { cos })
    }

    /// Open a PDF document from bytes.
    pub fn open_from_bytes(data: Vec<u8>) -> Result<Self> {
        let cos = CosDoc::open(data)?;
        Ok(Self { cos })
    }

    /// Get the underlying CosDoc for low-level access.
    pub fn cos(&self) -> &CosDoc {
        &self.cos
    }

    /// Get the underlying CosDoc mutably.
    pub fn cos_mut(&mut self) -> &mut CosDoc {
        &mut self.cos
    }

    /// Get the catalog dictionary's object reference.
    fn catalog_ref(&self) -> Result<ObjectId> {
        self.cos
            .trailer()
            .get(b"Root".as_slice())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| FolioError::InvalidObject("Missing /Root in trailer".into()))
    }

    /// Get the Pages dictionary reference from the catalog.
    fn pages_ref(&mut self) -> Result<ObjectId> {
        let catalog_ref = self.catalog_ref()?;
        let catalog = self
            .cos
            .get_object(catalog_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Catalog not found".into()))?
            .clone();

        catalog
            .dict_get(b"Pages")
            .and_then(|o| o.as_reference())
            .ok_or_else(|| FolioError::InvalidObject("Missing /Pages in catalog".into()))
    }

    /// Get the number of pages in the document.
    pub fn page_count(&mut self) -> Result<u32> {
        let pages_ref = self.pages_ref()?;
        let pages = self
            .cos
            .get_object(pages_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Pages dict not found".into()))?
            .clone();

        pages
            .dict_get_i64(b"Count")
            .map(|c| c as u32)
            .ok_or_else(|| FolioError::InvalidObject("Missing /Count in Pages".into()))
    }

    /// Get a page by 1-based index.
    pub fn get_page(&mut self, page_num: u32) -> Result<Page> {
        if page_num == 0 {
            return Err(FolioError::InvalidArgument(
                "Page numbers are 1-based".into(),
            ));
        }

        let page_refs = self.collect_page_refs()?;
        let index = (page_num - 1) as usize;

        if index >= page_refs.len() {
            return Err(FolioError::InvalidArgument(format!(
                "Page {} out of range (document has {} pages)",
                page_num,
                page_refs.len()
            )));
        }

        let page_ref = page_refs[index];
        let page_obj = self
            .cos
            .get_object(page_ref.num)?
            .ok_or_else(|| {
                FolioError::InvalidObject(format!("Page object {} not found", page_ref.num))
            })?
            .clone();

        // Resolve inherited attributes from parent Pages nodes.
        // Per PDF spec §7.7.3.4, these keys are inheritable:
        // MediaBox, CropBox, Rotate, Resources
        let page_obj = self.resolve_inherited_attrs(page_obj)?;

        Ok(Page::new(page_ref, page_obj, page_num))
    }

    /// Resolve inherited attributes by walking up the /Parent chain.
    ///
    /// PDF spec §7.7.3.4: MediaBox, CropBox, Rotate, and Resources
    /// are inheritable — if not present on the page dict, they are
    /// inherited from the nearest ancestor Pages node that defines them.
    fn resolve_inherited_attrs(&mut self, page_obj: PdfObject) -> Result<PdfObject> {
        const INHERITABLE: &[&[u8]] = &[b"MediaBox", b"CropBox", b"Rotate", b"Resources"];

        let mut dict = match page_obj.as_dict() {
            Some(d) => d.clone(),
            None => return Ok(page_obj),
        };

        // Check which keys are missing
        let missing: Vec<&[u8]> = INHERITABLE
            .iter()
            .filter(|&&key| !dict.contains_key(key))
            .copied()
            .collect();

        if missing.is_empty() {
            return Ok(page_obj);
        }

        // Walk up the /Parent chain
        let mut parent_ref = dict
            .get(b"Parent".as_slice())
            .and_then(|o| o.as_reference());
        let mut visited = std::collections::HashSet::new();

        while let Some(pref) = parent_ref {
            if visited.contains(&pref.num) {
                break; // prevent cycles
            }
            visited.insert(pref.num);

            let parent = match self.cos.get_object(pref.num)? {
                Some(obj) => obj.clone(),
                None => break,
            };

            let parent_dict = match parent.as_dict() {
                Some(d) => d,
                None => break,
            };

            // Copy missing inheritable keys from this ancestor
            for &key in &missing {
                if !dict.contains_key(key) {
                    if let Some(value) = parent_dict.get(key) {
                        dict.insert(key.to_vec(), value.clone());
                    }
                }
            }

            // If all keys are now resolved, stop
            if INHERITABLE.iter().all(|&key| dict.contains_key(key)) {
                break;
            }

            // Continue up the chain
            parent_ref = parent_dict
                .get(b"Parent".as_slice())
                .and_then(|o| o.as_reference());
        }

        Ok(PdfObject::Dict(dict))
    }

    /// Collect all page object references by walking the page tree.
    fn collect_page_refs(&mut self) -> Result<Vec<ObjectId>> {
        let pages_ref = self.pages_ref()?;
        let mut result = Vec::new();
        self.collect_pages_recursive(pages_ref, &mut result)?;
        Ok(result)
    }

    fn collect_pages_recursive(
        &mut self,
        node_ref: ObjectId,
        result: &mut Vec<ObjectId>,
    ) -> Result<()> {
        let node = self
            .cos
            .get_object(node_ref.num)?
            .ok_or_else(|| {
                FolioError::InvalidObject(format!("Page tree node {} not found", node_ref.num))
            })?
            .clone();

        let type_name = node.dict_get_name(b"Type").unwrap_or(b"");

        match type_name {
            b"Pages" => {
                // Intermediate node — recurse into Kids
                if let Some(kids) = node.dict_get(b"Kids").and_then(|o| o.as_array()) {
                    for kid in kids {
                        if let Some(kid_ref) = kid.as_reference() {
                            self.collect_pages_recursive(kid_ref, result)?;
                        }
                    }
                }
            }
            b"Page" | _ => {
                // Leaf node (a page)
                result.push(node_ref);
            }
        }

        Ok(())
    }

    /// Create a new page with the given media box and add it to the document.
    pub fn create_page(&mut self, media_box: Rect) -> Result<u32> {
        let pages_ref = self.pages_ref()?;

        // Create the page object
        let mut page_dict = IndexMap::new();
        page_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Page".to_vec()));
        page_dict.insert(b"Parent".to_vec(), PdfObject::Reference(pages_ref));
        page_dict.insert(
            b"MediaBox".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(media_box.x1),
                PdfObject::Real(media_box.y1),
                PdfObject::Real(media_box.x2),
                PdfObject::Real(media_box.y2),
            ]),
        );
        let page_id = self.cos.create_indirect(PdfObject::Dict(page_dict));

        // Add to Pages Kids array and increment Count
        let pages = self
            .cos
            .get_object(pages_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Pages not found".into()))?
            .clone();

        let mut pages_dict = pages.as_dict().cloned().unwrap_or_default();

        // Update Kids
        let mut kids = pages_dict
            .get(b"Kids".as_slice())
            .and_then(|o| o.as_array())
            .map(|a| a.to_vec())
            .unwrap_or_default();
        kids.push(PdfObject::Reference(page_id));
        pages_dict.insert(b"Kids".to_vec(), PdfObject::Array(kids.clone()));

        // Update Count
        pages_dict.insert(b"Count".to_vec(), PdfObject::Integer(kids.len() as i64));

        self.cos
            .update_object(pages_ref.num, PdfObject::Dict(pages_dict));

        Ok(kids.len() as u32)
    }

    /// Get document info (title, author, etc.).
    pub fn doc_info(&mut self) -> Result<DocInfo> {
        let info_ref = self
            .cos
            .trailer()
            .get(b"Info".as_slice())
            .and_then(|o| o.as_reference());

        match info_ref {
            Some(id) => {
                let obj = self
                    .cos
                    .get_object(id.num)?
                    .cloned()
                    .unwrap_or(PdfObject::Null);
                Ok(DocInfo::from_dict(
                    obj.as_dict().cloned().unwrap_or_default(),
                ))
            }
            None => Ok(DocInfo::default()),
        }
    }

    /// Check if the document has been modified.
    pub fn is_modified(&self) -> bool {
        self.cos.is_modified()
    }

    /// Save the document to bytes.
    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>> {
        self.cos.save_to_bytes()
    }

    /// Save the document to a file.
    pub fn save(&mut self, path: &str) -> Result<()> {
        self.cos.save_to_file(path)
    }
}
