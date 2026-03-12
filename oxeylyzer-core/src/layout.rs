use std::{collections::BTreeMap, sync::Arc};

use libdof::{combos::Combos, magic::Magic, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    REPEAT_KEY, REPLACEMENT_CHAR, Result, SHIFT_CHAR, SPACE_CHAR, cached_layout::FastLayout,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PosPair(pub u8, pub u8);

impl<U: Into<u8>> From<(U, U)> for PosPair {
    fn from((p1, p2): (U, U)) -> Self {
        Self(p1.into(), p2.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutMetadata {
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub link: Option<String>,
    pub languages: Vec<Language>,
    pub anchor: Anchor,
    pub fingering_name: Option<NamedFingering>,
    pub parsed_board: ParseKeyboard,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(from = "Dof", into = "Dof")]
pub struct Layout {
    pub name: String,
    pub keys: Box<[char]>,
    pub fingers: Box<[Finger]>,
    pub keyboard: Box<[PhysicalKey]>,
    pub shape: Shape,
    pub metadata: Arc<LayoutMetadata>,
}

#[test]
fn thing() {
    let layout =
        serde_json::from_str::<Layout>(include_str!("../../static/layouts/english/sturdy.dof"))
            .unwrap();

    let s = serde_json::to_string_pretty(&layout).unwrap();

    println!("{s}");
}

impl Layout {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(path)?;

        serde_json::from_str::<Dof>(&s)
            .map(Into::into)
            .map_err(Into::into)
    }

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

        let mut key_iter = layout.keys.into_iter();
        let main_layer = layout
            .shape
            .inner()
            .iter()
            .map(|&len| {
                key_iter
                    .by_ref()
                    .take(len)
                    .map(|c| match c {
                        REPLACEMENT_CHAR => Key::Empty,
                        REPEAT_KEY => Key::Special(SpecialKey::Repeat),
                        SPACE_CHAR => Key::Special(SpecialKey::Space),
                        SHIFT_CHAR => Key::Special(SpecialKey::Shift),
                        c => Key::Char(c),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut finger_iter = layout.fingers.into_iter();
        let fingering = layout
            .shape
            .inner()
            .iter()
            .map(|&len| finger_iter.by_ref().take(len).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let mut board_iter = layout.keyboard.into_iter();
        let board = layout
            .shape
            .inner()
            .iter()
            .map(|&len| board_iter.by_ref().take(len).collect::<Vec<_>>())
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
                .matrix
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
                .matrix
                .iter()
                .map(|&u| layout.mapping.get_c(u))
                .collect(),
            fingers: layout.matrix_fingers,
            keyboard: layout.matrix_physical,
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
