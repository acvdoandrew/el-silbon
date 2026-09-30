# Releasing a test build

1. Push `main`, then build the Windows executable on GitHub's Windows
   runner: `gh workflow run windows.yml`, wait for it
   (`gh run watch`), and download it: `gh run download <run id> -n
   el_silbon-windows-x86_64 -D dist/windows`.
2. Build Linux here: `cargo build --release --locked`.
3. Package each with `tools/package.sh EXE OS VERSION` (into `target/dist/`): the executable next to
   a copy of `assets/` (the game loads `assets/` beside the executable)
   and `docs/PLAYING.txt`.
4. `gh release create <tag> --prerelease --title ... --notes ... <zips>`.

On a Windows machine with the MSVC toolchain, one command builds and
packages the Windows zip (Git Bash has no `zip` for `package.sh`):
`powershell -File tools\package.ps1 0.1.0-test.N` (add `-NoBuild` to
package `target\release` as it is). The zip, about 110 MB, lands in
`target\dist\` and can be attached to the release or sent directly
(Discord takes it). Everyone in a session needs the same build.

The repository is public by the user's choice (2026-09-30), so its
releases are too.

The whistle recording in `assets/audio/source/` is third-party and not
cleared for public release (see `assets/SOURCES.md`); fine for private
test builds.
