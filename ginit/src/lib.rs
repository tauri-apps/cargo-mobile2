// pub mod cargo;
// pub mod init;
// pub mod steps;

use ginit_core::DynPlugin;
use std::collections::HashMap;

pub struct Instance<'a> {
    plugins: HashMap<&'a str, Box<dyn DynPlugin>>,
}

// must be able to query if it has a target
