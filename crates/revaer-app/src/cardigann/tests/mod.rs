//! Rule validation and real lifecycle regressions for imported public HTML.

pub(crate) const DEFINITION: &str = include_str!("public-html.yml");

mod definition;
mod runtime;
