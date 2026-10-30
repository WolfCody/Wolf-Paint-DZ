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
    kotlin = read("app/src/main/kotlin/com/rpxpaint/app/core/RpxNativeException.kt")
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
    compile_sdk = re.search(r"compileSdk = (\d+)", read("app/build.gradle.kts"))
    if compile_sdk:
        pins["PLATFORM"] = f"android-{compile_sdk.group(1)}"
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
    if "./gradlew :app:testDebugUnitTest :app:assembleDebug --stacktrace" not in android:
        errors.append("android job must build with ./gradlew :app:testDebugUnitTest :app:assembleDebug --stacktrace")
    if re.search(r"^\s*(?:run:\s*|script:\s*)?gradle\s+:", wf, re.M):
        errors.append("workflow runs the system 'gradle' for a build; use ./gradlew")
    if re.search(r"^\s+gradle-version:", android, re.M):
        errors.append("android job must not install Gradle via setup-gradle gradle-version")
    verify_script = read("scripts/verify-apk-permissions.sh")
    if re.search(r"aapt2\"?[^\n]*\|\|\s*true", verify_script):
        errors.append("verify-apk-permissions.sh must not ignore aapt2 failures with '|| true'")


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
              check_workflow_and_versions, check_reproducibility_files):
    check()

for w in warnings:
    print("WARNING:", w)
if errors:
    print("FAILED:")
    print("\n".join(" - " + e for e in errors))
    sys.exit(1)
print("project consistency: OK")
