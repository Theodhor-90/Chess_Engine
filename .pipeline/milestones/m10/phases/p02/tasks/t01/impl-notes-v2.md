All verification commands pass. Here's the summary:

- **`tools/datagen/src/main.rs`** — Changed format specifier on line 323 from `"{} | {} | {}"` to `"{} | {} | {:.1}"` so the result field always prints with one decimal place (`1.0`, `0.5`, `0.0` instead of `1`, `0.5`, `0`), fixing nnue-pytorch `.plain` format compatibility.