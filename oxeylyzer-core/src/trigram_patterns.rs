use libdof::dofinitions::{Finger, Finger::*, Hand, Hand::*};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TrigramPattern {
    Alternate,
    AlternateSfs,
    Inroll,
    Outroll,
    Onehand,
    Redirect,
    RedirectSfs,
    BadRedirect,
    BadRedirectSfs,
    Sfb,
    BadSfb,
    Sft,
    Thumb,
    Other,
    Invalid,
}

trait TrigramFinger {
    fn eq(self, other: Self) -> bool;

    fn gt(self, other: Self) -> bool;

    fn lt(self, other: Self) -> bool;

    fn is_bad(&self) -> bool;
}

impl TrigramFinger for Finger {
    fn eq(self, other: Self) -> bool {
        self as u8 == other as u8
    }

    fn gt(self, other: Self) -> bool {
        self as u8 > other as u8
    }

    fn lt(self, other: Self) -> bool {
        (self as u8) < (other as u8)
    }

    fn is_bad(&self) -> bool {
        matches!(self, LP | LR | LM | RM | RR | RP)
    }
}

#[derive(Debug)]
pub(crate) struct Trigram {
    f1: Finger,
    f2: Finger,
    f3: Finger,
    h1: Hand,
    h2: Hand,
    h3: Hand,
}

impl std::fmt::Display for Trigram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}", self.f1, self.f2, self.f3)
    }
}

impl Trigram {
    fn new(f1: Finger, f2: Finger, f3: Finger) -> Self {
        Trigram {
            f1,
            f2,
            f3,
            h1: f1.hand(),
            h2: f2.hand(),
            h3: f3.hand(),
        }
    }

    fn is_thumb(&self) -> bool {
        self.f1.is_thumb() || self.f2.is_thumb() || self.f3.is_thumb()
    }

    fn is_alt(&self) -> bool {
        matches!(
            (self.h1, self.h2, self.h3),
            (Left, Right, Left) | (Right, Left, Right)
        )
    }

    fn is_sfs(&self) -> bool {
        self.f1.eq(self.f3)
    }

    fn get_alternate(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_sfs() {
            true => AlternateSfs,
            false => Alternate,
        }
    }

    fn is_roll(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => true,
            (Right, Left, Left) => true,
            (Right, Right, Left) => true,
            (Left, Right, Right) => true,
            _ => false,
        }
    }

    fn is_inroll(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => self.f1.lt(self.f2),
            (Right, Left, Left) => self.f2.lt(self.f3),
            (Right, Right, Left) => self.f1.gt(self.f2),
            (Left, Right, Right) => self.f2.gt(self.f3),
            _ => unreachable!(),
        }
    }

    fn get_roll(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_inroll() {
            true => Inroll,
            false => Outroll,
        }
    }

    fn on_one_hand(&self) -> bool {
        matches!(
            (self.h1, self.h2, self.h3),
            (Left, Left, Left) | (Right, Right, Right)
        )
    }

    fn is_redir(&self) -> bool {
        (self.f1.lt(self.f2) == self.f2.gt(self.f3)) && self.on_one_hand()
    }

    fn is_bad_redir(&self) -> bool {
        self.is_redir() && self.f1.is_bad() && self.f2.is_bad() && self.f3.is_bad()
    }

    fn has_sfb(&self) -> bool {
        self.f1.eq(self.f2) || self.f2.eq(self.f3)
    }

    fn is_sft(&self) -> bool {
        self.f1.eq(self.f2) && self.f2.eq(self.f3)
    }

    fn get_one_hand(&self) -> TrigramPattern {
        use TrigramPattern::*;

        if self.is_sft() {
            Sft
        } else if self.has_sfb() {
            BadSfb
        } else if self.is_redir() {
            match (self.is_sfs(), self.is_bad_redir()) {
                (false, false) => Redirect,
                (false, true) => BadRedirect,
                (true, false) => RedirectSfs,
                (true, true) => BadRedirectSfs,
            }
        } else {
            Onehand
        }
    }

    fn get_trigram_pattern(&self) -> TrigramPattern {
        if self.is_thumb() {
            TrigramPattern::Thumb
        } else if self.is_alt() {
            self.get_alternate()
        } else if self.on_one_hand() {
            self.get_one_hand()
        } else if self.has_sfb() {
            TrigramPattern::Sfb
        } else if self.is_roll() {
            self.get_roll()
        } else {
            TrigramPattern::Other
        }
    }
}

pub fn get_trigram_combinations() -> Box<[TrigramPattern]> {
    let mut combinations = [TrigramPattern::Other; 1000];

    for f3 in Finger::FINGERS {
        for f2 in Finger::FINGERS {
            for f1 in Finger::FINGERS {
                let index = (f3 as usize) * 100 + (f2 as usize) * 10 + (f1 as usize);
                let trigram = Trigram::new(f3, f2, f1);
                combinations[index] = trigram.get_trigram_pattern();
            }
        }
    }

    Box::new(combinations)
}

#[cfg(test)]
mod tests {
    use super::{TrigramPattern::*, *};
    use crate::{char_mapping::CharMapping, generate::LayoutGeneration, layout::FastLayout};
    use once_cell::sync::Lazy;

    static CON: Lazy<CharMapping> =
        Lazy::new(|| CharMapping::from("abcdefghijklmnopqrstuvwxyz',.;"));

    static GEN: Lazy<LayoutGeneration> =
        Lazy::new(|| LayoutGeneration::new("english", "./static", None).unwrap());

    #[test]
    fn is_alt() {
        let t1 = Trigram::new(LR, LM, LI);
        let t2 = Trigram::new(LR, RP, LI);
        let t3 = Trigram::new(RM, LM, RM);

        assert!(!t1.is_alt());
        assert!(t2.is_alt());
        assert!(t3.is_alt());
    }

    #[test]
    fn rolls() {
        let t1 = Trigram::new(LR, LM, RR);
        let t2 = Trigram::new(RR, LR, LM);
        let t3 = Trigram::new(LM, LR, RR);
        let t4 = Trigram::new(RR, LM, LR);
        let t5 = Trigram::new(LP, RI, RM);

        assert!(!(t1.on_one_hand() || t1.is_alt() || t1.has_sfb()));
        assert!(!(t2.on_one_hand() || t2.is_alt() || t2.has_sfb()));
        assert!(!(t3.on_one_hand() || t3.is_alt() || t3.has_sfb()));
        assert!(!(t4.on_one_hand() || t4.is_alt() || t4.has_sfb()));
        assert!(!(t5.on_one_hand() || t5.is_alt() || t5.has_sfb()));

        assert!(t1.is_roll());
        assert!(t2.is_roll());
        assert!(t3.is_roll());
        assert!(t4.is_roll());
        assert!(t5.is_roll());

        assert!(t1.is_inroll());
        assert!(t2.is_inroll());
        assert!(!t3.is_inroll());
        assert!(!t4.is_inroll());
        assert!(!t5.is_inroll());

        assert_eq!(t1.get_roll(), Inroll);
        assert_eq!(t2.get_roll(), Inroll);
        assert_eq!(t3.get_roll(), Outroll);
        assert_eq!(t4.get_roll(), Outroll);
        assert_eq!(t5.get_roll(), Outroll);
    }

    #[test]
    fn redirs() {
        let t1 = Trigram::new(LR, LI, LM);
        let t2 = Trigram::new(LM, LI, LR);
        let t3 = Trigram::new(RR, RI, RM);
        let t4 = Trigram::new(RM, RI, RR);

        assert!(t1.is_redir());
        assert!(t2.is_redir());
        assert!(t3.is_redir());
        assert!(t4.is_redir());
        assert!(!t1.is_sfs());
        assert!(!t2.is_sfs());
        assert!(!t3.is_sfs());
        assert!(!t4.is_sfs());
        assert!(!t1.is_bad_redir());
        assert!(!t2.is_bad_redir());
        assert!(!t3.is_bad_redir());
        assert!(!t4.is_bad_redir());

        let t1 = Trigram::new(LP, LI, LP);
        let t2 = Trigram::new(LI, LP, LI);
        let t3 = Trigram::new(RI, RR, RI);
        let t4 = Trigram::new(RR, RI, RR);

        assert!(t1.is_redir());
        assert!(t2.is_redir());
        assert!(t3.is_redir());
        assert!(t4.is_redir());
        assert!(t1.is_sfs());
        assert!(t2.is_sfs());
        assert!(t3.is_sfs());
        assert!(t4.is_sfs());
        assert!(!t1.is_bad_redir());
        assert!(!t2.is_bad_redir());
        assert!(!t3.is_bad_redir());
        assert!(!t4.is_bad_redir());

        let t1 = Trigram::new(LR, LP, LM);
        let t2 = Trigram::new(LM, LP, LR);
        let t3 = Trigram::new(RR, RP, RM);
        let t4 = Trigram::new(RM, RP, RR);

        assert!(t1.is_redir());
        assert!(t2.is_redir());
        assert!(t3.is_redir());
        assert!(t4.is_redir());
        assert!(!t1.is_sfs());
        assert!(!t2.is_sfs());
        assert!(!t3.is_sfs());
        assert!(!t4.is_sfs());
        assert!(t1.is_bad_redir());
        assert!(t2.is_bad_redir());
        assert!(t3.is_bad_redir());
        assert!(t4.is_bad_redir());

        let t1 = Trigram::new(LP, LR, LP);
        let t2 = Trigram::new(LR, LP, LR);
        let t3 = Trigram::new(RM, RR, RM);
        let t4 = Trigram::new(RR, RM, RR);

        assert!(t1.is_redir());
        assert!(t2.is_redir());
        assert!(t3.is_redir());
        assert!(t4.is_redir());
        assert!(t1.is_sfs());
        assert!(t2.is_sfs());
        assert!(t3.is_sfs());
        assert!(t4.is_sfs());
        assert!(t1.is_bad_redir());
        assert!(t2.is_bad_redir());
        assert!(t3.is_bad_redir());
        assert!(t4.is_bad_redir());
    }

    #[test]
    fn trigram_combinations() {
        let dvorak_bytes = CON.to_lossy("',.pyfgcrlaoeuidhtns;qjkxbmwvz".chars());
        let dvorak = FastLayout::try_from(dvorak_bytes.as_slice()).expect("couldn't create dvorak");

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['h', 'o', 't'])),
            Alternate
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['l', 'o', 'w'])),
            Alternate
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['l', 'a', 'z'])),
            AlternateSfs
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['i', 'r', 'k'])),
            AlternateSfs
        );

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['a', 'b', 'c'])),
            Outroll
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['u', '\'', 'v'])),
            Outroll
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['t', 'h', 'e'])),
            Inroll
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['o', 'u', 't'])),
            Inroll
        );

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['r', 't', 'h'])),
            Onehand
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['h', 't', 'r'])),
            Onehand
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['o', 'e', 'u'])),
            Onehand
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy([';', '.', 'x'])),
            Onehand
        );

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['r', 'd', 's'])),
            Redirect
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['f', 'n', 'w'])),
            Redirect
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['c', 'b', 't'])),
            RedirectSfs
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['t', 'b', 'c'])),
            RedirectSfs
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['d', 's', 'f'])),
            RedirectSfs
        );

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['r', 't', 's'])),
            BadRedirect
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['s', 't', 'r'])),
            BadRedirect
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['a', 'j', 'a'])),
            BadRedirectSfs
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['j', 'a', 'j'])),
            BadRedirectSfs
        );

        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['g', 'h', 't'])),
            BadSfb
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['u', 'p', '.'])),
            BadSfb
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['p', 'u', 'k'])),
            Sft
        );
        assert_eq!(
            GEN.get_trigram_pattern(&dvorak, &CON.to_trigram_lossy(['n', 'v', 'r'])),
            Sft
        );
    }
}
