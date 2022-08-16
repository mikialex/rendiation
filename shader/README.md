# RRF Shader Abstraction System

## Features planed:

rust api(all type checked, soundness as possible, elegant) for runtime shader ast building(WIP)

rust api for shader ast post processing, shader component system, nice application integration.

rust subset to shader, any shader language to the other, ast level(procedure macro ast translation)

## Design choice: pure data flow graph or ad hoc shader builder?

For simplicity and soundness, we should always stick to the pure DFG concept of our shadergraph. However, it's still super useful to create or embed ast level structured control flow into the DFG. Our solution nowadays is a mixed one between the DFG and AST ad hoc builder. The AST control flow node can depend nodes, read write, generate new node in the scope, and each scope contains a standalone DFG structure.