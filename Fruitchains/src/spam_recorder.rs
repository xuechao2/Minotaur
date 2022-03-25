use std::collections::HashSet;

use crate::transaction::{SpamId, SignedTransaction};


pub struct SpamRecorder {
    set: HashSet<SpamId>
}

impl SpamRecorder {
    pub fn new() -> Self {
        Self {
            set: HashSet::new()
        }
    }

    /// return false if the element is already in
    pub fn test(&self, t: &SignedTransaction) -> bool {
        !self.set.contains(&(t.into()))
    }
    /// return false if the element is already in
    pub fn test_and_set(&mut self, t: &SignedTransaction) -> bool {
        self.set.insert(t.into())
    }
}