//! PDF actions — operations triggered by events.

use folio_cos::PdfObject;
use indexmap::IndexMap;

/// PDF action types (ISO 32000-2 Table 198).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    GoTo,
    GoToR,
    GoToE,
    Launch,
    Thread,
    URI,
    Sound,
    Movie,
    Hide,
    Named,
    SubmitForm,
    ResetForm,
    ImportData,
    JavaScript,
    SetOCGState,
    Rendition,
    Trans,
    GoTo3DView,
    RichMediaExecute,
    Unknown,
}

impl ActionType {
    pub fn from_name(name: &[u8]) -> Self {
        match name {
            b"GoTo" => Self::GoTo,
            b"GoToR" => Self::GoToR,
            b"GoToE" => Self::GoToE,
            b"Launch" => Self::Launch,
            b"Thread" => Self::Thread,
            b"URI" => Self::URI,
            b"Sound" => Self::Sound,
            b"Movie" => Self::Movie,
            b"Hide" => Self::Hide,
            b"Named" => Self::Named,
            b"SubmitForm" => Self::SubmitForm,
            b"ResetForm" => Self::ResetForm,
            b"ImportData" => Self::ImportData,
            b"JavaScript" => Self::JavaScript,
            b"SetOCGState" => Self::SetOCGState,
            b"Rendition" => Self::Rendition,
            b"Trans" => Self::Trans,
            b"GoTo3DView" => Self::GoTo3DView,
            b"RichMediaExecute" => Self::RichMediaExecute,
            _ => Self::Unknown,
        }
    }
}

/// A parsed PDF action.
#[derive(Debug, Clone)]
pub struct Action {
    /// The action type.
    pub action_type: ActionType,
    /// The raw action dictionary.
    pub dict: IndexMap<Vec<u8>, PdfObject>,
}

impl Action {
    /// Parse an action from a PDF dictionary.
    pub fn from_dict(dict: &IndexMap<Vec<u8>, PdfObject>) -> Self {
        let action_type = dict
            .get(b"S".as_slice())
            .and_then(|o| o.as_name())
            .map(ActionType::from_name)
            .unwrap_or(ActionType::Unknown);

        Self {
            action_type,
            dict: dict.clone(),
        }
    }

    /// Parse from a PdfObject (must be a dict).
    pub fn from_object(obj: &PdfObject) -> Option<Self> {
        let dict = obj.as_dict()?;
        Some(Self::from_dict(dict))
    }

    /// Get the URI for a URI action.
    pub fn uri(&self) -> Option<String> {
        if self.action_type != ActionType::URI {
            return None;
        }
        self.dict
            .get(b"URI".as_slice())?
            .as_str()
            .map(|s| String::from_utf8_lossy(s).into_owned())
    }

    /// Get the JavaScript source for a JavaScript action.
    pub fn javascript(&self) -> Option<String> {
        if self.action_type != ActionType::JavaScript {
            return None;
        }
        match self.dict.get(b"JS".as_slice())? {
            PdfObject::Str(s) => Some(String::from_utf8_lossy(&s).into_owned()),
            _ => None,
        }
    }

    /// Get the destination for a GoTo action.
    pub fn destination(&self) -> Option<&PdfObject> {
        if self.action_type != ActionType::GoTo {
            return None;
        }
        self.dict.get(b"D".as_slice())
    }

    /// Get the named action name (for Named actions).
    pub fn named_action(&self) -> Option<String> {
        if self.action_type != ActionType::Named {
            return None;
        }
        self.dict
            .get(b"N".as_slice())?
            .as_name()
            .map(|n| String::from_utf8_lossy(n).into_owned())
    }

    /// Get the next action in the chain.
    pub fn next(&self) -> Option<&PdfObject> {
        self.dict.get(b"Next".as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_parsing() {
        assert_eq!(ActionType::from_name(b"GoTo"), ActionType::GoTo);
        assert_eq!(ActionType::from_name(b"URI"), ActionType::URI);
        assert_eq!(ActionType::from_name(b"JavaScript"), ActionType::JavaScript);
        assert_eq!(ActionType::from_name(b"Unknown"), ActionType::Unknown);
    }
}
