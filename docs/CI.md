# CI guide (`.github/workflows/android.yml`)

CI is a development tool. The shipped app has no network code and no INTERNET permission; CI only builds and checks it.

## Jobs
| Job | Purpose | Blocks the workflow |
|---|---|---|
| `bootstrap` | reuse (or, if absent, generate) `Cargo.lock` and the Gradle wrapper; verify third-party action pins online; check that `./gradlew --version` prints exactly the pinned Gradle; upload them as `bootstrap-files` | yes |
| `rust` | strict static checks, Python tests of the check scripts, `cargo test --locked`, clippy (report only), `cargo check` for 3 Android targets | yes (clippy output is informational) |
| `android` | SDK install **and verification**, unit tests, debug APK, then an unsigned R8 release build; APK checks (below); opt-in emulator tests | yes |
| `supply-chain` | `cargo deny check bans licenses sources`, offline lock-file policy; advisories informational | yes (advisories step: no) |

All jobs except `bootstrap` use the committed or bootstrapped `Cargo.lock` and wrapper (download of the same artifact), never their own resolution.

## What the APK checks do
* `scripts/verify-apk-permissions.sh` (aapt2): exits 0 only after aapt2 read the manifest and INTERNET is absent. Run for the debug and the release APK.
* `scripts/check_apk_native.py` (pure Python): every configured ABI has `lib/<abi>/librpx_core.so`, stored uncompressed; for 64-bit ABIs the zip offset is a multiple of 16384 and every ELF `PT_LOAD` has `p_align >= 0x4000`; the ELF exports every `Java_..._RustBridge_*` implied by the Kotlin `external fun`s.
* `zipalign -c -P 16 -v 4` (release APK): official zip-alignment check.
* `scripts/check_r8_keep.py`: reads AGP's `seeds.txt`, `mapping.txt`, `usage.txt` in `build/outputs/mapping/release/`. `seeds.txt` (what the keep rules matched) must list **every** `RustBridge` native method with its exact signature and the `(String)` constructor of every `RpxNativeException` subclass (Rust constructs them by name). `mapping.txt` (what R8 renamed) must map `RustBridge`, `RpxNativeException` and its subclasses to their own names, and a native method that *is* listed there must keep its name and signature (`nativeInit -> a` is an error); R8 writes only renamed or line-numbered members to `mapping.txt`, so an unchanged native method that is absent there is fine, because `seeds.txt` proves it was kept. `usage.txt` must not list them as removed. Class names may contain `$`, `-` and other characters (Kotlin mangles names such as `clickable-O2vRcR0`). Corrupt or unrecognized lines and missing or empty files fail the check; the failing lines are printed next to the expected ones.
* Gradle task `verifyNoInternet<Variant>` (hooked into `assemble*`/`bundle*`) checks the merged manifest.

The check scripts have Python tests (`python3 -m unittest discover -s scripts -p 'test_*.py'` or `-s scripts/tests`): synthetic ELF/APK/R8 samples, real system libraries for the ELF parser, and verbatim fragments of the R8 9.3.16 output of CI run #6 (`RustBridge` header line of `mapping.txt`, a Compose class name with `$` and `-`, five `seeds.txt` lines) for the R8 check. Not yet seen by the author: the full real `mapping.txt`/`seeds.txt`/`usage.txt` (the CI artifact `r8-mapping-release` needs a GitHub login), so the way R8 prints the exception-constructor lines in `seeds.txt` and the `usage.txt` layout are still assumptions; the check accepts the `<init>(java.lang.String)` and the ProGuard-style `ShortName(java.lang.String)` forms and prints the actual lines when none matches.

## Action pinning policy
* `actions/*` (GitHub-owned): release tags.
* everything else: `uses: owner/repo@<40-hex SHA> # vX.Y.Z`.
* `python3 scripts/verify_action_pins.py` checks the structure offline; with `--online` it asks `git ls-remote` and requires the SHA to be the commit of that release tag. The `bootstrap` job runs the online mode on every workflow run.
* To update: run `git ls-remote https://github.com/<owner>/<repo>.git "refs/tags/vX.Y.Z*"` (use the `^{}` line if the tag is annotated), put the SHA and the tag into the workflow and into `docs/VERSIONS.md`, run `python3 scripts/verify_action_pins.py --online`.
* Run the online check **before pushing** a pin change: a SHA that does not exist makes the job fail during setup, before any step can report it.

## cargo-deny policy (`deny.toml`, cargo-deny 0.20.2)
* bans: a **fixed list of well-known** HTTP, TLS, async-network and telemetry crates (list in `deny.toml`) anywhere in the dependency tree; wildcard version requirements (path dependencies of unpublished workspace crates excepted);
* `bans.std-replacements` (new in 0.20, built-in level `deny`) is set to `warn` explicitly: it reports crates that std now replaces (for example `once_cell` inside `android_logger`) and is a hygiene hint, not an offline or security rule;
* sources: crates.io only; no git dependencies;
* licenses: MIT, Apache-2.0, Unicode-3.0 (alternatives of an `OR` expression count);
* graph: only the three Android targets.
If `cargo deny` fails: read the offender in the log; `cargo tree -i <crate> --target aarch64-linux-android` shows who pulls it in. Do not silence a ban. A license that is genuinely acceptable may be added to `allow` after reading its terms.

**A deny-list is not a proof.** `deny.toml` only forbids crates that are on the list. A library that is not listed, but opens sockets itself (directly through `libc`, or through a crate nobody listed), passes this check. `cargo-deny` says nothing about what the code of an allowed crate does.

## What the offline checks do and do not prove
The offline promise rests on several independent checks. None of them is a proof alone; together they make a violation hard to introduce by accident.

| Layer | What it checks | What it cannot see |
|---|---|---|
| Android permission (`verifyNoInternet<Variant>` on the merged manifest, `verify-apk-permissions.sh` with aapt2 on the debug and the release APK) | the app does not request `android.permission.INTERNET`, so the system denies it ordinary network sockets | code that is merely *present*; local (loopback, UNIX-domain) sockets; other permission-free channels |
| Kotlin source scan (`check_project_consistency.py`) | forbidden API names (`java.net`, `HttpURLConnection`, `okhttp3`, `WebView`, `INTERNET`, ...) in `app/src/**.kt` | reflection, dynamically built class names, code inside AndroidX or other binary dependencies |
| Rust source scan (same script) | `std::net`, `TcpStream`, `UdpSocket`, `reqwest`, `hyper`, `ureq`, `tokio::net` in our own `crates/**.rs` | the source of dependencies, `libc` calls |
| Cargo dependency tree (`deny.toml` bans, `check_lock_policy.py` on `Cargo.lock`) | the known network, TLS and telemetry crates are absent; only crates.io sources | any network-capable crate not on the list; capabilities of allowed crates |
| Gradle dependency rules (same script) | names such as firebase, okhttp, retrofit, ktor, analytics in the build files | transitive Kotlin/Java dependencies are not enumerated by this scan |
| Manual runtime test | airplane mode, all functions still work | paths that are not exercised |

Rules that follow from the limits: every **new** dependency (Kotlin or Rust) is reviewed before it is added: its manifest and enabled features (`cargo tree -e features -i <crate>`), and whether it contains socket or TLS code; the result goes into the pull request. No existing restriction is relaxed to make a build pass.

## Opt-in emulator tests
Run workflow -> tick `run_instrumented`. API 34, x86_64, `google_apis`, on a GitHub-hosted runner with KVM; adds roughly 10 to 20 minutes. Kept manual on purpose (time, flakiness); a nightly schedule can be added once the tests have passed a few times. Status of the first run: see `docs/VERIFICATION.md`.

## Running the checks locally (Termux or desktop)
```bash
python3 scripts/check_project_consistency.py --strict
python3 scripts/check_jni_symbols.py
python3 scripts/check_lock_policy.py
python3 -m unittest discover -s scripts/tests
scripts/test_verify_apk_permissions.sh
python3 scripts/verify_action_pins.py --online      # needs git and network
```
