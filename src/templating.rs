use crate::{CONFIG, NAME};
use bicycle::{handlebars::handlebars_helper, Bicycle, EscapeFn, HelperDef, JsonMap};
use std::collections::HashMap;

handlebars_helper!(prefix_path: |path: str|
    CONFIG.prefix_path(path)
        .to_str()
        .expect("Prefixed path contained invalid unicode")
        .to_owned()
);

handlebars_helper!(unprefix_path: |path: str|
    CONFIG.unprefix_path(path)
        .to_str()
        .expect("Unprefixed path contained invalid unicode")
        .to_owned()
);

pub fn init_templating() -> Bicycle {
    Bicycle::new(
        EscapeFn::None,
        {
            let mut helpers = HashMap::<_, Box<dyn HelperDef>>::new();
            helpers.insert("prefix_path", Box::new(prefix_path));
            helpers.insert("unprefix_path", Box::new(unprefix_path));
            helpers
        },
        {
            let mut map = JsonMap::default();
            map.insert("tool_name", &*NAME);
            map
        },
    )
}
