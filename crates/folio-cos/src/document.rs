//! CosDoc — low-level PDF document access.
//!
//! Provides access to the PDF's object graph via the cross-reference table.

use crate::object::{ObjectId, PdfObject, PdfStream};
use crate::parser;
use crate::serialize;
use crate::tokenizer::{Token, Tokenizer};
use crate::xref::{self, XrefEntry, XrefTable};
use folio_core::{FolioError, Result};
use indexmap::IndexMap;
use std::collections::HashMap;

/// A low-level PDF document providing access to the COS object graph.
pub struct CosDoc {
    /// The raw PDF data (for reading existing documents).
    data: Vec<u8>,
    /// The cross-reference table.
    xref: XrefTable,
    /// Cache of already-parsed objects.
    object_cache: HashMap<u32, PdfObject>,
    /// Newly created or modified objects (not yet saved).
    modified_objects: HashMap<u32, PdfObject>,
    /// Next available object number.
    next_obj_num: u32,
    /// Whether the document has been modified.
    is_modified: bool,
}

impl CosDoc {
    /// Open a PDF document from raw bytes.
    pub fn open(data: Vec<u8>) -> Result<Self> {
        // Verify PDF header
        if !data.starts_with(b"%PDF-") {
            return Err(FolioError::Parse {
                offset: 0,
                message: "Not a PDF file (missing %PDF- header)".into(),
            });
        }

        // Parse cross-reference table(s)
        let xref = xref::parse_all_xrefs(&data)?;

        let next_obj_num = xref
            .trailer
            .get(b"Size".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as u32;

        Ok(Self {
            data,
            xref,
            object_cache: HashMap::new(),
            modified_objects: HashMap::new(),
            next_obj_num,
            is_modified: false,
        })
    }

    /// Open a PDF document from a file path.
    pub fn open_file(path: &str) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::open(data)
    }

    /// Create a new empty PDF document.
    pub fn new() -> Self {
        let mut trailer = IndexMap::new();
        trailer.insert(b"Size".to_vec(), PdfObject::Integer(1));

        Self {
            data: Vec::new(),
            xref: XrefTable {
                entries: IndexMap::new(),
                trailer,
            },
            object_cache: HashMap::new(),
            modified_objects: HashMap::new(),
            next_obj_num: 1,
            is_modified: true,
        }
    }

    /// Get the trailer dictionary.
    pub fn trailer(&self) -> &IndexMap<Vec<u8>, PdfObject> {
        &self.xref.trailer
    }

    /// Get a mutable reference to the trailer dictionary.
    pub fn trailer_mut(&mut self) -> &mut IndexMap<Vec<u8>, PdfObject> {
        &mut self.xref.trailer
    }

    /// Get an object by its object number.
    ///
    /// Returns the object directly (resolves the xref entry to load from file).
    pub fn get_object(&mut self, obj_num: u32) -> Result<Option<&PdfObject>> {
        // Check modified objects first
        if self.modified_objects.contains_key(&obj_num) {
            return Ok(self.modified_objects.get(&obj_num));
        }

        // Check cache
        if self.object_cache.contains_key(&obj_num) {
            return Ok(self.object_cache.get(&obj_num));
        }

        // Load from xref
        let entry = match self.xref.entries.get(&obj_num) {
            Some(e) => *e,
            None => return Ok(None),
        };

        match entry {
            XrefEntry::InUse { offset, .. } => {
                let (_id, obj) = parser::parse_indirect_object_at(&self.data, offset as usize)?;
                self.object_cache.insert(obj_num, obj);
                Ok(self.object_cache.get(&obj_num))
            }
            XrefEntry::Free { .. } => Ok(None),
            XrefEntry::Compressed { stream_obj, .. } => {
                // Object is stored in an Object Stream (/Type /ObjStm).
                // We need to load the stream, decompress it, and extract the object.
                self.load_object_stream(stream_obj)?;
                Ok(self.object_cache.get(&obj_num))
            }
        }
    }

    /// Resolve a PdfObject::Reference to the referenced object.
    /// Returns the object itself if it's not a reference.
    pub fn resolve(&mut self, obj: &PdfObject) -> Result<PdfObject> {
        match obj {
            PdfObject::Reference(id) => match self.get_object(id.num)? {
                Some(resolved) => Ok(resolved.clone()),
                None => Ok(PdfObject::Null),
            },
            _ => Ok(obj.clone()),
        }
    }

    /// Load and decompress an Object Stream (/Type /ObjStm), caching all
    /// contained objects into `object_cache`.
    ///
    /// An object stream packs multiple non-stream objects into a single stream.
    /// Format: the stream data begins with N pairs of "obj_num offset" integers,
    /// followed by the serialized objects. /First gives the byte offset in the
    /// decoded data where the objects begin (after the integer pairs).
    fn load_object_stream(&mut self, stream_obj_num: u32) -> Result<()> {
        // Avoid infinite recursion: if we're already loading this stream, bail
        if self.object_cache.contains_key(&stream_obj_num) {
            return Ok(());
        }

        // Load the object stream itself (it must be a regular InUse object)
        let entry = match self.xref.entries.get(&stream_obj_num) {
            Some(XrefEntry::InUse { offset, .. }) => *offset,
            _ => {
                return Err(FolioError::InvalidObject(format!(
                    "Object stream {} not found or not InUse",
                    stream_obj_num
                )));
            }
        };

        let (_id, stream_obj) = parser::parse_indirect_object_at(&self.data, entry as usize)?;
        let stream = match &stream_obj {
            PdfObject::Stream(s) => s,
            _ => {
                return Err(FolioError::InvalidObject(format!(
                    "Object {} is not a stream (expected ObjStm)",
                    stream_obj_num
                )));
            }
        };

        // Cache the stream object itself
        self.object_cache.insert(stream_obj_num, stream_obj.clone());

        // Get /N (number of objects) and /First (byte offset of first object)
        let n = stream
            .dict
            .get(b"N".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as usize;
        let first = stream
            .dict
            .get(b"First".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as usize;

        // Decode the stream data
        let decoded = self.decode_stream(stream)?;

        if decoded.is_empty() || n == 0 {
            return Ok(());
        }

        // Parse the N pairs of (obj_num, offset) from the beginning of decoded data
        let header = &decoded[..first.min(decoded.len())];
        let mut tokenizer = Tokenizer::new_at(header, 0);
        let mut obj_entries: Vec<(u32, usize)> = Vec::with_capacity(n);

        for _ in 0..n {
            let obj_num = match tokenizer.next_token()? {
                Some(Token::Integer(num)) => num as u32,
                _ => break,
            };
            let offset = match tokenizer.next_token()? {
                Some(Token::Integer(off)) => off as usize,
                _ => break,
            };
            obj_entries.push((obj_num, offset));
        }

        // Parse each object from the data section (starting at /First offset)
        let objects_data = &decoded[first.min(decoded.len())..];

        for (i, &(obj_num, offset)) in obj_entries.iter().enumerate() {
            // Determine the end of this object's data
            let end = if i + 1 < obj_entries.len() {
                obj_entries[i + 1].1
            } else {
                objects_data.len()
            };

            if offset >= objects_data.len() {
                continue;
            }

            let obj_data = &objects_data[offset..end.min(objects_data.len())];
            let mut obj_tokenizer = Tokenizer::new_at(obj_data, 0);

            match parser::parse_object(&mut obj_tokenizer) {
                Ok(Some(obj)) => {
                    self.object_cache.insert(obj_num, obj);
                }
                Ok(None) => {
                    self.object_cache.insert(obj_num, PdfObject::Null);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse object {} from ObjStm {}: {}",
                        obj_num,
                        stream_obj_num,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Create a new indirect object and return its ObjectId.
    pub fn create_indirect(&mut self, obj: PdfObject) -> ObjectId {
        let id = ObjectId::new(self.next_obj_num, 0);
        self.modified_objects.insert(self.next_obj_num, obj);
        self.next_obj_num += 1;
        self.is_modified = true;
        id
    }

    /// Update an existing indirect object.
    pub fn update_object(&mut self, obj_num: u32, obj: PdfObject) {
        self.modified_objects.insert(obj_num, obj);
        self.is_modified = true;
    }

    /// Get the number of entries in the xref table.
    pub fn xref_size(&self) -> u32 {
        self.next_obj_num
    }

    /// Check if the document has been modified.
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Save the document to bytes (full save, not incremental).
    ///
    /// All objects are written as a flat traditional xref table,
    /// even if the original used xref streams or object streams.
    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>> {
        // First, eagerly load all compressed objects into cache
        let compressed_entries: Vec<(u32, u32)> = self
            .xref
            .entries
            .iter()
            .filter_map(|(&num, entry)| match entry {
                XrefEntry::Compressed { stream_obj, .. } => Some((num, *stream_obj)),
                _ => None,
            })
            .collect();

        for (_obj_num, stream_obj) in &compressed_entries {
            if !self.object_cache.contains_key(stream_obj) {
                let _ = self.load_object_stream(*stream_obj);
            }
        }

        let mut objects: Vec<(ObjectId, PdfObject)> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect objects from xref entries
        for (&obj_num, entry) in &self.xref.entries {
            if seen.contains(&obj_num) {
                continue;
            }

            match entry {
                XrefEntry::InUse { offset, .. } => {
                    let obj = if let Some(modified) = self.modified_objects.get(&obj_num) {
                        modified.clone()
                    } else if let Some(cached) = self.object_cache.get(&obj_num) {
                        cached.clone()
                    } else if let Ok((_id, obj)) =
                        parser::parse_indirect_object_at(&self.data, *offset as usize)
                    {
                        obj
                    } else {
                        continue;
                    };

                    // Skip object streams and xref streams — their contents are
                    // written as regular objects in the flat output.
                    let is_objstm_or_xref = obj
                        .dict_get_name(b"Type")
                        .is_some_and(|t| t == b"ObjStm" || t == b"XRef");
                    if !is_objstm_or_xref {
                        objects.push((ObjectId::new(obj_num, 0), obj));
                        seen.insert(obj_num);
                    }
                }
                XrefEntry::Compressed { .. } => {
                    // Objects from object streams — should now be in cache
                    if let Some(obj) = self.object_cache.get(&obj_num) {
                        objects.push((ObjectId::new(obj_num, 0), obj.clone()));
                        seen.insert(obj_num);
                    }
                }
                XrefEntry::Free { .. } => {}
            }
        }

        // Add newly created objects
        for (&obj_num, obj) in &self.modified_objects {
            if !seen.contains(&obj_num) {
                objects.push((ObjectId::new(obj_num, 0), obj.clone()));
            }
        }

        objects.sort_by_key(|(id, _)| id.num);

        // Clean the trailer: remove xref-stream-specific keys and /Prev
        // (since we're writing a traditional xref table, not a stream)
        let mut clean_trailer = self.xref.trailer.clone();
        for key in &[
            b"Prev".as_slice(),
            b"W".as_slice(),
            b"Index".as_slice(),
            b"Filter".as_slice(),
            b"DecodeParms".as_slice(),
            b"Length".as_slice(),
            b"Type".as_slice(),
            b"XRefStm".as_slice(),
        ] {
            clean_trailer.shift_remove(*key);
        }

        serialize::serialize_pdf(&objects, &clean_trailer)
    }

    /// Save the document to a file.
    pub fn save_to_file(&mut self, path: &str) -> Result<()> {
        let data = self.save_to_bytes()?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Decode a stream object's data using its filters.
    pub fn decode_stream(&self, stream: &PdfStream) -> Result<Vec<u8>> {
        if stream.decoded {
            return Ok(stream.data.clone());
        }

        let filter_names = self.get_stream_filters(stream);
        let params = self.get_stream_filter_params(stream);

        if filter_names.is_empty() {
            return Ok(stream.data.clone());
        }

        folio_filters::decode_filter_chain(&filter_names, &stream.data, &params)
    }

    /// Get the filter names for a stream.
    fn get_stream_filters(&self, stream: &PdfStream) -> Vec<Vec<u8>> {
        match stream.dict.get(b"Filter".as_slice()) {
            Some(PdfObject::Name(name)) => vec![name.clone()],
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|obj| obj.as_name().map(|n| n.to_vec()))
                .collect(),
            _ => vec![],
        }
    }

    /// Get the decode parameters for a stream's filters.
    fn get_stream_filter_params(
        &self,
        stream: &PdfStream,
    ) -> Vec<Option<folio_filters::FilterParams>> {
        let filters = self.get_stream_filters(stream);
        let params_obj = stream.dict.get(b"DecodeParms".as_slice());

        match params_obj {
            Some(PdfObject::Dict(d)) => {
                vec![Some(dict_to_filter_params(d)); filters.len().max(1)]
            }
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .map(|obj| obj.as_dict().map(dict_to_filter_params))
                .collect(),
            _ => vec![None; filters.len()],
        }
    }
}

/// Convert a PDF dictionary to FilterParams.
fn dict_to_filter_params(dict: &IndexMap<Vec<u8>, PdfObject>) -> folio_filters::FilterParams {
    folio_filters::FilterParams {
        predictor: dict
            .get(b"Predictor".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        colors: dict
            .get(b"Colors".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        bits_per_component: dict
            .get(b"BitsPerComponent".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(8) as i32,
        columns: dict
            .get(b"Columns".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        early_change: dict
            .get(b"EarlyChange".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let doc = CosDoc::new();
        assert_eq!(doc.xref_size(), 1);
        assert!(doc.is_modified());
    }

    #[test]
    fn test_create_indirect() {
        let mut doc = CosDoc::new();
        let id = doc.create_indirect(PdfObject::Integer(42));
        assert_eq!(id.num, 1);
        assert_eq!(id.gen_num, 0);

        let obj = doc.get_object(1).unwrap().unwrap();
        assert_eq!(obj.as_i64(), Some(42));
    }

    #[test]
    fn test_open_minimal_pdf() {
        // Minimal valid PDF
        let pdf = build_minimal_pdf();
        let mut doc = CosDoc::open(pdf).unwrap();

        // Check trailer
        let root_ref = doc
            .trailer()
            .get(b"Root".as_slice())
            .unwrap()
            .as_reference()
            .unwrap();
        assert_eq!(root_ref.num, 1);

        // Check catalog
        let catalog = doc.get_object(1).unwrap().unwrap();
        assert_eq!(catalog.dict_get_name(b"Type"), Some(b"Catalog".as_slice()));
    }

    fn build_minimal_pdf() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");

        let obj1_offset = buf.len();
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        let obj2_offset = buf.len();
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");

        let xref_offset = buf.len();
        buf.extend_from_slice(b"xref\n0 3\n");
        buf.extend_from_slice(b"0000000000 65535 f \n");
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        buf.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\n");
        buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

        buf
    }
}
