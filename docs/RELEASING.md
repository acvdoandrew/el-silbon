# Releasing a test build

1. Push `main`, then build the Windows executable on GitHub's Windows
   runner: `gh workflow run windows.yml`, wait for it
   (`gh run watch`), and download it: `gh run download <run id> -n
   el_silbon-windows-x86_64 -D dist/windows`.
2. Build Linux here: `cargo build --release --locked`.
3. Package each as `el_silbon-<version>-<os>.zip`: the executable next to
   a copy of `assets/` (the game loads `assets/` beside the executable)
   and `docs/PLAYING.txt`.
4. `gh release create <tag> --prerelease --title ... --notes ... <zips>`.

The repository is private, so its releases are too: testers need access
to the repository, or the zip shared directly.

The whistle recording in `assets/audio/source/` is third-party and not
cleared for public release (see `assets/SOURCES.md`); fine for private
test builds.
