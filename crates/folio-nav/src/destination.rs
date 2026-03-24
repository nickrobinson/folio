//! PDF destinations — specify a page and view.

use folio_cos::{ObjectId, PdfObject};

/// Destination fit types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FitType {
    /// /XYZ left top zoom
    XYZ { left: f64, top: f64, zoom: f64 },
    /// /Fit — fit entire page
    Fit,
    /// /FitH top
    FitH { top: f64 },
    /// /FitV left
    FitV { left: f64 },
    /// /FitR left bottom right top
    FitR {
        left: f64,
        bottom: f64,
        right: f64,
        top: f64,
    },
    /// /FitB — fit bounding box
    FitB,
    /// /FitBH top
    FitBH { top: f64 },
    /// /FitBV left
    FitBV { left: f64 },
}

/// A parsed PDF destination.
#[derive(Debug, Clone)]
pub struct Destination {
    /// The target page reference.
    pub page_ref: Option<ObjectId>,
    /// The fit type and parameters.
    pub fit: FitType,
}

impl Destination {
    /// Parse a destination from a PDF object.
    ///
    /// Destinations can be:
    /// - An array: [page /FitType params...]
    /// - A name (named destination — needs resolution from the Names tree)
    /// - A string (named destination)
    pub fn from_object(obj: &PdfObject) -> Option<Self> {
        let arr = obj.as_array()?;
        if arr.is_empty() {
            return None;
        }

        let page_ref = arr[0].as_reference();
        let fit_name = arr.get(1).and_then(|o| o.as_name()).unwrap_or(b"Fit");

        let f = |idx: usize| -> f64 { arr.get(idx).and_then(|o| o.as_f64()).unwrap_or(0.0) };

        let fit = match fit_name {
            b"XYZ" => FitType::XYZ {
                left: f(2),
                top: f(3),
                zoom: f(4),
            },
            b"Fit" => FitType::Fit,
            b"FitH" => FitType::FitH { top: f(2) },
            b"FitV" => FitType::FitV { left: f(2) },
            b"FitR" => FitType::FitR {
                left: f(2),
                bottom: f(3),
                right: f(4),
                top: f(5),
            },
            b"FitB" => FitType::FitB,
            b"FitBH" => FitType::FitBH { top: f(2) },
            b"FitBV" => FitType::FitBV { left: f(2) },
            _ => FitType::Fit,
        };

        Some(Self { page_ref, fit })
    }
}
