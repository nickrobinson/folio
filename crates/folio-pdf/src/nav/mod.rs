//! PDF navigation: bookmarks (outlines), destinations, and actions.

mod action;
mod bookmark;
mod destination;

pub use action::{Action, ActionType};
pub use bookmark::Bookmark;
pub use destination::{Destination, FitType};
