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
            _ => Self::Continue
        }
    }
}

impl Repl {
    pub fn walk(&mut self, value: &Sexp) -> Result<(String, ReplStatus)> {
        match value {
            Sexp::Atom(atom) => Ok((atom.to_string(), ReplStatus::Continue)),
            Sexp::List(sexps) => {
                let (command_parts, statuses) = sexps
                    .iter()
                    .map(|v| match v {
                        Sexp::Atom(_) => self.walk(v),
                        Sexp::List(_) => {
                            let (result, status) = self.walk(v)?;

                            Ok((format!("{:x}", md5::compute(&result)), status))
                        }
                    })
                    .collect::<Result<(Vec<_>, Vec<_>)>>()?;
                
                let subcommand = command_parts.join(" ");
                let repl_status = statuses.into_iter().reduce(|acc, e| acc + e).unwrap_or(ReplStatus::Continue);

                println!("subcommand: {subcommand}");
                let repl_status = repl_status + self.parse_flags(&subcommand)?;

                Ok((subcommand, repl_status))
            }
        }
    }

    pub fn respond(&mut self, line: &str) -> Result<ReplStatus> {
        self.clear_temp_layouts();

        let expr = add_outside_parens(line);

        let sexp = sexp::parse(&expr).map_err(|e| ReplError::SexpError(e.message.to_string()))?;

        let (final_command, repl_status) = self.walk(&sexp)?;

        println!("final command: {}", final_command);

        Ok(dbg!(repl_status))
    }

    fn parse_flags(&mut self, line: &str) -> Result<ReplStatus> {
        use crate::flags::{Repl, ReplCmd::*};

        let args = shlex::split(line)
            .ok_or(ReplError::ShlexError)?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let flags = Repl::from_vec(args)?;

        let layout = match flags.subcommand {
            Analyze(a) => self.analyze(&a.name_or_nr)?,
            Compare(c) => self.compare(&c.name1, &c.name2)?,
            Swap(s) => self.swap(&s.name, &s.swaps)?,
            Rank(_) => self.rank(),
            Generate(g) => self.generate(g.count)?,
            Improve(i) => self.improve(&i.name, i.count, i.pins)?,
            Save(s) => self.save(s.n, s.name)?,
            Sfbs(s) => self.sfbs(&s.name, s.count)?,
            Fspeed(s) => self.fspeed(&s.name, s.count)?,
            Stretches(s) => self.stretches(&s.name, s.count)?,
            Language(l) => self.language(l.language)?,
            Include(l) => self.include(&l.languages)?,
            Languages(_) => self.languages()?,
            Load(l) => self.load(l.language, l.all, l.raw)?,
            Ngram(n) => self.ngram(&n.ngram)?,
            Reload(_) => self.reload()?,
            Quit(_) => return Ok(ReplStatus::Quit),
        };

        if let Some(layout) = layout {
            self.insert_temp_layout(line, layout);
        }

        Ok(ReplStatus::Continue)
    }
}
