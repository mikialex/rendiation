# Shadergraph WGSL Support

The supporting divided into 2 part:

* ast parser
    * used in shader parsing in rust procedure macro
* codegen-graph
    * generate the entire wgsl shader module from the shadergraph core data structure

## Why we not just using the naga?

* In future development, we aimed for provide better error report, this require us build a polished parser
* a playground for practice parsing technique