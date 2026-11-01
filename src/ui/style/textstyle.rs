//! Text style manager.

use crate::app::{Message, StyleKind};
use crate::t;
use crate::ui::font_picker::{FaceStyle, FontChoice, FontPickerState};
use crate::ui::style::common::muted_style;
use crate::ui::style::form::field_style;
use iced::widget::{canvas, checkbox, column, row, text, text_input, Space};
use iced::{mouse, Element, Length, Point, Rectangle, Theme};
use std::borrow::Cow;

pub struct TextStyleView<'a> {
    pub styles: Vec<String>,
    pub selected: &'a str,
    pub current: &'a str,
    pub tab: u8,
    pub compare_name: String,
    pub compare_opts: Vec<String>,
    pub comparison_sections: Vec<String>,
    pub read_only: bool,
    pub in_use: bool,
    pub font_buf: &'a str,
    pub width_buf: &'a str,
    pub oblique_buf: &'a str,
    pub height_buf: &'a str,
    pub bigfont_buf: &'a str,
    pub ttf_buf: &'a str,
    /// Why each number field's text cannot be applied, if it cannot.
    pub width_error: Option<Cow<'static, str>>,
    pub oblique_error: Option<Cow<'static, str>>,
    pub height_error: Option<Cow<'static, str>>,
    pub invalid_fields: usize,
    pub use_big_font: bool,
    pub font_picker: &'a FontPickerState,
    /// The TrueType font style (typeface flags).
    pub bold: bool,
    pub italic: bool,
    pub backward: bool,
    pub upside_down: bool,
    pub vertical: bool,
    pub annotative: bool,
    pub rename_active: Option<&'a str>,
    pub rename_buf: &'a str,
}

fn primary_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.palette().primary.base.color),
    }
}

struct TextPreviewCanvas {
    font: String,
    /// `font` in the style's bold / italic face, when it has one.
    face: String,
    big_font: String,
    width_factor: f32,
    oblique: f32,
    height: f32,
    upside_down: bool,
    vertical: bool,
    annotative: bool,
}

impl canvas::Program<Message> for TextPreviewCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let ink = theme.palette().background.base.text;
        let glyph_height = if self.height <= 0.0 {
            18.0
        } else {
            (self.height * 7.0).clamp(6.0, 52.0)
        };
        let rotation = if self.upside_down {
            std::f32::consts::PI
        } else {
            0.0
        };
        let strokes = if self.vertical {
            let mut result = Vec::new();
            for (index, character) in "Ab3".chars().enumerate() {
                let (mut glyph, _) = crate::scene::text::lff::tessellate_text_ex(
                    [0.0, -(index as f32) * glyph_height * 1.25],
                    glyph_height,
                    rotation,
                    self.width_factor,
                    self.oblique,
                    &self.face,
                    &character.to_string(),
                );
                result.append(&mut glyph);
            }
            result
        } else {
            crate::scene::text::lff::tessellate_text_ex(
                [0.0, 0.0],
                glyph_height,
                rotation,
                self.width_factor,
                self.oblique,
                &self.face,
                "AaBbCc 0123",
            )
            .0
        };
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for stroke in &strokes {
            for &[x, y] in stroke {
                min[0] = min[0].min(x);
                min[1] = min[1].min(y);
                max[0] = max[0].max(x);
                max[1] = max[1].max(y);
            }
        }
        if min[0] <= max[0] {
            let span = [(max[0] - min[0]).max(1.0), (max[1] - min[1]).max(1.0)];
            let available = [bounds.width - 24.0, bounds.height - 34.0];
            let scale = (available[0] / span[0])
                .min(available[1] / span[1])
                .min(1.0)
                .max(0.0);
            let middle = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
            let center = [bounds.width * 0.5, (bounds.height - 18.0) * 0.5];
            let map = |x: f32, y: f32| {
                Point::new(
                    center[0] + (x - middle[0]) * scale,
                    center[1] - (y - middle[1]) * scale,
                )
            };
            for stroke in &strokes {
                if stroke.len() < 2 {
                    continue;
                }
                let path = canvas::Path::new(|builder| {
                    builder.move_to(map(stroke[0][0], stroke[0][1]));
                    for &[x, y] in &stroke[1..] {
                        builder.line_to(map(x, y));
                    }
                });
                frame.stroke(
                    &path,
                    canvas::Stroke::default().with_color(ink).with_width(1.35),
                );
            }
        }
        let font_label = if self.big_font.trim().is_empty() {
            self.font.clone()
        } else {
            format!("{} + {}", self.font, self.big_font)
        };
        let mode = if self.annotative {
            t!("Paper height")
        } else {
            t!("Drawing height")
        };
        frame.fill_text(canvas::Text {
            content: format!("{}  ·  {} {:.3}", font_label, mode, self.height),
            position: Point::new(10.0, bounds.height - 9.0),
            color: ink.scale_alpha(0.65),
            size: iced::Pixels(10.0),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
        vec![frame.into_geometry()]
    }
}

fn input_row<'a>(
    label: Cow<'static, str>,
    placeholder: Cow<'static, str>,
    value: &'a str,
    field: &'static str,
    read_only: bool,
) -> Element<'a, Message> {
    number_row(label, placeholder, value, field, read_only, None)
}

/// An input row; with an `error`, the field turns red and the reason shows
/// under it.
fn number_row<'a>(
    label: Cow<'static, str>,
    placeholder: Cow<'static, str>,
    value: &'a str,
    field: &'static str,
    read_only: bool,
    error: Option<Cow<'static, str>>,
) -> Element<'a, Message> {
    use crate::ui::style::editor_table::{danger_input_style, danger_text_style};
    let control: Element<'a, Message> = if read_only {
        crate::ui::read_only::field(value, 11.0, Length::Fixed(180.0))
    } else {
        text_input(placeholder.as_ref(), value)
            .on_input(move |value| Message::TextStyleEdit { field, value })
            .style(if error.is_some() {
                danger_input_style
            } else {
                field_style
            })
            .size(11)
            .width(180)
            .into()
    };
    let line = labelled_row(label, control);
    match error {
        Some(error) => column![
            line,
            row![
                Space::new().width(158),
                text(error)
                    .size(10)
                    .style(danger_text_style)
                    .width(Length::Fill)
            ]
        ]
        .spacing(3)
        .into(),
        None => line,
    }
}

/// A caption and its control on one line, the controls lined up.
fn labelled_row<'a>(
    label: Cow<'static, str>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        text(label).size(11).style(muted_style).width(150),
        control.into()
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into()
}

fn toggle<'a>(
    label: Cow<'static, str>,
    value: bool,
    field: &'static str,
    read_only: bool,
) -> Element<'a, Message> {
    let checkbox = checkbox(value).label(label).size(14).text_size(11);
    if read_only {
        checkbox.into()
    } else {
        checkbox
            .on_toggle(move |_| Message::TextStyleToggle(field))
            .into()
    }
}

pub fn view_window<'a>(
    v: TextStyleView<'a>,
    sizing: crate::ui::modal::ModalSizing,
) -> Element<'a, Message> {
    let effective_font = if v.ttf_buf.trim().is_empty() {
        v.font_buf
    } else {
        v.ttf_buf
    };
    let width = v
        .width_buf
        .trim()
        .parse::<f32>()
        .unwrap_or(1.0)
        .abs()
        .clamp(0.01, 100.0);
    let ttf = !v.ttf_buf.trim().is_empty();
    let preview = canvas(TextPreviewCanvas {
        font: effective_font.to_string(),
        face: crate::entities::text_support::styled_font(
            effective_font,
            ttf && v.bold,
            ttf && v.italic,
        )
        .unwrap_or_else(|| effective_font.to_string()),
        big_font: v.bigfont_buf.to_string(),
        width_factor: if v.backward { -width } else { width },
        oblique: v
            .oblique_buf
            .trim()
            .parse::<f32>()
            .unwrap_or(0.0)
            .to_radians(),
        height: v.height_buf.trim().parse::<f32>().unwrap_or(0.0).max(0.0),
        upside_down: v.upside_down,
        vertical: v.vertical,
        annotative: v.annotative,
    })
    .width(Length::Fill)
    .height(112);

    let content: Element<'a, Message> = match v.tab {
        0 => {
            let face = FaceStyle::of(v.bold, v.italic);
            let face_style: Element<'a, Message> = if ttf && !v.read_only {
                iced::widget::pick_list(
                    Some(face),
                    FaceStyle::offered(v.ttf_buf, face),
                    |face: &FaceStyle| face.to_string(),
                )
                .on_select(Message::TextStyleFaceStyle)
                .text_size(11)
                .width(180)
                .into()
            } else {
                // A stroke font has no bold or italic face.
                crate::ui::read_only::field(
                    if ttf {
                        face.to_string()
                    } else {
                        t!("TrueType fonts only").into_owned()
                    }
                    .as_str(),
                    11.0,
                    Length::Fixed(180.0),
                )
            };
            column![
                labelled_row(
                    t!("Font"),
                    crate::ui::font_picker::view(crate::ui::font_picker::FontPicker {
                        state: v.font_picker,
                        selected: if ttf {
                            FontChoice::TrueType(v.ttf_buf.to_string())
                        } else {
                            FontChoice::Stroke(v.font_buf.to_string())
                        },
                        file: v.font_buf.to_string(),
                        width: Length::Fixed(300.0),
                        read_only: v.read_only,
                        on_event: Message::TextStyleFont,
                    }),
                ),
                labelled_row(t!("Font style"), face_style),
                toggle(
                    t!("Use Big Font"),
                    v.use_big_font,
                    "use_big_font",
                    v.read_only || ttf,
                ),
                input_row(
                    t!("Big font"),
                    t!("big-font file…"),
                    v.bigfont_buf,
                    "bigfont",
                    v.read_only || !v.use_big_font,
                ),
            ]
            .spacing(8)
            .into()
        }
        _ => column![
            text(t!("Size")).size(11).style(primary_style),
            number_row(
                if v.annotative {
                    t!("Paper text height")
                } else {
                    t!("Fixed height")
                },
                t!("0 = variable"),
                v.height_buf,
                "height",
                v.read_only,
                v.height_error,
            ),
            text(t!("Effects")).size(11).style(primary_style),
            number_row(
                t!("Width factor"),
                "1.0".into(),
                v.width_buf,
                "width",
                v.read_only,
                v.width_error,
            ),
            number_row(
                t!("Oblique angle"),
                "0.0".into(),
                v.oblique_buf,
                "oblique",
                v.read_only,
                v.oblique_error,
            ),
            row![
                toggle(t!("Backward"), v.backward, "backward", v.read_only),
                toggle(t!("Upside down"), v.upside_down, "upside_down", v.read_only),
                toggle(
                    t!("Vertical"),
                    v.vertical,
                    "vertical",
                    v.read_only || !v.ttf_buf.trim().is_empty(),
                ),
            ]
            .spacing(18),
            toggle(t!("Annotative"), v.annotative, "annotative", v.read_only),
        ]
        .spacing(9)
        .into(),
    };
    let status = if v.read_only {
        t!("Referenced style · Read only")
    } else if v.selected.eq_ignore_ascii_case(v.current) {
        t!("Current style")
    } else if v.in_use {
        t!("Used in drawing")
    } else {
        t!("Available style")
    };
    let summary = if v.comparison_sections.is_empty() {
        t!("No differences").into_owned()
    } else {
        format!("{}: {}", t!("Different"), v.comparison_sections.join(", "))
    };
    let editor = crate::ui::style::style_manager::editor_shell(
        crate::ui::style::style_manager::EditorShell {
            sizing,
            selected: v.selected.to_string(),
            status,
            preview: preview.into(),
            comparison: Some(crate::ui::style::style_manager::EditorComparison {
                selected: v.compare_name,
                options: v.compare_opts,
                summary,
                on_select: Message::TextStyleDialogCompare,
            }),
            tabs: vec![
                crate::ui::style::style_manager::EditorTab {
                    label: t!("Fonts"),
                    active: v.tab == 0,
                    on_press: Message::TextStyleDialogTab(0),
                },
                crate::ui::style::style_manager::EditorTab {
                    label: t!("Size and Effects"),
                    active: v.tab == 1,
                    on_press: Message::TextStyleDialogTab(1),
                },
            ],
            content,
        },
    );
    crate::ui::style::style_manager::view(crate::ui::style::style_manager::Scaffold {
        sizing,
        kind: StyleKind::Text,
        styles: &v.styles,
        selected: v.selected,
        current: Some(v.current),
        rename_active: v.rename_active,
        rename_buf: v.rename_buf,
        on_new: Message::TextStyleDialogNew,
        on_copy: Message::TextStyleDialogCopy,
        on_delete: Message::TextStyleDialogDelete,
        on_import: Some(Message::StyleImportPick(StyleKind::Text)),
        on_select: Message::TextStyleDialogSelect,
        on_set_current: Message::TextStyleDialogSetCurrent,
        on_apply: Message::TextStyleApply,
        read_only: v.read_only,
        in_use_filter: None,
        apply_blocked: match v.invalid_fields {
            0 => None,
            1 => Some(t!("1 field needs fixing before Apply.")),
            count => Some(crate::tf!("{count} fields need fixing before Apply.")),
        },
        editor,
    })
}
