# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

### Development Build
```bash
cargo build
```

### Release Build
```bash
cargo build --release
```

### Build with FFmpeg support
```bash
cargo build --features ffmpeg
```

### Build All Components (using the convenience script)
```bash
# Development build
./build-launcher.sh

# Release build
./build-launcher.sh --release
```

This builds the main enigmatick binary, proxy, tasks, and launcher with embedded binaries.

## Database Management

### Run Migrations
```bash
cargo run -- migrate
```

### Database Features
- Default: PostgreSQL (`pg` feature)
- Alternative: SQLite (`sqlite` feature) 
- Never enable both features simultaneously

### Configuration Files
- PostgreSQL: `diesel-pg.toml` with `migrations.pg/` directory
- SQLite: `diesel-sqlite.toml` with `migrations.sqlite/` directory

## Testing

### Run Tests
```bash
cargo test
```

### Run Tests with FFmpeg Features
```bash
cargo test --features ffmpeg
```

## Running the Application

### Initialize Directory Structure
```bash
cargo run -- init
```

### Generate Configuration Template
```bash
cargo run -- template
```

### Start Server
```bash
cargo run -- app
```

### Other Commands
```bash
cargo run -- cache <command>           # Manage cached media
cargo run -- system-user              # Create system user
cargo run -- instances <command>      # Manage federated instances
cargo run -- send <command>           # Send activities
cargo run -- muted-terms <command>    # Manage muted terms
```

## Code Architecture

### Core Modules

**`src/lib.rs`**: Main library with global configuration, traits, and ActivityPub implementations
- Contains all environment variable definitions and lazy static configs
- Defines key traits: `GetWebfinger`, `GetHashtags`, `HasReplies`, `FetchReplies`, `LoadEphemeral`
- Handles conditional compilation for PostgreSQL vs SQLite

**`src/server/`**: Axum-based web server
- `mod.rs`: Server setup, routing, and state management
- `routes/`: HTTP endpoints organized by functionality
  - `inbox/`: ActivityPub inbox handling (federation)
  - `outbox/`: ActivityPub outbox handling  
  - `admin.rs`: Administrative functions
  - `authentication.rs`: User auth
  - `client.rs`: Frontend serving
  - `encryption.rs`: E2E encryption
  - `user.rs`: User profiles and data

**`src/models/`**: Database models and operations
- Each file corresponds to a database table/entity
- Diesel ORM integration with separate schemas for PostgreSQL/SQLite
- `activities.rs`: ActivityPub activities
- `actors.rs`: Users and remote actors
- `objects.rs`: Posts, articles, notes
- `follows.rs`: Follow relationships

**`src/runner/`**: Background task processors
- `note.rs`: Process incoming notes/posts
- `announce.rs`: Handle boosts/reblogs
- `user.rs`: User-related background tasks
- `encrypted.rs`: Handle encrypted messages

**`src/retriever.rs`**: Federation and remote data fetching
**`src/signing.rs`**: ActivityPub HTTP signatures
**`src/webfinger.rs`**: WebFinger protocol implementation

### Multi-Binary Workspace

The project uses a Cargo workspace with multiple binaries:
- **Main binary**: `src/bin/enigmatick/mod.rs` - Core application
- **Proxy**: `proxy/` - ACME/TLS proxy for Let's Encrypt
- **Tasks**: `tasks/` - Background task processor
- **Launcher**: `launcher/` - Unified launcher that embeds all components

### Database Schema Management

- Uses Diesel ORM with migrations
- Dual database support (PostgreSQL/SQLite) with feature flags
- Schema files: `src/schema-pg.rs` and `src/schema-sqlite.rs`
- Migration directories: `migrations.pg/` and `migrations.sqlite/`

### ActivityPub Federation

Implements ActivityPub protocol for federation with Mastodon, Pleroma, etc.:
- HTTP signature verification for authenticated requests
- Inbox/outbox pattern for activity distribution
- Object fetching and caching from remote instances
- WebFinger for actor discovery

### Configuration

Environment variables (required):
- `SERVER_NAME`: Public domain name
- `DATABASE_URL`: Database connection string
- `SYSTEM_USER`: System account username
- `MEDIA_DIR`: Media storage directory
- Instance metadata: `INSTANCE_TITLE`, `INSTANCE_DESCRIPTION`, `INSTANCE_CONTACT`
- Registration settings: `REGISTRATION_ENABLED`, `REGISTRATION_APPROVAL_REQUIRED`

Optional:
- `ACME_PROXY=true`: Enable built-in Let's Encrypt proxy
- `ACME_PORT`: TLS proxy port (default: 443)
- `SERVER_ADDRESS`: Backend server address (default: 0.0.0.0:8001)

### Frontend Integration

- Bundled Svelte frontend in `client/` directory
- Static files served via Axum
- All client assets are embedded in the binary using `rust-embed`