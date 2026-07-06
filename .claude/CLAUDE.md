# Rendiation Project Instructions

**Before writing code, check skills first**, This project maintains skill documents under `.claude/skills/` that cover all major subsystems.Before reading source code or using any project-specific API, scan the available skill list and
invoke matching skills. Only explore source files for details the skills don't cover.

Do not use any form of long `======` or `------` in comment.

Do not use any ordered list(or ordering number) in comment, for example `// 1. some comment`

Do not write explict type if the type can be inferred by compiler. For code that using rust iterator collect, ONLY write the target container type if the container type can not be inferred by compiler, for example: `let result: Vec<_> = some_iter.collect();`

Use `cargo fmt` to format code after writing any rust code in project.
