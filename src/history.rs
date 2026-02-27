#[derive(Clone, Debug)]
pub struct History<T: Clone> {
    undo: Vec<T>,
    redo: Vec<T>,
    max_depth: usize,
}

impl<T: Clone> Default for History<T> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            max_depth: 50,
        }
    }
}

impl<T: Clone> History<T> {
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            max_depth: max_depth.max(1),
            ..Self::default()
        }
    }

    pub fn save_state(&mut self, state: &T) {
        self.undo.push(state.clone());
        if self.undo.len() > self.max_depth {
            let overflow = self.undo.len() - self.max_depth;
            self.undo.drain(0..overflow);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: &T) -> Option<T> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        if self.redo.len() > self.max_depth {
            let overflow = self.redo.len() - self.max_depth;
            self.redo.drain(0..overflow);
        }
        Some(previous)
    }

    pub fn redo(&mut self, current: &T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        if self.undo.len() > self.max_depth {
            let overflow = self.undo.len() - self.max_depth;
            self.undo.drain(0..overflow);
        }
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_tracks_undo_and_redo_like_typescript_stacks() {
        let mut history = History::with_max_depth(3);
        let mut current = 1i32;

        history.save_state(&current); // [1]
        current = 2;
        history.save_state(&current); // [1,2]
        current = 3;
        history.save_state(&current); // [1,2,3]
        current = 4;

        let step1 = history.undo(&current).expect("undo 1");
        assert_eq!(step1, 3);
        current = step1;

        let step2 = history.undo(&current).expect("undo 2");
        assert_eq!(step2, 2);
        current = step2;
        assert!(history.can_redo());

        let redo = history.redo(&current).expect("redo");
        assert_eq!(redo, 3);
    }

    #[test]
    fn history_respects_max_depth() {
        let mut history = History::with_max_depth(2);
        history.save_state(&1);
        history.save_state(&2);
        history.save_state(&3);
        assert!(history.can_undo());

        // Oldest state (1) should be trimmed; first undo should yield 3.
        assert_eq!(history.undo(&4), Some(3));
        assert_eq!(history.undo(&3), Some(2));
        assert_eq!(history.undo(&2), None);
    }
}
