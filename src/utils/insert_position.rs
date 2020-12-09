#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum InsertPosition {
    Front(usize),
    Back(usize),
    FrontOverwrite(usize),
    BackOverwrite(usize),
}

impl InsertPosition {
    pub fn apply_to(self, text: &str, replacement: &str) -> String {
        let idx = match self {
            InsertPosition::Front(pos) | InsertPosition::FrontOverwrite(pos) => pos,
            InsertPosition::Back(pos) | InsertPosition::BackOverwrite(pos) => {
                text.len().checked_sub(pos).unwrap_or(0)
            }
        }
        .min(text.len());

        let mut new_text = text.to_string();
        match self {
            InsertPosition::Front(_) | InsertPosition::Back(_) => {
                new_text.insert_str(idx, &replacement);
            }
            InsertPosition::FrontOverwrite(_) | InsertPosition::BackOverwrite(_) => {
                let range = idx..(idx + replacement.len()).min(text.len());
                new_text.replace_range(range, &replacement);
            }
        }
        new_text
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum RemoveCharacterPosition {
    Front(usize),
    Back(usize),
}

impl RemoveCharacterPosition {
    fn get_position(&self, text: &str) -> usize {
        match self {
            RemoveCharacterPosition::Front(pos) => *pos,
            RemoveCharacterPosition::Back(pos) => text.len().checked_sub(*pos).unwrap_or(0),
        }
        .min(text.len())
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct RemoveRangePosition(pub RemoveCharacterPosition, pub RemoveCharacterPosition);

impl RemoveRangePosition {
    pub fn apply_to(self, text: &str, replacement: &str) -> String {
        let pos_from = self.0.get_position(text).min(text.len());
        let pos_to = self.1.get_position(text).min(text.len());

        let mut new_text = text.to_string();
        if pos_from <= pos_to {
            new_text.replace_range(pos_from..pos_to, replacement);
        }
        new_text
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insert_position() {
        use InsertPosition::*;

        assert_eq!(Front(0).apply_to("text", "INS"), "INStext");
        assert_eq!(Front(1).apply_to("text", "INS"), "tINSext");
        assert_eq!(Front(3).apply_to("text", "INS"), "texINSt");
        assert_eq!(Front(4).apply_to("text", "INS"), "textINS");
        assert_eq!(Front(5).apply_to("text", "INS"), "textINS");

        assert_eq!(Back(0).apply_to("text", "INS"), "textINS");
        assert_eq!(Back(1).apply_to("text", "INS"), "texINSt");
        assert_eq!(Back(3).apply_to("text", "INS"), "tINSext");
        assert_eq!(Back(4).apply_to("text", "INS"), "INStext");
        assert_eq!(Back(5).apply_to("text", "INS"), "INStext");

        assert_eq!(FrontOverwrite(0).apply_to("text", "OW"), "OWxt");
        assert_eq!(FrontOverwrite(1).apply_to("text", "OW"), "tOWt");
        assert_eq!(FrontOverwrite(2).apply_to("text", "OW"), "teOW");
        assert_eq!(FrontOverwrite(3).apply_to("text", "OW"), "texOW");
        assert_eq!(FrontOverwrite(4).apply_to("text", "OW"), "textOW");
        assert_eq!(FrontOverwrite(5).apply_to("text", "OW"), "textOW");

        assert_eq!(BackOverwrite(0).apply_to("text", "OW"), "textOW");
        assert_eq!(BackOverwrite(1).apply_to("text", "OW"), "texOW");
        assert_eq!(BackOverwrite(2).apply_to("text", "OW"), "teOW");
        assert_eq!(BackOverwrite(3).apply_to("text", "OW"), "tOWt");
        assert_eq!(BackOverwrite(4).apply_to("text", "OW"), "OWxt");
        assert_eq!(BackOverwrite(5).apply_to("text", "OW"), "OWxt");
    }

    #[test]
    fn test_remove_character_position() {
        use RemoveCharacterPosition::*;

        assert_eq!(Front(0).get_position("text"), 0);
        assert_eq!(Front(1).get_position("text"), 1);
        assert_eq!(Front(2).get_position("text"), 2);
        assert_eq!(Front(3).get_position("text"), 3);
        assert_eq!(Front(4).get_position("text"), 4);
        assert_eq!(Front(5).get_position("text"), 4);

        assert_eq!(Back(0).get_position("text"), 4);
        assert_eq!(Back(1).get_position("text"), 3);
        assert_eq!(Back(2).get_position("text"), 2);
        assert_eq!(Back(3).get_position("text"), 1);
        assert_eq!(Back(4).get_position("text"), 0);
        assert_eq!(Back(5).get_position("text"), 0);
    }

    #[test]
    fn test_remove_range_position() {
        use RemoveCharacterPosition::*;

        assert_eq!(
            RemoveRangePosition(Front(0), Front(0)).apply_to("text", "INS"),
            "INStext"
        );
        assert_eq!(
            RemoveRangePosition(Back(0), Back(0)).apply_to("text", "INS"),
            "textINS"
        );
        assert_eq!(
            RemoveRangePosition(Front(1), Back(1)).apply_to("text", "INS"),
            "tINSt"
        );
        assert_eq!(
            RemoveRangePosition(Back(3), Front(3)).apply_to("text", "INS"),
            "tINSt"
        );

        assert_eq!(
            RemoveRangePosition(Front(1), Front(0)).apply_to("text", "INS"),
            "text"
        );
    }
}
