use std::{
    error::Error,
    fmt::{self, Display},
    ops::Range,
};

use editor::{DisplayPoint, display_map::DisplaySnapshot, movement};
use text::{Bias, Selection};

use crate::{
    helix::boundary::{FuzzyBoundary, ImmediateBoundary},
    object::Object as VimObject,
};

/// A text object from helix or an extra one
pub trait HelixTextObject {
    fn range(
        &self,
        map: &DisplaySnapshot,
        relative_to: Range<DisplayPoint>,
        around: bool,
    ) -> Option<Range<DisplayPoint>>;

    fn next_range(
        &self,
        map: &DisplaySnapshot,
        relative_to: Range<DisplayPoint>,
        around: bool,
    ) -> Option<Range<DisplayPoint>>;

    fn previous_range(
        &self,
        map: &DisplaySnapshot,
        relative_to: Range<DisplayPoint>,
        around: bool,
    ) -> Option<Range<DisplayPoint>>;
}

impl VimObject {
    /// Returns the range of the object the cursor is over.
    /// Follows helix convention.
    pub fn helix_range(
        self,
        map: &DisplaySnapshot,
        selection: Selection<DisplayPoint>,
        around: bool,
    ) -> Result<Option<Range<DisplayPoint>>, VimToHelixError> {
        let cursor = cursor_range(&selection, map);
        if let Some(helix_object) = self.to_helix_object() {
            Ok(helix_object.range(map, cursor, around))
        } else {
            Err(VimToHelixError)
        }
    }
    /// Returns the range of the next object the cursor is not over.
    /// Follows helix convention.
    pub fn helix_next_range(
        self,
        map: &DisplaySnapshot,
        selection: Selection<DisplayPoint>,
        around: bool,
    ) -> Result<Option<Range<DisplayPoint>>, VimToHelixError> {
        let cursor = cursor_range(&selection, map);
        if let Some(helix_object) = self.to_helix_object() {
            Ok(helix_object.next_range(map, cursor, around))
        } else {
            Err(VimToHelixError)
        }
    }
    /// Returns the range of the previous object the cursor is not over.
    /// Follows helix convention.
    pub fn helix_previous_range(
        self,
        map: &DisplaySnapshot,
        selection: Selection<DisplayPoint>,
        around: bool,
    ) -> Result<Option<Range<DisplayPoint>>, VimToHelixError> {
        let cursor = cursor_range(&selection, map);
        if let Some(helix_object) = self.to_helix_object() {
            Ok(helix_object.previous_range(map, cursor, around))
        } else {
            Err(VimToHelixError)
        }
    }
}

#[derive(Debug)]
pub struct VimToHelixError;
impl Display for VimToHelixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Not all vim text objects have an implemented helix equivalent"
        )
    }
}
impl Error for VimToHelixError {}

impl VimObject {
    fn to_helix_object(self) -> Option<Box<dyn HelixTextObject>> {
        Some(match self {
            Self::AngleBrackets => Box::new(ImmediateBoundary::AngleBrackets),
            Self::AnyBrackets => Box::new(HelixAnyBrackets),
            Self::BackQuotes => Box::new(ImmediateBoundary::BackQuotes),
            Self::CurlyBrackets => Box::new(ImmediateBoundary::CurlyBrackets),
            Self::DoubleQuotes => Box::new(ImmediateBoundary::DoubleQuotes),
            Self::Paragraph => Box::new(FuzzyBoundary::Paragraph),
            Self::Parentheses => Box::new(ImmediateBoundary::Parentheses),
            Self::Quotes => Box::new(ImmediateBoundary::SingleQuotes),
            Self::Sentence => Box::new(FuzzyBoundary::Sentence),
            Self::SquareBrackets => Box::new(ImmediateBoundary::SquareBrackets),
            Self::Subword { ignore_punctuation } => {
                Box::new(ImmediateBoundary::Subword { ignore_punctuation })
            }
            Self::VerticalBars => Box::new(ImmediateBoundary::VerticalBars),
            Self::Word { ignore_punctuation } => {
                Box::new(ImmediateBoundary::Word { ignore_punctuation })
            }
            _ => return None,
        })
    }
}

struct HelixAnyBrackets;

impl HelixTextObject for HelixAnyBrackets {
    fn range(
        &self,
        map: &DisplaySnapshot,
        relative_to: Range<DisplayPoint>,
        around: bool,
    ) -> Option<Range<DisplayPoint>> {
        let bracket_types: &[ImmediateBoundary] = &[
            ImmediateBoundary::Parentheses,
            ImmediateBoundary::SquareBrackets,
            ImmediateBoundary::CurlyBrackets,
            ImmediateBoundary::AngleBrackets,
            ImmediateBoundary::DoubleQuotes,
            ImmediateBoundary::SingleQuotes,
            ImmediateBoundary::BackQuotes,
        ];
        bracket_types
            .iter()
            .filter_map(|bracket| bracket.range(map, relative_to.clone(), around))
            .min_by_key(|range| {
                range.end.to_offset(map, Bias::Left) - range.start.to_offset(map, Bias::Left)
            })
    }

    fn next_range(
        &self,
        _map: &DisplaySnapshot,
        _relative_to: Range<DisplayPoint>,
        _around: bool,
    ) -> Option<Range<DisplayPoint>> {
        None
    }

    fn previous_range(
        &self,
        _map: &DisplaySnapshot,
        _relative_to: Range<DisplayPoint>,
        _around: bool,
    ) -> Option<Range<DisplayPoint>> {
        None
    }
}

/// Returns the start of the cursor of a selection, whether that is collapsed or not.
pub(crate) fn cursor_range(
    selection: &Selection<DisplayPoint>,
    map: &DisplaySnapshot,
) -> Range<DisplayPoint> {
    if selection.is_empty() | selection.reversed {
        selection.head()..movement::right(map, selection.head())
    } else {
        movement::left(map, selection.head())..selection.head()
    }
}

#[cfg(test)]
mod test {
    use db::indoc;

    use crate::{state::Mode, test::VimTestContext};

    #[gpui::test]
    async fn test_select_surrounding_pair_object(cx: &mut gpui::TestAppContext) {
        let mut cx = VimTestContext::new(cx, true).await;
        cx.enable_helix();

        cx.set_state("hello (woˇrld) test", Mode::HelixNormal);
        cx.simulate_keystrokes("m i m");
        cx.assert_state("hello («worldˇ») test", Mode::HelixNormal);

        cx.set_state("hello (woˇrld) test", Mode::HelixNormal);
        cx.simulate_keystrokes("m a m");
        cx.assert_state("hello «(world)ˇ» test", Mode::HelixNormal);

        // Selects innermost when nested
        cx.set_state("hello ([woˇrld]) test", Mode::HelixNormal);
        cx.simulate_keystrokes("m i m");
        cx.assert_state("hello ([«worldˇ»]) test", Mode::HelixNormal);

        // Works with double quotes
        cx.set_state("hello \"woˇrld\" test", Mode::HelixNormal);
        cx.simulate_keystrokes("m i m");
        cx.assert_state("hello \"«worldˇ»\" test", Mode::HelixNormal);

        cx.set_state("hello \"woˇrld\" test", Mode::HelixNormal);
        cx.simulate_keystrokes("m a m");
        cx.assert_state("hello «\"world\"ˇ» test", Mode::HelixNormal);

        // Works with single quotes
        cx.set_state("hello 'woˇrld' test", Mode::HelixNormal);
        cx.simulate_keystrokes("m i m");
        cx.assert_state("hello '«worldˇ»' test", Mode::HelixNormal);

        // Selects innermost when quotes are nested inside brackets
        cx.set_state("(\"foˇo\")", Mode::HelixNormal);
        cx.simulate_keystrokes("m i m");
        cx.assert_state("(\"«fooˇ»\")", Mode::HelixNormal);
    }

    #[gpui::test]
    async fn test_select_word_object(cx: &mut gpui::TestAppContext) {
        let mut cx = VimTestContext::new(cx, true).await;
        let start = indoc! {"
                The quick brˇowˇnˇ
                fox «ˇjumps» ov«er
                the laˇ»zy dogˇ

                "
        };

        cx.set_state(start, Mode::HelixNormal);

        cx.simulate_keystrokes("m i w");

        cx.assert_state(
            indoc! {"
            The quick «brownˇ»
            fox «jumpsˇ» over
            the «lazyˇ» dogˇ

            "
            },
            Mode::HelixNormal,
        );

        cx.set_state(start, Mode::HelixNormal);

        cx.simulate_keystrokes("m a w");

        cx.assert_state(
            indoc! {"
            The quick« brownˇ»
            fox «jumps ˇ»over
            the «lazy ˇ»dogˇ

            "
            },
            Mode::HelixNormal,
        );
    }
}
