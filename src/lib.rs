extern crate emacs;
extern crate rmpv;

use emacs::{Env, Result};

pub mod msgpack;

emacs::plugin_is_GPL_compatible!();

#[emacs::module(mod_in_name = false)]
fn init(_: &Env) -> Result<()> {
    Ok(())
}
