use sexp::Sexp;

use crate::repl::Repl;

fn add_outside_parens(expr: &str) -> String {
    match expr.starts_with('(') {
        true => expr.to_string(),
        _ => format!("({})", expr),
    }
}

impl Repl {
    pub fn walk(&self, value: &Sexp) -> String {
        match value {
            Sexp::Atom(atom) => atom.to_string(),
            Sexp::List(sexps) => {
                let subcommand = sexps
                    .iter()
                    .map(|v| match v {
                        Sexp::Atom(_) => self.walk(v),
                        Sexp::List(_) => {
                            let result = self.walk(v);

                            format!("{:x}", md5::compute(&result))
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" ");

                // self.respond();

                println!("subcommand: {subcommand}");

                subcommand
            }
        }
    }
}
