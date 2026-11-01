//! The text style manager's working copy of the selected style.
//!
//! The fields show rounded numbers, so writing them back as typed would round
//! every value of a style the user only looked at. The draft keeps the style's
//! exact values and the text each number field showed when it was loaded; only
//! a field whose text the user changed is parsed and written (#2413).

use crate::entities::text_support::{TTF_FLAG_BOLD, TTF_FLAG_ITALIC};
use codec::tables::TextStyle;

/// A number field: the text shown when the style was loaded, and the text in
/// the field now.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct NumberField {
    shown: String,
    pub(super) text: String,
}

impl NumberField {
    fn new(shown: String) -> Self {
        Self {
            text: shown.clone(),
            shown,
        }
    }

    pub(super) fn is_modified(&self) -> bool {
        self.text.trim() != self.shown.trim()
    }

    /// The typed value, when the user changed the field and it parses.
    fn typed(&self) -> Option<f64> {
        self.is_modified()
            .then(|| self.text.trim().parse::<f64>().ok())
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TextStyleDraft {
    /// The selected style with its exact stored values. Toggles and font
    /// fields edit it directly; they hold no rounded text.
    pub(super) style: TextStyle,
    pub(super) width: NumberField,
    /// Shown in degrees, stored in radians.
    pub(super) oblique: NumberField,
    pub(super) height: NumberField,
}

impl Default for TextStyleDraft {
    fn default() -> Self {
        Self::load(&TextStyle::new("Standard"))
    }
}

impl TextStyleDraft {
    pub(super) fn load(style: &TextStyle) -> Self {
        Self {
            style: style.clone(),
            width: NumberField::new(format!("{:.4}", style.width_factor)),
            oblique: NumberField::new(format!("{:.2}", style.oblique_angle.to_degrees())),
            height: NumberField::new(format!("{:.4}", style.height)),
        }
    }

    /// Switch to a stroke (SHX) font. A stroke font has no bold or italic face,
    /// so those TrueType bits go; the character set and pitch bits stay.
    pub(super) fn pick_stroke_font(&mut self, file: String) {
        let style = &mut self.style;
        style.font_file = file;
        style.true_type_font.clear();
        style.true_type_font_flags &= !(TTF_FLAG_BOLD | TTF_FLAG_ITALIC);
    }

    /// Switch to a TrueType family. The font file names the family's regular
    /// face (bold and italic are flags), or stays empty when no installed file
    /// is known. Vertical text and a big font apply to stroke fonts only.
    pub(super) fn pick_system_font(&mut self, family: String) {
        let style = &mut self.style;
        style.font_file =
            crate::scene::text::sysfont::face_file(&family, false, false).unwrap_or_default();
        style.true_type_font = family;
        style.is_vertical = false;
        style.big_font_file.clear();
    }

    /// The style as edited: the changed number fields written in, every other
    /// value exactly as stored.
    pub(super) fn resolved(&self) -> TextStyle {
        let mut style = self.style.clone();
        if let Some(width) = self.width.typed() {
            style.width_factor = width;
        }
        if let Some(oblique) = self.oblique.typed() {
            style.oblique_angle = oblique.to_radians();
        }
        if let Some(height) = self.height.typed() {
            style.height = height.max(0.0);
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> TextStyle {
        let mut style = TextStyle::new("Exact");
        style.width_factor = 0.123456789;
        style.oblique_angle = 0.1;
        style.height = 2.123456789;
        style
    }

    #[test]
    fn untouched_fields_keep_the_exact_values() {
        let draft = TextStyleDraft::load(&style());
        assert_eq!(draft.width.text, "0.1235");
        assert_eq!(draft.resolved(), style());
    }

    #[test]
    fn only_the_changed_field_is_written() {
        let mut draft = TextStyleDraft::load(&style());
        draft.height.text = "3.5".to_string();
        let resolved = draft.resolved();
        assert_eq!(resolved.height, 3.5);
        assert_eq!(resolved.width_factor, 0.123456789);
        assert_eq!(resolved.oblique_angle, 0.1);
    }

    #[test]
    fn typing_the_shown_text_again_is_no_change() {
        let mut draft = TextStyleDraft::load(&style());
        draft.oblique.text = "9".to_string();
        draft.oblique.text = " 5.73 ".to_string();
        assert!(!draft.oblique.is_modified());
        assert_eq!(draft.resolved(), style());
    }
}
