//! PDF date parsing and formatting.
//!
//! PDF dates follow the format: D:YYYYMMDDHHmmSSOHH'mm'
//! where O is the timezone offset direction (+, -, or Z).

/// A parsed PDF date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PdfDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Timezone indicator: '+', '-', or 'Z'
    pub ut: char,
    /// Timezone offset hours
    pub ut_hour: u8,
    /// Timezone offset minutes
    pub ut_minutes: u8,
}

impl PdfDate {
    /// Parse a PDF date string.
    ///
    /// Accepts formats like:
    /// - "D:20231015120000+05'30'"
    /// - "D:20231015"
    /// - "20231015120000Z"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix("D:").unwrap_or(s);
        if s.len() < 4 {
            return None;
        }

        let year: u16 = s.get(0..4)?.parse().ok()?;
        let month: u8 = s.get(4..6).and_then(|v| v.parse().ok()).unwrap_or(1);
        let day: u8 = s.get(6..8).and_then(|v| v.parse().ok()).unwrap_or(1);
        let hour: u8 = s.get(8..10).and_then(|v| v.parse().ok()).unwrap_or(0);
        let minute: u8 = s.get(10..12).and_then(|v| v.parse().ok()).unwrap_or(0);
        let second: u8 = s.get(12..14).and_then(|v| v.parse().ok()).unwrap_or(0);

        let rest = s.get(14..).unwrap_or("");
        let (ut, ut_hour, ut_minutes) = if rest.is_empty() {
            ('Z', 0, 0)
        } else {
            let first = rest.chars().next()?;
            match first {
                'Z' => ('Z', 0, 0),
                '+' | '-' => {
                    let tz = &rest[1..];
                    let tz = tz.replace('\'', "");
                    let uh: u8 = tz.get(0..2).and_then(|v| v.parse().ok()).unwrap_or(0);
                    let um: u8 = tz.get(2..4).and_then(|v| v.parse().ok()).unwrap_or(0);
                    (first, uh, um)
                }
                _ => ('Z', 0, 0),
            }
        };

        Some(PdfDate {
            year,
            month,
            day,
            hour,
            minute,
            second,
            ut,
            ut_hour,
            ut_minutes,
        })
    }

    /// Format as a PDF date string.
    pub fn to_pdf_string(&self) -> String {
        if self.ut == 'Z' {
            format!(
                "D:{:04}{:02}{:02}{:02}{:02}{:02}Z",
                self.year, self.month, self.day, self.hour, self.minute, self.second
            )
        } else {
            format!(
                "D:{:04}{:02}{:02}{:02}{:02}{:02}{}{:02}'{:02}'",
                self.year,
                self.month,
                self.day,
                self.hour,
                self.minute,
                self.second,
                self.ut,
                self.ut_hour,
                self.ut_minutes
            )
        }
    }
}

impl std::fmt::Display for PdfDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_pdf_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full() {
        let d = PdfDate::parse("D:20231015120000+05'30'").unwrap();
        assert_eq!(d.year, 2023);
        assert_eq!(d.month, 10);
        assert_eq!(d.day, 15);
        assert_eq!(d.hour, 12);
        assert_eq!(d.minute, 0);
        assert_eq!(d.second, 0);
        assert_eq!(d.ut, '+');
        assert_eq!(d.ut_hour, 5);
        assert_eq!(d.ut_minutes, 30);
    }

    #[test]
    fn test_parse_minimal() {
        let d = PdfDate::parse("D:2023").unwrap();
        assert_eq!(d.year, 2023);
        assert_eq!(d.month, 1);
        assert_eq!(d.day, 1);
    }

    #[test]
    fn test_parse_zulu() {
        let d = PdfDate::parse("D:20231015120000Z").unwrap();
        assert_eq!(d.ut, 'Z');
        assert_eq!(d.ut_hour, 0);
    }

    #[test]
    fn test_roundtrip() {
        let original = "D:20231015120000+05'30'";
        let d = PdfDate::parse(original).unwrap();
        assert_eq!(d.to_pdf_string(), original);
    }

    #[test]
    fn test_roundtrip_zulu() {
        let original = "D:20231015120000Z";
        let d = PdfDate::parse(original).unwrap();
        assert_eq!(d.to_pdf_string(), original);
    }
}
