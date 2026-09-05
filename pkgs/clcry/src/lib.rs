pub mod buffer;
pub mod direction;
pub mod grid;
pub mod linear_layout;
pub mod progress_bar;
pub mod style;
pub mod text;
pub mod view;

#[cfg(test)]
mod test_utils;

pub use buffer::{Buffer, Cell, CellStyle, Color};
pub use direction::Direction;
pub use grid::{Grid, GridColumn, GridTrack};
pub use linear_layout::{ContentAlignment, LinearLayout, NoWrap, Wrap};
pub use progress_bar::ProgressBar;
pub use style::{BorderStyle, Insets, ViewStyle, ViewStyleExt};
pub use text::{Text, TextExt, TextRun};
pub use view::{AxisConstraint, Constraints, Rect, Size, View};

pub type Flex = LinearLayout<Wrap>;
pub type Stack = LinearLayout<NoWrap>;

/// Creates a wrapping text view from plain and styled fragments.
#[macro_export]
macro_rules! text {
	($($fragment:expr),* $(,)?) => {{
		let mut text = $crate::Text::empty();
		$(
			text = text.push($fragment);
		)*
		text
	}};
}

/// Creates a non-wrapping horizontal layout from view expressions.
#[macro_export]
macro_rules! hstack {
	(.. $children:expr) => {
		$crate::Stack::new($crate::Direction::Row, $children)
	};
	($($child:expr),* $(,)?) => {
		$crate::Stack::new(
			$crate::Direction::Row,
			vec![$(Box::new($child) as Box<dyn $crate::View>),*],
		)
	};
}

/// Creates a non-wrapping vertical layout from view expressions.
#[macro_export]
macro_rules! vstack {
	(.. $children:expr) => {
		$crate::Stack::new($crate::Direction::Column, $children)
	};
	($($child:expr),* $(,)?) => {
		$crate::Stack::new(
			$crate::Direction::Column,
			vec![$(Box::new($child) as Box<dyn $crate::View>),*],
		)
	};
}

/// Creates a wrapping horizontal layout from view expressions.
#[macro_export]
macro_rules! hflex {
	(.. $children:expr) => {
		$crate::Flex::new($crate::Direction::Row, $children)
	};
	($($child:expr),* $(,)?) => {
		$crate::Flex::new(
			$crate::Direction::Row,
			vec![$(Box::new($child) as Box<dyn $crate::View>),*],
		)
	};
}

/// Creates a wrapping vertical layout from view expressions.
#[macro_export]
macro_rules! vflex {
	(.. $children:expr) => {
		$crate::Flex::new($crate::Direction::Column, $children)
	};
	($($child:expr),* $(,)?) => {
		$crate::Flex::new(
			$crate::Direction::Column,
			vec![$(Box::new($child) as Box<dyn $crate::View>),*],
		)
	};
}
