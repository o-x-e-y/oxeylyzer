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
    Other,
    Invalid,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum Hand {
    Left,
    Right,
}

use Hand::*;

impl std::ops::Not for Hand {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Left => Right,
            Right => Left,
        }
    }
}

impl From<Finger> for Hand {
    fn from(value: Finger) -> Self {
        value.hand()
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Finger {
    LP,
    LR,
    LM,
    LI,
    RI,
    RM,
    RR,
    RP,
    LT,
    RT,
}

use Finger::*;

impl std::fmt::Display for Finger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let to_write = match self {
            LP => "left pinky",
            LR => "left ring",
            LM => "left middle",
            LI => "left index",
            RI => "right index",
            RM => "right middle",
            RR => "right ring",
            RP => "right pinky",
            LT => "left thumb",
            RT => "right thumb",
        };
        write!(f, "{}", to_write)
    }
}

impl Finger {
    pub const fn eq(self, other: Self) -> bool {
        self as u8 == other as u8
    }

    pub const fn gt(self, other: Self) -> bool {
        self as u8 > other as u8
    }

    pub const fn lt(self, other: Self) -> bool {
        (self as u8) < (other as u8)
    }

    const fn hand(&self) -> Hand {
        match self {
            LP | LR | LM | LI | LT => Left,
            _ => Right,
        }
    }

    const fn is_bad(&self) -> bool {
        match self {
            LP | LR | LM | RM | RR | RP => true,
            _ => false,
        }
    }

    pub const fn from_usize(value: usize) -> Self {
        match value {
            0 => LP,
            1 => LR,
            2 => LM,
            3 => LI,
            4 => RI,
            5 => RM,
            6 => RR,
            7 => RP,
            8 => LT,
            9 => RT,
            _ => unreachable!(),
        }
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
    const fn new(f1: Finger, f2: Finger, f3: Finger) -> Self {
        Trigram {
            f1,
            f2,
            f3,
            h1: f1.hand(),
            h2: f2.hand(),
            h3: f3.hand(),
        }
    }

    const fn is_alt(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Right, Left) => true,
            (Right, Left, Right) => true,
            _ => false,
        }
    }

    const fn is_sfs(&self) -> bool {
        self.f1.eq(self.f3)
    }

    const fn get_alternate(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_sfs() {
            true => AlternateSfs,
            false => Alternate,
        }
    }

    const fn is_roll(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => true,
            (Right, Left, Left) => true,
            (Right, Right, Left) => true,
            (Left, Right, Right) => true,
            _ => false,
        }
    }

    const fn is_inroll(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => self.f1.lt(self.f2),
            (Right, Left, Left) => self.f2.lt(self.f3),
            (Right, Right, Left) => self.f1.gt(self.f2),
            (Left, Right, Right) => self.f2.gt(self.f3),
            _ => unreachable!(),
        }
    }

    const fn get_roll(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_inroll() {
            true => Inroll,
            false => Outroll,
        }
    }

    const fn on_one_hand(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Left) => true,
            (Right, Right, Right) => true,
            _ => false,
        }
    }

    const fn is_redir(&self) -> bool {
        (self.f1.lt(self.f2) == self.f2.gt(self.f3)) && self.on_one_hand()
    }

    const fn is_bad_redir(&self) -> bool {
        self.is_redir() && self.f1.is_bad() && self.f2.is_bad() && self.f3.is_bad()
    }

    const fn has_sfb(&self) -> bool {
        self.f1.eq(self.f2) || self.f2.eq(self.f3)
    }

    const fn is_sft(&self) -> bool {
        self.f1.eq(self.f2) && self.f2.eq(self.f3)
    }

    const fn get_one_hand(&self) -> TrigramPattern {
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

    const fn get_trigram_pattern(&self) -> TrigramPattern {
        if self.is_alt() {
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

const fn get_trigram_combinations() -> [TrigramPattern; 512] {
    let mut combinations: [TrigramPattern; 512] = [TrigramPattern::Other; 512];

    let mut c3 = 0;
    while c3 < 8 {
        let mut c2 = 0;
        while c2 < 8 {
            let mut c1 = 0;
            while c1 < 8 {
                let index = c3 * 64 + c2 * 8 + c1;
                let trigram = Trigram::new(
                    Finger::from_usize(c3),
                    Finger::from_usize(c2),
                    Finger::from_usize(c1),
                );
                combinations[index] = trigram.get_trigram_pattern();
                c1 += 1;
            }
            c2 += 1;
        }
        c3 += 1;
    }
    combinations
}

pub static TRIGRAM_COMBINATIONS: [TrigramPattern; 512] = get_trigram_combinations();

#[cfg(test)]
mod tests {
    use super::{TrigramPattern::*, *};
    use crate::*;
    use layout::{FastLayout, Layout};
    use once_cell::sync::Lazy;
    use utility::ConvertU8;

    static CON: Lazy<ConvertU8> =
        Lazy::new(|| ConvertU8::with_chars("abcdefghijklmnopqrstuvwxyz',.;"));

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
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['h', 'o', 't'])),
            Alternate
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['l', 'o', 'w'])),
            Alternate
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['l', 'a', 'z'])),
            AlternateSfs
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['i', 'r', 'k'])),
            AlternateSfs
        );

        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['a', 'b', 'c'])),
            Outroll
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['u', '\'', 'v'])),
            Outroll
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['t', 'h', 'e'])),
            Inroll
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['o', 'u', 't'])),
            Inroll
        );

        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['r', 't', 'h'])),
            Onehand
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['h', 't', 'r'])),
            Onehand
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['o', 'e', 'u'])),
            Onehand
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy([';', '.', 'x'])),
            Onehand
        );

        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['r', 'd', 's'])),
            Redirect
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['f', 'n', 'w'])),
            Redirect
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['c', 'b', 't'])),
            RedirectSfs
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['t', 'b', 'c'])),
            RedirectSfs
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['d', 's', 'f'])),
            RedirectSfs
        );

        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['r', 't', 's'])),
            BadRedirect
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['s', 't', 'r'])),
            BadRedirect
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['a', 'j', 'a'])),
            BadRedirectSfs
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['j', 'a', 'j'])),
            BadRedirectSfs
        );

        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['g', 'h', 't'])),
            BadSfb
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['u', 'p', '.'])),
            BadSfb
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['p', 'u', 'k'])),
            Sft
        );
        assert_eq!(
            dvorak.get_trigram_pattern(&CON.to_trigram_lossy(['n', 'v', 'r'])),
            Sft
        );
    }
}
