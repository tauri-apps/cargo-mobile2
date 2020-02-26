# `ginit-bundle`

This tool can be used for 2 things (wow!):
1. Packaging ginit
2. Packaging plugins

Which of those behaviors to take is determined by the current directory. If you're in ginit's manifest root, it'll package ginit... and if you're in a plugin's manifest root, it'll package the plugin! Crazy stuff.

By default, bundles will end up in a `bundles` folder within said manifest root. Of course, Francesca's so lovely that `ginit-bundle` will inform you of the location anyway...
