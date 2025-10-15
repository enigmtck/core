# Enigmatick Release Process

## For Users

### Installation

**Option 1: Install script (Recommended)**
```bash
curl -sSL https://gitlab.com/enigmatick/enigmatick-core/-/raw/main/install.sh | sh
```

**Option 2: Manual download**
1. Go to [Releases](https://gitlab.com/enigmatick/enigmatick-core/-/releases)
2. Download the binary for your platform
3. Make it executable: `chmod +x enigmatick-*`
4. Move to your PATH: `mv enigmatick-* ~/.local/bin/enigmatick`

**Option 3: cargo-binstall**
```bash
cargo binstall enigmatick
```

### Running

```bash
enigmatick server    # Start the server (app + proxy + tasks)
enigmatick migrate   # Run database migrations
enigmatick init      # Initialize directories
enigmatick --help    # See all commands
```

## For Maintainers

### Creating a Release

1. **Update version** in:
   - `Cargo.toml` (workspace root)
   - `launcher/Cargo.toml`

2. **Update CHANGELOG.md** with release notes

3. **Commit and tag**:
   ```bash
   git add .
   git commit -m "Release v0.4.2"
   git tag v0.4.2
   git push origin main --tags
   ```

4. **GitLab CI will automatically**:
   - Build binaries for supported platforms
   - Create a GitLab Release
   - Attach binaries to the release

5. **Verify the release**:
   - Check [Releases page](https://gitlab.com/enigmatick/enigmatick-core/-/releases)
   - Test installation script
   - Announce on social media

### Building Locally

To test the launcher build locally:

```bash
./build-launcher.sh --release
./launcher/target/release/enigmatick --version
```

### Platform Support

Currently building for:
- ✅ Linux x86_64 (primary)
- ⏳ macOS x86_64 (requires macOS runner)
- ⏳ macOS ARM64 (requires macOS runner)
- ⏳ Linux ARM64 (can add cross-compilation)

To add more platforms, update `.gitlab-ci.yml`.
