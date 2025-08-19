#![allow(unused)]
use eframe::egui;

/// textedit for hex strings
///
/// potential issues
/// 1. when deleting to the right, eg [xx| x] using DEL key,
/// nothing happens (same behavior in DSA)
///
/// 2. when deleting to the left eg [xx |xx] using delete key,
/// nothing happens but only cursor moves to [xx| xx], this
/// maybe is not an issue
pub struct HexEdit {
    text: String,
}

impl HexEdit {
    pub fn new(s: &str) -> Self {
        let (text, _) = HexEdit::format_hex_and_calculate_cursor(s, s.len());
        Self { text }
    }

    /// format the input text and re-locate cursor,
    /// this function is extracted out for unittesting purposes
    fn format_hex_and_calculate_cursor(
        current_text: &str,
        original_cursor_pos: usize,
    ) -> (String, usize) {
        let pure_hex: String = current_text
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();

        let msg_repadded = pure_hex
            .as_bytes()
            .chunks(2)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<&str>>()
            .join(" ");

        if current_text == msg_repadded {
            return (msg_repadded, original_cursor_pos);
        }

        // Simplified cursor logic for example, actual logic is more complex
        let mut num_chars_before_cursor = 0;
        for (i, c) in current_text.chars().enumerate() {
            if i >= original_cursor_pos {
                break;
            }
            if c.is_ascii_hexdigit() {
                num_chars_before_cursor += 1;
            }
        }

        let mut new_cursor_pos = 0;
        let mut hex_chars_counted = 0;
        if num_chars_before_cursor == 0 {
            new_cursor_pos = 0;
        } else {
            for (i, ch) in msg_repadded.chars().enumerate() {
                if ch.is_ascii_hexdigit() {
                    hex_chars_counted += 1;
                }
                if hex_chars_counted == num_chars_before_cursor {
                    new_cursor_pos = i + 1;
                    break;
                }
            }
            if hex_chars_counted < num_chars_before_cursor {
                // Cursor was at the end of hex chars
                new_cursor_pos = msg_repadded.len();
            }
        }
        (msg_repadded, new_cursor_pos)
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.text)
                .hint_text("auto spaced hex string")
                .desired_width(ui.available_width())
                .font(egui::TextStyle::Monospace)
                .text_color(if ui.visuals().dark_mode {
                    egui::Color32::from_rgb(229, 192, 123) // one Dark Pro yellow
                // egui::Color32::YELLOW   // just not right
                } else {
                    egui::Color32::from_rgb(65, 105, 225) // blue
                    // egui::Color32::BLUE  // just not right
                }),
        );

        // reserve, in case if needed in future
        // response = response.on_hover_text("space are auto padded");

        if response.changed() {
            let original_text_clone = self.text.clone();
            let text_edit_state =
                egui::widgets::text_edit::TextEditState::load(ui.ctx(), response.id);
            let original_cursor_pos = text_edit_state
                .as_ref()
                .and_then(|s| s.cursor.char_range().map(|r| r.primary.index))
                .unwrap_or(self.text.len());

            let (new_text, new_cursor_idx) =
                Self::format_hex_and_calculate_cursor(&original_text_clone, original_cursor_pos);

            /*  Only update self.message and cursor position if re-padded
                message is different from the originla message

                notice we are assuming all chars are 1 byte to avoid
                further complexity (mixing of char index and byte index)

                the complexity lies for example, if inserting in the middle
                    [oo| oo]        given cursor before the space
                    [oox| oo]       after insert, ocp = 3, noc = 3
                    [oo xo o]       re-padded
                    [oo x|o o]      ncp should = 4

                    [oo |oo]        given cursor after the space
                    [oo x|oo]       after insert, ocp = 4, noc = 3
                    [oo xo o]       re-padded
                    [oo x|o o]      ncp should still = 4, that is to say,
                                    no difference inserting before or after space

                how do we achieve this
                    in essence we need to make sure noc stays the same
                    find the same amount of noc, the i+1 will be the new cursor position
                    notice i starts from 0, that is, when we counted enough noc
                    i will stay 1 pos ahead of the ncp
            */
            if self.text != new_text {
                self.text = new_text;
                if let Some(mut state) = text_edit_state {
                    let new_ccursor = egui::text::CCursor::new(new_cursor_idx);
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(new_ccursor)));
                    state.store(ui.ctx(), response.id);
                }
            }
        }
    }

    pub fn get_text_raw(&self) -> String {
        self.text.clone()
    }

    // remove all the padded spaces
    pub fn get_text(&self) -> String {
        self.text
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect()
    }

    pub fn set_text(&mut self, s: &str) {
        let (formatted_text, _) = Self::format_hex_and_calculate_cursor(s, s.len());
        self.text = formatted_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_formatting_simple_insert() {
        let current_text = "123";
        let cursor_pos = 3; // After '3'
        let (formatted_text, new_cursor) =
            HexEdit::format_hex_and_calculate_cursor(current_text, cursor_pos);
        assert_eq!(formatted_text, "12 3");
        assert_eq!(new_cursor, 4); // After "12 3|"
    }

    #[test]
    fn test_hex_formatting_insert_with_space() {
        let current_text = "12 345"; // User types '5'
        let cursor_pos = 6; // After '5'
        let (formatted_text, new_cursor) =
            HexEdit::format_hex_and_calculate_cursor(current_text, cursor_pos);
        assert_eq!(formatted_text, "12 34 5");
        assert_eq!(new_cursor, 7); // "12 34 5|"
    }

    #[test]
    fn test_hex_formatting_filter_non_hex() {
        let current_text = "12axby34";
        let cursor_pos = 8;
        let (formatted_text, new_cursor) =
            HexEdit::format_hex_and_calculate_cursor(current_text, cursor_pos);
        assert_eq!(formatted_text, "12 ab 34");
        assert_eq!(new_cursor, 8);
    }

    #[test]
    fn test_cursor_at_beginning_after_delete_all() {
        let current_text = ""; // User deleted everything
        let cursor_pos = 0;
        let (formatted_text, new_cursor) =
            HexEdit::format_hex_and_calculate_cursor(current_text, cursor_pos);
        assert_eq!(formatted_text, "");
        assert_eq!(new_cursor, 0);
    }

    #[test]
    fn test_inserting_in_middle() {
        // Initial: "12 34", cursor: "12 |34" (index 3)
        // User types 'AB': "12 AB34"
        let current_text = "12 AB34";
        let cursor_pos = 5; // "12 AB|34"
        let (formatted_text, new_cursor) =
            HexEdit::format_hex_and_calculate_cursor(current_text, cursor_pos);
        assert_eq!(formatted_text, "12 AB 34");
        assert_eq!(new_cursor, 5); // "12 AB| 34"
    }
}
