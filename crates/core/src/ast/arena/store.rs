use crate::ast::config::FileId;
use std::collections::HashMap;

/// File-owned arena groups. Registration never replaces an existing file.
#[derive(Debug, Default)]
pub struct ArenaStore<T> {
    files: HashMap<FileId, T>,
}

impl<T> ArenaStore<T> {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: FileId, value: T) -> Result<(), FileId> {
        match self.files.entry(id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
            std::collections::hash_map::Entry::Occupied(_) => Err(id),
        }
    }

    pub fn get(&self, id: FileId) -> Option<&T> {
        self.files.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (FileId, &T)> {
        self.files.iter().map(|(id, value)| (*id, value))
    }
    pub fn get_mut(&mut self, id: FileId) -> Option<&mut T> {
        self.files.get_mut(&id)
    }
    pub fn remove(&mut self, id: FileId) -> Option<T> {
        self.files.remove(&id)
    }
}
