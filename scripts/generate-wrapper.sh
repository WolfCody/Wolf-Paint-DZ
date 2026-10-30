#!/usr/bin/env bash
# Generates the official Gradle Wrapper (gradlew, gradlew.bat, gradle-wrapper.jar) for the Gradle
# version pinned in gradle/wrapper/gradle-wrapper.properties. Linux/macOS/Git-Bash.
#
# Needs any installed Gradle (>= 8) and Java, plus network access to services.gradle.org.
# Runs in an empty temp directory, so the Android SDK is not required.
#
# Two stages, because the wrapper JAR is copied from the Gradle that RUNS the `wrapper` task:
#   1. `gradle wrapper` with the installed Gradle writes the scripts and sets the target version
#      (the JAR may still come from an older installed Gradle).
#   2. `./gradlew wrapper` runs the pinned Gradle itself and regenerates the JAR from it.
# Finally `./gradlew --version` must report exactly the pinned version, otherwise this fails.
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "FAIL: $*" >&2; exit 1; }

version="$(sed -n 's/.*gradle-\([0-9][0-9.]*\)-bin\.zip.*/\1/p' gradle/wrapper/gradle-wrapper.properties | head -n1)"
[ -n "$version" ] || fail "cannot read the Gradle version from gradle/wrapper/gradle-wrapper.properties"
command -v gradle > /dev/null 2>&1 || fail "no 'gradle' on PATH (any Gradle >= 8 works for stage 1)"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
: > "$tmp/settings.gradle"

echo "== stage 1: gradle wrapper --gradle-version $version"
(cd "$tmp" && gradle wrapper --gradle-version "$version" --distribution-type bin)
[ -f "$tmp/gradlew" ] && [ -f "$tmp/gradle/wrapper/gradle-wrapper.jar" ] || fail "stage 1 did not produce the wrapper"

echo "== stage 2: ./gradlew wrapper (regenerates the JAR with Gradle $version)"
(cd "$tmp" && chmod +x gradlew && ./gradlew wrapper --gradle-version "$version" --distribution-type bin)

echo "== verification: ./gradlew --version"
reported="$(cd "$tmp" && ./gradlew --version)"
printf '%s\n' "$reported"
printf '%s\n' "$reported" | grep -Eq "^Gradle ${version//./\\.}[[:space:]]*$" \
  || fail "generated wrapper does not run Gradle $version"
grep -q "gradle-${version}-bin.zip" "$tmp/gradle/wrapper/gradle-wrapper.properties" \
  || fail "generated gradle-wrapper.properties does not point to Gradle $version"

cp "$tmp/gradlew" "$tmp/gradlew.bat" ./
mkdir -p gradle/wrapper
cp "$tmp/gradle/wrapper/gradle-wrapper.jar" gradle/wrapper/
chmod +x gradlew

jar_sha="$( (sha256sum gradle/wrapper/gradle-wrapper.jar 2>/dev/null || shasum -a 256 gradle/wrapper/gradle-wrapper.jar) | cut -d' ' -f1)"
echo "Wrapper for Gradle $version written."
echo "gradle-wrapper.jar sha256: $jar_sha"
echo "(compare it with the 'Wrapper JAR Checksum' of Gradle $version on https://gradle.org/release-checksums/)"
echo "Commit gradlew, gradlew.bat, gradle/wrapper/gradle-wrapper.jar; on Git: git update-index --chmod=+x gradlew"
