use serde::Serialize;
use chrono::prelude::*;

use crate::model::Note;

#[derive(Serialize, Debug)]
pub struct FilteredNote {
    pub content: String,
    pub tags: Vec<String>,
    pub date: DateTime<Utc>
}

impl From<&Note> for FilteredNote {
    fn from(note: &Note) -> Self {
        FilteredNote {
            content: note.content.to_owned(),
            date: note.date.unwrap(),
            tags: note.tags.to_owned()
        }
    }
}

#[derive(Serialize, Debug)]
pub struct NotesResponse {
    pub author: String,
    pub notes: Vec<FilteredNote>
}
