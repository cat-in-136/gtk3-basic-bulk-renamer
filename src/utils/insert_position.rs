use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum TextCharPosition {
    Front(usize),
    Back(usize),
}

impl TextCharPosition {
    fn get_position(&self, text: &str) -> usize {
        let mut grapheme_indices =
            UnicodeSegmentation::grapheme_indices(text, true).map(|(pos, _)| pos);
        match self {
            TextCharPosition::Front(pos) => grapheme_indices.nth(*pos).unwrap_or(text.len()),
            TextCharPosition::Back(pos) => {
                if *pos == 0 {
                    text.len()
                } else {
                    grapheme_indices.nth_back(*pos - 1).unwrap_or(0)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum TextInsertOrOverwrite {
    Insert = 0,
    Overwrite,
}

impl Default for TextInsertOrOverwrite {
    fn default() -> Self {
        Self::Insert
    }
}

pub(crate) trait BulkTextReplacement {
    fn apply_to(self, text: &str, replacement: &str) -> String;
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct InsertPosition(pub TextCharPosition, pub TextInsertOrOverwrite);

impl BulkTextReplacement for InsertPosition {
    fn apply_to(self, text: &str, replacement: &str) -> String {
        let idx = self.0.get_position(text);

        let mut new_text = text.to_string();
        match self.1 {
            TextInsertOrOverwrite::Insert => {
                new_text.insert_str(idx, &replacement);
            }
            TextInsertOrOverwrite::Overwrite => {
                let range = idx..(idx + replacement.len()).min(text.len());
                new_text.replace_range(range, &replacement);
            }
        }
        new_text
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct RemoveRangePosition(pub TextCharPosition, pub TextCharPosition);

impl BulkTextReplacement for RemoveRangePosition {
    fn apply_to(self, text: &str, replacement: &str) -> String {
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
    fn test_remove_character_position() {
        use TextCharPosition::*;

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

        // "😀🧝‍♀️🧝‍♂️": "😀" "🧝‍♀" "️🧝‍♂️"

        assert_eq!(Front(0).get_position("😀🧝‍♀️🧝‍♂️"), 0);
        assert_eq!(Front(1).get_position("😀🧝‍♀️🧝‍♂️"), 4);
        assert_eq!(Front(2).get_position("😀🧝‍♀️🧝‍♂️"), 17);
        assert_eq!(Front(3).get_position("😀🧝‍♀️🧝‍♂️"), 30);
        assert_eq!(Front(4).get_position("😀🧝‍♀️🧝‍♂️"), 30);

        assert_eq!(Back(0).get_position("😀🧝‍♀️🧝‍♂️"), 30);
        assert_eq!(Back(1).get_position("😀🧝‍♀️🧝‍♂️"), 17);
        assert_eq!(Back(2).get_position("😀🧝‍♀️🧝‍♂️"), 4);
        assert_eq!(Back(3).get_position("😀🧝‍♀️🧝‍♂️"), 0);
        assert_eq!(Back(4).get_position("😀🧝‍♀️🧝‍♂️"), 0);
    }

    #[test]
    fn test_insert_position() {
        use TextCharPosition::*;
        use TextInsertOrOverwrite::*;

        assert_eq!(
            InsertPosition(Front(0), Insert).apply_to("text", "INS"),
            "INStext"
        );
        assert_eq!(
            InsertPosition(Front(1), Insert).apply_to("text", "INS"),
            "tINSext"
        );
        assert_eq!(
            InsertPosition(Front(3), Insert).apply_to("text", "INS"),
            "texINSt"
        );
        assert_eq!(
            InsertPosition(Front(4), Insert).apply_to("text", "INS"),
            "textINS"
        );
        assert_eq!(
            InsertPosition(Front(5), Insert).apply_to("text", "INS"),
            "textINS"
        );

        assert_eq!(
            InsertPosition(Back(0), Insert).apply_to("text", "INS"),
            "textINS"
        );
        assert_eq!(
            InsertPosition(Back(1), Insert).apply_to("text", "INS"),
            "texINSt"
        );
        assert_eq!(
            InsertPosition(Back(3), Insert).apply_to("text", "INS"),
            "tINSext"
        );
        assert_eq!(
            InsertPosition(Back(4), Insert).apply_to("text", "INS"),
            "INStext"
        );
        assert_eq!(
            InsertPosition(Back(5), Insert).apply_to("text", "INS"),
            "INStext"
        );

        assert_eq!(
            InsertPosition(Front(0), Overwrite).apply_to("text", "OW"),
            "OWxt"
        );
        assert_eq!(
            InsertPosition(Front(1), Overwrite).apply_to("text", "OW"),
            "tOWt"
        );
        assert_eq!(
            InsertPosition(Front(2), Overwrite).apply_to("text", "OW"),
            "teOW"
        );
        assert_eq!(
            InsertPosition(Front(3), Overwrite).apply_to("text", "OW"),
            "texOW"
        );
        assert_eq!(
            InsertPosition(Front(4), Overwrite).apply_to("text", "OW"),
            "textOW"
        );
        assert_eq!(
            InsertPosition(Front(5), Overwrite).apply_to("text", "OW"),
            "textOW"
        );

        assert_eq!(
            InsertPosition(Back(0), Overwrite).apply_to("text", "OW"),
            "textOW"
        );
        assert_eq!(
            InsertPosition(Back(1), Overwrite).apply_to("text", "OW"),
            "texOW"
        );
        assert_eq!(
            InsertPosition(Back(2), Overwrite).apply_to("text", "OW"),
            "teOW"
        );
        assert_eq!(
            InsertPosition(Back(3), Overwrite).apply_to("text", "OW"),
            "tOWt"
        );
        assert_eq!(
            InsertPosition(Back(4), Overwrite).apply_to("text", "OW"),
            "OWxt"
        );
        assert_eq!(
            InsertPosition(Back(5), Overwrite).apply_to("text", "OW"),
            "OWxt"
        );
    }

    #[test]
    fn test_remove_range_position() {
        use TextCharPosition::*;

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
