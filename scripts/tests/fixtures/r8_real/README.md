Unmodified excerpts of the real R8 9.3.16 output of the release build (CI run "Fix R8 keep checker #7",
artifact `r8-mapping-release`). Lines were selected by pattern, never edited:

* `mapping.txt`: the header comments, the blocks of `RpxNativeException` and its subclasses, `RustBridge`
  (only `<clinit>` is listed: R8 writes renamed or line-numbered members only), `RustEngine*`, `DocumentCard`,
  Compose classes with `-` and `$$inlined$` in their names, classes that R8 marks `R8$$REMOVED$$CLASS$$N`.
* `seeds.txt`: every line that mentions `RustBridge` or `RpxNativeException` plus 12 unrelated lines.
* `usage.txt`: the first lines and the two places where our classes appear (removed members such as
  `$stable`, `INSTANCE`, `LIBRARY`, `<clinit>`).
Used by `scripts/tests/test_check_r8_keep.py`. Do not reformat these files.
