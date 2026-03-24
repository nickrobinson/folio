//! AcroForm — document-level form management.

use super::field::Field;
use crate::core::{FolioError, Result};
use crate::cos::{CosDoc, PdfObject};

/// Access to a PDF document's interactive form fields.
pub struct AcroForm;

impl AcroForm {
    /// Get all form fields in the document, walking the field tree.
    pub fn get_fields(doc: &mut CosDoc) -> Result<Vec<Field>> {
        let catalog_ref = doc
            .trailer()
            .get(b"Root".as_slice())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| FolioError::InvalidObject("No /Root in trailer".into()))?;

        let catalog = doc
            .get_object(catalog_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Catalog not found".into()))?
            .clone();

        let acroform = match catalog.dict_get(b"AcroForm") {
            Some(PdfObject::Reference(id)) => doc.get_object(id.num)?.cloned().unwrap_or_default(),
            Some(obj) => obj.clone(),
            None => return Ok(Vec::new()),
        };

        let fields_array = match acroform.dict_get(b"Fields") {
            Some(PdfObject::Array(arr)) => arr.clone(),
            _ => return Ok(Vec::new()),
        };

        let mut result = Vec::new();
        for field_ref in &fields_array {
            Self::collect_fields(field_ref, doc, "", &mut result);
        }

        Ok(result)
    }

    /// Recursively collect fields from the field tree.
    fn collect_fields(
        field_ref: &PdfObject,
        doc: &mut CosDoc,
        parent_name: &str,
        result: &mut Vec<Field>,
    ) {
        let (id, field_obj) = match field_ref {
            PdfObject::Reference(id) => match doc.get_object(id.num).ok().flatten().cloned() {
                Some(obj) => (Some(*id), obj),
                None => return,
            },
            obj => (None, obj.clone()),
        };

        let dict = match field_obj.as_dict() {
            Some(d) => d.clone(),
            None => return,
        };

        let field = Field::from_dict(dict.clone(), id, parent_name);

        // Check if this node has /Kids (children)
        let kids = dict
            .get(b"Kids".as_slice())
            .and_then(|o| o.as_array())
            .map(|a| a.to_vec());

        // A field with /FT is a terminal field (may also have Kids for widgets)
        let has_ft = dict.contains_key(b"FT".as_slice());
        // A field with /Kids but no /FT is an intermediate node
        let has_kids = kids.as_ref().is_some_and(|k| !k.is_empty());

        if has_ft {
            // This is a field — add it
            result.push(field);
        } else if has_kids {
            // Intermediate node — recurse into kids
            let current_name = field.name().to_string();
            if let Some(kids) = kids {
                for kid in &kids {
                    Self::collect_fields(kid, doc, &current_name, result);
                }
            }
        } else {
            // Widget-only node (merged field+widget without /FT)
            // Check parent for /FT
            result.push(field);
        }
    }

    /// Find a field by its fully qualified name.
    pub fn find_field(doc: &mut CosDoc, name: &str) -> Result<Option<Field>> {
        let fields = Self::get_fields(doc)?;
        Ok(fields.into_iter().find(|f| f.name() == name))
    }

    /// Get the number of form fields in the document.
    pub fn field_count(doc: &mut CosDoc) -> Result<usize> {
        Ok(Self::get_fields(doc)?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::field::FieldType;

    fn build_form_pdf() -> Vec<u8> {
        // Build a minimal PDF with a text field
        let mut buf = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n");

        let obj1_off = buf.len();
        buf.extend_from_slice(
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>\nendobj\n",
        );

        let obj2_off = buf.len();
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        let obj3_off = buf.len();
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
        );

        let obj4_off = buf.len();
        buf.extend_from_slice(b"4 0 obj\n<< /Fields [5 0 R 6 0 R] >>\nendobj\n");

        let obj5_off = buf.len();
        buf.extend_from_slice(b"5 0 obj\n<< /FT /Tx /T (name) /V (John Doe) >>\nendobj\n");

        let obj6_off = buf.len();
        buf.extend_from_slice(b"6 0 obj\n<< /FT /Btn /T (agree) /V /Yes /Ff 0 >>\nendobj\n");

        let xref_off = buf.len();
        buf.extend_from_slice(b"xref\n0 7\n");
        buf.extend_from_slice(b"0000000000 65535 f \n");
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj1_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj2_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj3_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj4_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj5_off).as_bytes());
        buf.extend_from_slice(format!("{:010} 00000 n \n", obj6_off).as_bytes());
        buf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\n");
        buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_off).as_bytes());

        buf
    }

    #[test]
    fn test_get_fields() {
        let data = build_form_pdf();
        let mut doc = CosDoc::open(data).unwrap();
        let fields = AcroForm::get_fields(&mut doc).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name(), "name");
        assert_eq!(fields[0].field_type(), FieldType::Text);
        assert_eq!(fields[0].value(), Some("John Doe".into()));
        assert_eq!(fields[1].name(), "agree");
        assert_eq!(fields[1].field_type(), FieldType::CheckBox);
        assert_eq!(fields[1].value(), Some("Yes".into()));
    }

    #[test]
    fn test_find_field() {
        let data = build_form_pdf();
        let mut doc = CosDoc::open(data).unwrap();
        let field = AcroForm::find_field(&mut doc, "name").unwrap();
        assert!(field.is_some());
        assert_eq!(field.unwrap().value(), Some("John Doe".into()));

        let missing = AcroForm::find_field(&mut doc, "nonexistent").unwrap();
        assert!(missing.is_none());
    }
}
