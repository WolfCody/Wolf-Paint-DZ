#!/usr/bin/env python3
"""Static project checks that need no Android SDK, Gradle or Rust.

Checks: version-catalog references, string/drawable references, ru/en string parity,
Kotlin<->Rust exception classes, offline rules (manifest, allowBackup, forbidden
network/analytics dependencies and APIs), XML/TOML well-formedness.
Warnings (not failures): Gradle wrapper or Cargo.lock not committed yet.
Use --strict to turn those warnings into failures (CI does after bootstrap).
Exit code 1 on any failure.
"""
import glob
import os
import re
import sys
import tomllib
import xml.dom.minidom
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_lock_policy  # noqa: E402
import verify_action_pins  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
errors: list[str] = []
warnings: list[str] = []


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def check_xml():
    for f in glob.glob(str(ROOT / "app/src/**/*.xml"), recursive=True):
        try:
            xml.dom.minidom.parse(f)
        except Exception as e:  # noqa: BLE001
            errors.append(f"XML not well-formed: {os.path.relpath(f, ROOT)}: {e}")


def check_catalog():
    cat = tomllib.loads(read("gradle/libs.versions.toml"))
    libs = {k.replace("-", ".") for k in cat["libraries"]}
    plugins = {k.replace("-", ".") for k in cat["plugins"]}
    versions = set(cat["versions"])
    for k, v in {**cat["libraries"], **cat["plugins"]}.items():
        ref = v.get("version", {}).get("ref") if isinstance(v.get("version"), dict) else None
        if ref and ref not in versions:
            errors.append(f"catalog: {k} uses unknown version.ref {ref}")
    for f in ["build.gradle.kts", "app/build.gradle.kts"]:
        t = read(f)
        for m in re.findall(r"libs\.plugins\.([\w.]+)", t):
            if m not in plugins:
                errors.append(f"{f}: unknown plugin alias libs.plugins.{m}")
        for m in re.findall(r"libs\.(?!plugins|versions)([\w.]+)", t):
            if m not in libs:
                errors.append(f"{f}: unknown library alias libs.{m}")
    code_only = "\n".join(
        line for f in ("build.gradle.kts", "app/build.gradle.kts", "gradle/libs.versions.toml")
        for line in read(f).splitlines() if not line.strip().startswith(("//", "#", "*", "/*"))
    )
    if re.search(r"libs\.plugins\.kotlin\.android|org\.jetbrains\.kotlin\.android|kotlin\(\"android\"\)|^kotlin-android\s*=", code_only, re.M):
        errors.append("kotlin-android plugin must not be used with AGP 9 (built-in Kotlin)")


def kotlin_sources():
    return glob.glob(str(ROOT / "app/src/**/*.kt"), recursive=True)


def check_resources():
    en = set(re.findall(r'name="(\w+)"', read("app/src/main/res/values/strings.xml")))
    ru = set(re.findall(r'name="(\w+)"', read("app/src/main/res/values-ru/strings.xml")))
    if en != ru:
        errors.append(f"string resources differ between en and ru: {sorted(en ^ ru)}")
    code = "".join(Path(f).read_text(encoding="utf-8") for f in kotlin_sources())
    for kind, name in set(re.findall(r"R\.(string|array|drawable|mipmap)\.(\w+)", code)):
        if kind in ("string", "array"):
            if name not in en:
                errors.append(f"R.{kind}.{name} used but not defined")
        elif not glob.glob(str(ROOT / f"app/src/main/res/{kind}*/{name}.*")):
            errors.append(f"R.{kind}.{name} used but file missing")


def check_exceptions():
    rust = read("crates/core-ffi/src/lib.rs")
    kotlin = read("app/src/main/kotlin/com/dzekiwolf/wolfpaintdz/core/RpxNativeException.kt")
    r = set(re.findall(r'RpxNativeException\$(\w+)"', rust))
    k = set(re.findall(r"class (\w+)\(message: String\) : RpxNativeException", kotlin))
    if r != k:
        errors.append(f"exception classes differ Rust vs Kotlin: {sorted(r ^ k)}")


def check_offline():
    manifest = read("app/src/main/AndroidManifest.xml")
    doc = xml.dom.minidom.parseString(manifest)
    if doc.getElementsByTagName("uses-permission"):
        errors.append("source manifest declares <uses-permission> (offline app: none expected)")
    app = doc.getElementsByTagName("application")[0]
    if app.getAttribute("android:allowBackup") != "false":
        errors.append('android:allowBackup must be "false"')
    forbidden_deps = re.compile(r"firebase|crashlytics|okhttp|retrofit|ktor|volley|admob|play-services-ads|analytics|sentry|datadog", re.I)
    for f in ["gradle/libs.versions.toml", "app/build.gradle.kts", "build.gradle.kts", "settings.gradle.kts"]:
        for line in read(f).splitlines():
            if forbidden_deps.search(line) and not line.strip().startswith(("//", "#")):
                errors.append(f"{f}: forbidden network/analytics dependency: {line.strip()}")
    api = re.compile(r"java\.net\.|HttpURLConnection|URLConnection|okhttp3|android\.net\.http|Socket\(|WebView|INTERNET")
    for f in kotlin_sources():
        for n, line in enumerate(Path(f).read_text(encoding="utf-8").splitlines(), 1):
            if api.search(line) and not line.strip().startswith(("//", "*", "/*")):
                errors.append(f"{os.path.relpath(f, ROOT)}:{n}: network API: {line.strip()}")
    rust_api = re.compile(r"std::net|TcpStream|UdpSocket|reqwest|hyper|ureq|tokio::net")
    for f in glob.glob(str(ROOT / "crates/**/*.rs"), recursive=True):
        for n, line in enumerate(Path(f).read_text(encoding="utf-8").splitlines(), 1):
            if rust_api.search(line):
                errors.append(f"{os.path.relpath(f, ROOT)}:{n}: network API: {line.strip()}")
    for f in glob.glob(str(ROOT / "crates/*/Cargo.toml")) + [str(ROOT / "Cargo.toml")]:
        if re.search(r"reqwest|hyper|ureq|tokio|openssl|rustls", Path(f).read_text(encoding="utf-8")):
            errors.append(f"{os.path.relpath(f, ROOT)}: network-capable crate declared")


def check_cargo():
    root = tomllib.loads(read("Cargo.toml"))
    for member in root["workspace"]["members"]:
        tomllib.loads(read(f"{member}/Cargo.toml"))
    if root["profile"]["release"].get("panic") != "unwind":
        errors.append('profile.release.panic must be "unwind" (JNI panic containment)')
    tc = tomllib.loads(read("rust-toolchain.toml"))["toolchain"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", tc["channel"]):
        errors.append("rust-toolchain channel should be an exact version for reproducible builds")


def parse_compile_sdk(gradle: str):
    """Returns (major, minor) of compileSdk from app/build.gradle.kts, or (None, None)."""
    structured = re.search(r"release\((\d+)\)\s*\{\s*minorApiLevel\s*=\s*(\d+)", gradle)
    if structured:
        return int(structured.group(1)), int(structured.group(2))
    plain = re.search(r"^\s*compileSdk\s*=\s*(\d+)", gradle, re.M)
    if plain:
        major = int(plain.group(1))
        if major >= 37:
            errors.append(
                f"compileSdk = {major} is ambiguous: since API 37 sdkmanager publishes only minor-versioned "
                f"platforms. Use compileSdk {{ version = release({major}) {{ minorApiLevel = 0 }} }}"
            )
        return major, 0
    errors.append("compileSdk not found in app/build.gradle.kts")
    return None, None


def sdk_platform_dir(major: int, minor: int) -> str:
    """Name of the Android SDK platform package/directory (after 'platforms;').

    Verified rule: up to API 36 the base level is 'android-36' and only minor releases carry a suffix
    ('android-36.1'); from API 37 on every platform is minor-versioned, the base one is 'android-37.0'
    and plain 'android-37' does not exist in sdkmanager.
    """
    if major <= 36 and minor == 0:
        return f"android-{major}"
    return f"android-{major}.{minor}"


def workflow_jobs(text: str) -> dict[str, str]:
    """Splits the workflow text into job bodies (2-space-indented keys under `jobs:`)."""
    body = text.split("\njobs:\n", 1)[1]
    parts = re.split(r"\n  ([A-Za-z0-9_-]+):\n", "\n" + body)
    return {parts[i]: parts[i + 1] for i in range(1, len(parts) - 1, 2)}


def check_workflow_and_versions():
    wf = read(".github/workflows/android.yml")
    env = dict(re.findall(r'^  ([A-Z_]+): "([^"]+)"', wf, re.M))
    pins = {
        "RUST_TOOLCHAIN": tomllib.loads(read("rust-toolchain.toml"))["toolchain"]["channel"],
        "NDK_VERSION": (re.search(r'val rpxNdkVersion = "([^"]+)"', read("app/build.gradle.kts")) or [None, None])[1],
        "GRADLE_VERSION": (re.search(r"gradle-([\d.]+)-bin\.zip", read("gradle/wrapper/gradle-wrapper.properties")) or [None, None])[1],
    }
    gradle = read("app/build.gradle.kts")
    major, minor = parse_compile_sdk(gradle)
    if major is not None:
        pins["PLATFORM"] = sdk_platform_dir(major, minor)
        platform_package = f"platforms;{sdk_platform_dir(major, minor)}"
        if platform_package not in read("docs/VERSIONS.md"):
            errors.append(f"docs/VERSIONS.md must mention the SDK package '{platform_package}'")
        target = re.search(r"targetSdk = (\d+)", gradle)
        min_sdk = re.search(r"minSdk = (\d+)", gradle)
        if target and int(target.group(1)) > major:
            errors.append(f"targetSdk {target.group(1)} is higher than compileSdk {major}")
        if target and min_sdk and int(min_sdk.group(1)) > int(target.group(1)):
            errors.append("minSdk is higher than targetSdk")
    for key, expected in pins.items():
        if env.get(key) != expected:
            errors.append(f"workflow env {key}={env.get(key)!r} but the project pins {expected!r}")
    versions_doc = read("docs/VERSIONS.md")
    for key in ("CARGO_NDK_VERSION", "NDK_VERSION", "GRADLE_VERSION", "RUST_TOOLCHAIN"):
        if env.get(key) and env[key] not in versions_doc:
            errors.append(f"docs/VERSIONS.md does not mention {key}={env[key]}")

    jobs = workflow_jobs(wf)
    for name in ("bootstrap", "rust", "android"):
        if name not in jobs:
            errors.append(f"workflow job '{name}' is missing")
    if errors and not all(n in jobs for n in ("bootstrap", "rust", "android")):
        return
    for name in ("rust", "android"):
        if not re.search(r"^    needs: bootstrap\s*$", jobs[name], re.M):
            errors.append(f"job '{name}' must have 'needs: bootstrap'")
        if "actions/download-artifact" not in jobs[name]:
            errors.append(f"job '{name}' must download the bootstrap-files artifact")
        if "cargo generate-lockfile" in jobs[name] or "generate-wrapper" in jobs[name]:
            errors.append(f"job '{name}' must not generate Cargo.lock or the wrapper itself")
        if "check_project_consistency.py --strict" not in jobs[name]:
            errors.append(f"job '{name}' must run check_project_consistency.py --strict")
    boot = jobs["bootstrap"]
    upload = boot.find("name: bootstrap-files")
    verify = boot.find("./gradlew --version")
    if upload < 0 or verify < 0 or upload > verify:
        errors.append("bootstrap must upload bootstrap-files before the verification steps")
    up_block = boot[max(0, boot.rfind("- name: Upload bootstrap files")):upload + 20]
    if "always()" not in up_block:
        errors.append("bootstrap-files upload must use if: always()")
    for needed in ("Cargo.lock", "gradlew", "gradlew.bat", "gradle/wrapper/gradle-wrapper.jar"):
        if needed not in boot[upload:upload + 400]:
            errors.append(f"bootstrap-files artifact does not include {needed}")
    android = jobs["android"]
    for needed in ("Failed to find package", "--list_installed", 'platforms;${PLATFORM}'):
        if needed not in android:
            errors.append(f"android job must verify SDK installation (missing: {needed})")
    if "./gradlew :app:testDebugUnitTest :app:assembleDebug --stacktrace" not in android:
        errors.append("android job must build with ./gradlew :app:testDebugUnitTest :app:assembleDebug --stacktrace")
    if re.search(r"^\s*(?:run:\s*|script:\s*)?gradle\s+:", wf, re.M):
        errors.append("workflow runs the system 'gradle' for a build; use ./gradlew")
    if re.search(r"^\s+gradle-version:", android, re.M):
        errors.append("android job must not install Gradle via setup-gradle gradle-version")
    verify_script = read("scripts/verify-apk-permissions.sh")
    if re.search(r"aapt2\"?[^\n]*\|\|\s*true", verify_script):
        errors.append("verify-apk-permissions.sh must not ignore aapt2 failures with '|| true'")


def check_naming():
    """Product name, package and old-name hygiene (the old name may live only in docs/NAMING.md)."""
    gradle = read("app/build.gradle.kts")
    namespace = (re.search(r'namespace\s*=\s*"([^"]+)"', gradle) or [None, None])[1]
    app_id = (re.search(r'applicationId\s*=\s*"([^"]+)"', gradle) or [None, None])[1]
    if not namespace or not app_id:
        errors.append("namespace/applicationId not found in app/build.gradle.kts")
        return
    for locale in ("values", "values-ru"):
        name = re.search(r'name="app_name">([^<]*)<', read(f"app/src/main/res/{locale}/strings.xml"))
        if not name or name.group(1) != "Wolf Paint DZ":
            errors.append(f"app_name in {locale}/strings.xml must be 'Wolf Paint DZ'")
    if 'android:label="@string/app_name"' not in read("app/src/main/AndroidManifest.xml"):
        errors.append("manifest application label must be @string/app_name")
    if not re.search(r'rootProject\.name = "wolf-paint-dz"', read("settings.gradle.kts")):
        errors.append('settings.gradle.kts: rootProject.name must be "wolf-paint-dz"')
    pkg_dir = Path("app/src") / "main" / "kotlin" / namespace.replace(".", "/")
    if not (ROOT / pkg_dir).is_dir():
        errors.append(f"Kotlin sources are not in the namespace directory {pkg_dir}")
    for f in kotlin_sources():
        m = re.search(r"^package ([\w.]+)", Path(f).read_text(encoding="utf-8"), re.M)
        if not m or not m.group(1).startswith(namespace):
            errors.append(f"{os.path.relpath(f, ROOT)}: package {m.group(1) if m else '?'} is outside {namespace}")
    mangled = namespace.replace(".", "_")
    if f"Java_{mangled}_core_RustBridge_" not in read("crates/core-ffi/src/jni_bridge.rs"):
        errors.append(f"jni_bridge.rs exports do not use the namespace prefix Java_{mangled}_core_RustBridge_")
    expected_prefix = namespace.replace(".", "/") + "/core/RpxNativeException$"
    class_consts = re.findall(r'"([^"]*RpxNativeException\$\w+)"', read("crates/core-ffi/src/lib.rs"))
    if not class_consts:
        errors.append("no exception class constants found in crates/core-ffi/src/lib.rs")
    for const in class_consts:
        if not const.startswith(expected_prefix):
            errors.append(f"core-ffi exception class '{const}' does not start with '{expected_prefix}'")
    if namespace.replace(".", "/") not in read("app/proguard-rules.pro").replace(".", "/"):
        errors.append("app/proguard-rules.pro does not reference the namespace")
    # Built from parts so that this file itself does not contain the old name.
    old = re.compile("rpx" + "[ _-]?" + "paint", re.I)
    skip_dirs = {".git", "build", "target", ".gradle", "__pycache__"}
    for path in ROOT.rglob("*"):
        if not path.is_file() or skip_dirs & set(path.relative_to(ROOT).parts):
            continue
        rel = path.relative_to(ROOT).as_posix()
        if rel == "docs/NAMING.md":
            continue
        if old.search(rel):
            errors.append(f"old product name in file path: {rel}")
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if old.search(line):
                errors.append(f"{rel}:{n}: old product name: {line.strip()[:90]}")


def check_hardening():
    """Stage 2.0: release/R8/16 KB checks, supply chain policy, action pins, required docs."""
    wf = read(".github/workflows/android.yml")
    jobs = workflow_jobs(wf)
    for needed in ("supply-chain",):
        if needed not in jobs:
            errors.append(f"workflow job '{needed}' is missing")
            return
    android = jobs["android"]
    for needed, why in (
        ("./gradlew :app:assembleRelease", "unsigned release build with R8"),
        ("app-release-unsigned.apk", "release APK checks"),
        ("zipalign", "16 KB zip alignment of the release APK"),
        ("-c -P 16", "zipalign must check with -P 16"),
        ("scripts/check_apk_native.py", "native library / ELF / JNI export check"),
        ("scripts/check_r8_keep.py", "R8 keep check"),
        ("scripts/verify-apk-permissions.sh", "aapt2 permission check"),
    ):
        if needed not in android:
            errors.append(f"android job is missing '{needed}' ({why})")
    if android.count("scripts/verify-apk-permissions.sh") < 2:
        errors.append("android job must run verify-apk-permissions.sh for both the debug and the release APK")
    if android.count("scripts/check_apk_native.py") < 2:
        errors.append("android job must run check_apk_native.py for both the debug and the release APK")

    uses_gradle = re.findall(r"uses: gradle/actions/setup-gradle@\S+[^\n]*\n(?:\s+with:\n(?:\s+[\w-]+:[^\n]*\n)*)?", wf)
    if not uses_gradle:
        errors.append("no setup-gradle step found")
    for block in uses_gradle:
        if "cache-provider: basic" not in block:
            errors.append("every setup-gradle step must set 'cache-provider: basic' (open-source cache provider)")

    deny_job = jobs["supply-chain"]
    for needed in ("cargo deny check bans licenses sources", "cargo metadata --locked", "scripts/check_lock_policy.py",
                   "cargo install cargo-deny --version"):
        if needed not in deny_job:
            errors.append(f"supply-chain job is missing '{needed}'")
    adv = deny_job[deny_job.find("check advisories"):] if "check advisories" in deny_job else ""
    if "continue-on-error: true" not in adv:
        errors.append("the advisories check must be explicitly informational (continue-on-error: true) and the only such step")
    if wf.count("continue-on-error") != 1:
        errors.append("continue-on-error may only be used once (the informational advisories step)")
    if "needs: bootstrap" not in deny_job:
        errors.append("supply-chain job must depend on bootstrap")
    if "verify_action_pins.py --online" not in jobs["bootstrap"]:
        errors.append("bootstrap must run verify_action_pins.py --online")

    pin_errors, pinned = verify_action_pins.check_structure(ROOT)
    errors.extend(pin_errors)
    if len(pinned) < 2:
        errors.append("expected at least two pinned third-party actions (setup-gradle, android-emulator-runner)")

    deny = tomllib.loads(read("deny.toml"))
    if deny["sources"].get("unknown-registry") != "deny" or deny["sources"].get("unknown-git") != "deny":
        errors.append("deny.toml: unknown-registry and unknown-git must be 'deny'")
    if not isinstance(deny["bans"].get("deny"), list) or len(deny["bans"]["deny"]) < 10:
        errors.append("deny.toml: [bans] deny list is missing (a key written after a sub-table header such as "
                      "[bans.std-replacements] belongs to that sub-table; keep sub-tables after the plain keys)")
    if set(deny["bans"].get("std-replacements", {})) - {"level", "scope", "ignore", "ignore-rust-version", "rust-version"}:
        errors.append("deny.toml: unexpected keys inside [bans.std-replacements]")
    if deny["bans"].get("std-replacements", {}).get("level") not in ("warn", "deny"):
        errors.append("deny.toml: [bans.std-replacements] level must be set explicitly (cargo-deny 0.20 defaults to deny)")
    if not deny["graph"].get("targets"):
        errors.append("deny.toml: [graph] targets must list the shipped Android targets")
    cargo_deny_env = re.search(r'CARGO_DENY_VERSION: "([^"]+)"', wf)
    if not cargo_deny_env or cargo_deny_env.group(1) not in read("docs/VERSIONS.md"):
        errors.append("docs/VERSIONS.md must mention the CARGO_DENY_VERSION used by the workflow")
    for sha, tag in sorted(set(re.findall(r"@([0-9a-f]{40}) # (v[\d.]+)", wf))):
        if sha not in read("docs/VERSIONS.md") or tag not in read("docs/VERSIONS.md"):
            errors.append(f"docs/VERSIONS.md must list the pinned action {tag} ({sha})")

    if (ROOT / "Cargo.lock").is_file():
        lock_errors, _ = check_lock_policy.check(read("Cargo.lock"), read("deny.toml"))
        errors.extend(lock_errors)
    ci_doc = read("docs/CI.md") if (ROOT / "docs/CI.md").is_file() else ""
    for phrase in ("A deny-list is not a proof", "What the offline checks do and do not prove", "fixed list of well-known"):
        if phrase not in ci_doc:
            errors.append(f"docs/CI.md must keep the statement '{phrase}' (limits of the offline checks)")
    if "deny-list" not in read("deny.toml") and "DENY-LIST" not in read("deny.toml"):
        errors.append("deny.toml header must say that its ban list is a deny-list, not a proof")
    for doc in ("docs/CI.md", "docs/DEVICES.md", "docs/VERIFICATION.md", "docs/STAGE1_AUDIT.md", "docs/STAGE2_PLAN.md"):
        if not (ROOT / doc).is_file():
            errors.append(f"{doc} is missing")


def check_reproducibility_files():
    missing = [p for p in ["gradlew", "gradlew.bat", "gradle/wrapper/gradle-wrapper.jar", "Cargo.lock"] if not (ROOT / p).exists()]
    props = read("gradle/wrapper/gradle-wrapper.properties")
    ver = re.search(r"gradle-([\d.]+)-bin\.zip", props)
    libs = tomllib.loads(read("gradle/libs.versions.toml"))["versions"]
    print(f"gradle wrapper version: {ver.group(1) if ver else '?'}, AGP: {libs['agp']}, Kotlin: {libs['kotlin']}")
    if missing:
        (errors if "--strict" in sys.argv else warnings).append(
            "not committed yet (generate with scripts/generate-wrapper.sh, cargo generate-lockfile): " + ", ".join(missing)
        )


for check in (check_xml, check_catalog, check_resources, check_exceptions, check_offline, check_cargo,
              check_workflow_and_versions, check_naming, check_hardening, check_reproducibility_files):
    check()

for w in warnings:
    print("WARNING:", w)
if errors:
    print("FAILED:")
    print("\n".join(" - " + e for e in errors))
    sys.exit(1)
print("project consistency: OK")
