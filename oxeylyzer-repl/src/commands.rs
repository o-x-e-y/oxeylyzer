use sexp::Sexp;

use crate::repl::*;

fn add_outside_parens(expr: &str) -> String {
    match expr.starts_with('(') {
        true => expr.to_string(),
        _ => format!("({})", expr),
    }
}

impl std::ops::Add for ReplStatus {
    type Output = ReplStatus;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Quit, _) | (_, Self::Quit) => Self::Quit,
            _ => Self::Continue,
        }
    }
}

impl Repl {
    fn walk_sexp(&mut self, value: &Sexp, depth: usize) -> Result<(String, String, ReplStatus)> {
        match value {
            Sexp::Atom(atom) => Ok((atom.to_string(), atom.to_string(), ReplStatus::Continue)),
            Sexp::List(sexps) => {
                let (command_parts, original_parts, statuses) = sexps
                    .iter()
                    .map(|v| match v {
                        Sexp::Atom(_) => self.walk_sexp(v, depth + 1),
                        Sexp::List(_) => {
                            let (result, original, status) = self.walk_sexp(v, depth + 1)?;

                            Ok((md5_hash(&result), format!("({original})"), status))
                        }
                    })
                    .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

                let subcommand = command_parts.join(" ");
                let original = original_parts.join(" ");
                let repl_status = statuses
                    .into_iter()
                    .reduce(|acc, e| acc + e)
                    .unwrap_or(ReplStatus::Continue);

                let repl_status = repl_status + self.parse_flags(&subcommand, &original, depth)?;

                Ok((subcommand, original, repl_status))
            }
        }
    }

    pub fn walk(&mut self, value: &Sexp) -> Result<ReplStatus> {
        self.walk_sexp(value, 0).map(|(_, _, status)| status)
    }

    pub fn respond(&mut self, line: &str) -> Result<ReplStatus> {
        self.clear_temp_layouts();

        let expr = add_outside_parens(line);

        let sexp = sexp::parse(&expr).map_err(|e| ReplError::SexpError(e.message.to_string()))?;

        self.walk(&sexp)
    }

    fn parse_flags(&mut self, command: &str, original: &str, depth: usize) -> Result<ReplStatus> {
        use crate::flags::{Repl, ReplCmd::*};

        let args = shlex::split(command)
            .ok_or(ReplError::ShlexError)?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let flags = Repl::from_vec(args)?;

        let response = match flags.subcommand {
            Analyze(a) => self.analyze(&a.name_or_nr),
            Compare(c) => self.compare(&c.name1, &c.name2),
            Swap(s) => self.swap(&s.name, &s.swaps),
            Rank(_) => self.rank(),
            Generate(i) => self.generate(&i.name, i.count, i.pins),
            Save(s) => self.save(s.n, s.name),
            Sfbs(s) => self.sfbs(&s.name, s.count),
            Fspeed(s) => self.fspeed(&s.name, s.count),
            Stretches(s) => self.stretches(&s.name, s.count),
            Scissors(s) => self.scissors(&s.name, s.count),
            Lsbs(s) => self.lsbs(&s.name, s.count),
            Pinkyring(s) => self.pinky_ring(&s.name, s.count),
            Language(l) => self.language(l.language),
            Include(l) => self.include(&l.languages),
            Languages(_) => self.languages(),
            Load(l) => self.load(l.language, l.all, l.raw),
            Ngram(n) => self.ngram(&n.ngram),
            Reload(_) => self.reload(),
            Quit(_) => return Ok(ReplStatus::Quit),
        };

        let response = match response {
            Ok(response) => response,
            Err(ReplError::CommandDoesNotReturnLayout(_)) => {
                return Err(ReplError::CommandDoesNotReturnLayout(original.to_string()));
            }
            Err(e) => return Err(e),
        };

        use ReplResponse as RR;

        match (response, depth) {
            (RR::NoLayout { printable }, 0) => println!("{printable}"),
            (RR::SingleLayout { layout, printable }, 0) => {
                self.insert_temp_layout(command, *layout);
                println!("{printable}");
            }
            (RR::MultipleLayouts { layouts, printable }, 0) => {
                self.insert_temp_layout(command, layouts[0].clone());
                println!("{printable}");
            }
            (RR::SingleLayout { layout, .. }, _) => self.insert_temp_layout(command, *layout),
            (RR::MultipleLayouts { layouts, .. }, _) => {
                self.insert_temp_layout(command, layouts[0].clone())
            }
            _ => {}
        }

        Ok(ReplStatus::Continue)
    }
}
