use std::{collections::BTreeMap, sync::Arc};

use libdof::{combos::Combos, magic::Magic, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{fast_layout::FastLayout, *};

/// Type alias representing a position index on the layout.
///
/// # Examples:
/// ```
/// # use oxeylyzer_core::layout::Pos;
/// let pos: Pos = 5;
/// ```
pub type Pos = u8;

/// A pair of positions on a keyboard layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PosPair(pub Pos, pub Pos);

impl PosPair {
    /// Creates a new `PosPair`.
    ///
    /// # Examples:
    /// ```
    /// use oxeylyzer_core::layout::PosPair;
    ///
    /// let pair = PosPair::new(0, 1);
    /// assert_eq!(pair.0, 0);
    /// assert_eq!(pair.1, 1);
    /// ```
    pub const fn new(a: Pos, b: Pos) -> Self {
        Self(a, b)
    }
}

impl<U: Into<u8>> From<(U, U)> for PosPair {
    fn from((p1, p2): (U, U)) -> Self {
        Self(p1.into(), p2.into())
    }
}

impl std::fmt::Display for PosPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

/// Metadata associated with a keyboard layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutMetadata {
    /// The authors of the layout.
    pub authors: Vec<String>,
    /// The year the layout was created.
    pub year: Option<u32>,
    /// A link to more information about the layout.
    pub link: Option<String>,
    /// Languages supported by this layout.
    pub languages: Vec<Language>,
    /// The anchor point for the layout.
    pub anchor: Anchor,
    /// The name of the fingering system used.
    pub fingering_name: Option<NamedFingering>,
    /// The parsed physical keyboard layout.
    pub parsed_board: ParseKeyboard,
}

/// A keyboard layout representation containing keys, fingers, and geometry.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(from = "Dof", into = "Dof")]
pub struct Layout {
    /// The name of the layout.
    pub name: String,
    /// The characters assigned to each key.
    pub keys: Arc<[char]>,
    /// The finger assigned to each key.
    pub fingers: Arc<[Finger]>,
    /// The physical layout of the keyboard.
    pub keyboard: Arc<[PhysicalKey]>,
    /// The shape of the keyboard rows.
    pub shape: Shape,
    /// Metadata associated with the layout.
    pub metadata: Arc<LayoutMetadata>,
}

impl Layout {
    /// Loads a layout from a JSON file. This JSON file should use the [`Dof`](libdof::Dof) format,
    /// signified by its `.dof` extension.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::layout::Layout;
    /// let path = "static/layouts/gust.dof";
    ///
    /// let layout = Layout::load(path).unwrap();
    /// assert_eq!(layout.name, "Gust");
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(&path).path_context(path)?;

        serde_json::from_str::<Dof>(&s)
            .map(Into::into)
            .map_err(|e| OxeylyzerError::AnyhowError(e.into()))
    }

    /// Loads a layout from a JSON file at a specific URL. This JSON file should use the
    /// [`Dof`](libdof::Dof) format, signified by its `.dof` extension.
    ///
    /// # Examples:
    /// ```
    /// # use oxeylyzer_core::layout::Layout;
    /// let path = "static/layouts/gust.dof";
    ///
    /// let layout = Layout::load(path).unwrap();
    /// assert_eq!(layout.name, "Gust");
    /// ```
    #[cfg(target_arch = "wasm32")]
    pub async fn load(url: &str) -> Result<Self> {
        let dof = gloo_net::http::Request::get(url)
            .send()
            .await?
            .json::<Dof>()
            .await?;

        Ok(dof.into())
    }
}

impl From<Dof> for Layout {
    fn from(dof: Dof) -> Self {
        use libdof::prelude::{Key, SpecialKey};

        let keys = dof
            .main_layer()
            .keys()
            .map(|k| match k {
                Key::Char(c) => *c,
                Key::Special(s) => match s {
                    SpecialKey::Repeat => REPEAT_KEY,
                    SpecialKey::Space => SPACE_CHAR,
                    SpecialKey::Shift => SHIFT_CHAR,
                    _ => REPLACEMENT_CHAR,
                },
                _ => REPLACEMENT_CHAR,
            })
            .collect();

        let DofInternal {
            name,
            authors,
            board,
            parsed_board,
            year,
            languages,
            link,
            anchor,
            fingering,
            fingering_name,
            ..
        } = dof.into_inner();

        let fingers = fingering.keys().copied().collect();
        let keyboard = board.keys().cloned().collect();
        let shape = board.shape();

        let metadata = Arc::from(LayoutMetadata {
            authors,
            year,
            link,
            languages,
            anchor,
            fingering_name,
            parsed_board,
        });

        Layout {
            name,
            keys,
            fingers,
            keyboard,
            shape,
            metadata,
        }
    }
}

impl From<Layout> for Dof {
    fn from(layout: Layout) -> Self {
        let LayoutMetadata {
            authors,
            languages,
            anchor,
            fingering_name,
            parsed_board,
            ..
        } = layout.metadata.as_ref().clone();

        let mut key_iter = layout.keys.iter();
        let main_layer = layout
            .shape
            .inner()
            .iter()
            .map(|&len| {
                key_iter
                    .by_ref()
                    .take(len)
                    .map(|c| match *c {
                        REPLACEMENT_CHAR => Key::Empty,
                        REPEAT_KEY => Key::Special(SpecialKey::Repeat),
                        SPACE_CHAR => Key::Special(SpecialKey::Space),
                        SHIFT_CHAR => Key::Special(SpecialKey::Shift),
                        c => Key::Char(c),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut finger_iter = layout.fingers.iter();
        let fingering = layout
            .shape
            .inner()
            .iter()
            .map(|&len| finger_iter.by_ref().take(len).copied().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let mut board_iter = layout.keyboard.iter();
        let board = layout
            .shape
            .inner()
            .iter()
            .map(|&len| board_iter.by_ref().take(len).cloned().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let internal = DofInternal {
            name: layout.name,
            description: None,
            year: None,
            link: None,
            authors,
            languages,
            parsed_board,
            board: board.into(),
            layers: BTreeMap::from_iter([("main".into(), main_layer.into())]),
            anchor,
            magic: Magic::default(),
            combos: Combos::default(),
            fingering: fingering.into(),
            fingering_name,
            has_generated_shift: false,
        };

        internal.into()
    }
}

impl From<FastLayout> for Layout {
    fn from(layout: FastLayout) -> Self {
        let name = match layout.name {
            Some(name) => name,
            None => layout
                .keys
                .iter()
                .copied()
                .skip(10) // TODO: maybe make more accurate based on board shape + anchor
                .take(4)
                .map(|u| layout.mapping.get_c(u))
                .collect::<String>(),
        };

        Self {
            name,
            keys: layout
                .keys
                .iter()
                .map(|&u| layout.mapping.get_c(u))
                .collect(),
            fingers: layout.fingers,
            keyboard: layout.keyboard,
            shape: layout.shape,
            metadata: layout.metadata.clone(),
        }
    }
}

impl std::fmt::Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.name)?;

        let mut iter = self.keys.iter();

        for l in self.shape.inner().iter() {
            let mut i = 0;
            for c in iter.by_ref() {
                write!(f, "{c} ")?;
                i += 1;

                if *l == i {
                    break;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
