# Enigmatick Core

This is the core component of the Enigmatick social networking platform. Enigmatick implements functionality consistent with ActivityPub services; when everything is working properly, users may follow and interact with people hosted on services like Mastodon, Pleroma, Pixelfed, and others.

## Installation

### Distribution Dependencies

Enigmatick relies on a number of libraries provided by the host system. For Debian, you can install everything required using:

```
sudo apt-get install -y --no-install-recommends \
    libpq-dev libsqlite3-dev ssl-cert ca-certificates \
    curl apt-transport-https lsb-release file git-core \
    build-essential libssl-dev libssl3 libgexiv2-dev \
    cmake clang ffmpeg nasm pkg-config postgresql
```

### Rust Environment

Enigmatick is written primarily in Rust. The backend service is 100% Rust, and the frontend application is written in Svelte. The frontend application is compiled statically and bundled with the Rust application, so you don't need to worry about running that service separately.

Visit https://rustup.rs/ to install Rust on your server.

### Enigmatick Installation

The simplest way to install Enigmatick is to use `cargo`:

```
cargo install enigmatick
```

## Setup and Configuration

### Database Setup

Enigmatick requires the installation of a PostgreSQL server. These commands may need to be tweaked to set up the user and password properly. You may also need to adjust `pg_hba.conf` in the `/etc/postgresql` subdirectory for your database version to allow local TCP connections.

```
sudo su - postgres
createuser -l dbuser
psql postgres
  > ALTER USER dbuser WITH PASSWORD 'dbpassword';
  > \q
createdb -O dbuser enigmatick
```

### Application Setup

To configure the server in the current directory, use these commands:

```
enigmatick init
```

This will create the directory structure for the Enigmatick server in the current folder.

```
enigmatick template
```

This will copy the bundled `.env.template` file into the current directory. Copy this to `.env` and modify it according to your needs.

### Configuration

The `.env` file contains all the necessary configuration for your Enigmatick instance. Below are some of the key variables you will need to set.

#### Server and Network

*   `SERVER_NAME`: Your public-facing domain name (e.g., `enigmatick.example.com`). This is crucial for federation to work correctly.
*   `ACME_PROXY`: Set to `true` to enable the built-in TLS proxy, which automatically obtains and renews a Let's Encrypt certificate. If enabled, your server must be reachable from the public internet on port 443.
*   `ACME_PORT`: The port the ACME TLS proxy will listen on. Defaults to `443`.
*   `ACME_EMAILS`: The email addresses to use for Let's Encrypt registration.
*   `SERVER_ADDRESS`: The local IP address and port for the backend server to listen on. Defaults to `127.0.0.1:8000` for use with the built-in proxy.

#### Database

*   `DATABASE_URL`: The connection string for the PostgreSQL database.

#### Instance Metadata

These variables control how your instance is presented to the fediverse and to users.

*   `INSTANCE_TITLE`: The name of your instance.
*   `INSTANCE_DESCRIPTION`: A short description of your instance.
*   `INSTANCE_CONTACT`: An email address for the instance administrator.
*   `SYSTEM_USER`: A dedicated user for server-to-server activities. Defaults to `system`.
*   `MEDIA_DIR`: The directory for storing uploaded media, avatars, and other assets.

#### Registration

*   `REGISTRATION_ENABLED`: Set to `true` to allow new user sign-ups.
*   `REGISTRATION_APPROVAL_REQUIRED`: If `true`, new user accounts must be approved by an administrator.
*   `REGISTRATION_MESSAGE`: A message to display on the registration page.

## Operation

### Database Migrations

Before starting the server for the first time, and after any updates, you must run database migrations. This sets up and maintains the database schema.

```
enigmatick migrate
```

### Running the Server

`enigmatick server` will start the Enigmatick server from the current folder using the configuration you've set in `.env`.

Enigmatick includes a built-in reverse proxy that can automatically handle TLS using Let's Encrypt. To enable this, set `ACME_PROXY=true` in your `.env` file. Your server must be accessible from the public internet on the `ACME_PORT` (usually 443) for certificate validation to succeed.

If you prefer to use your own reverse proxy (like nginx or Caddy), set `ACME_PROXY=false` and configure your proxy to forward requests to the backend service on `SERVER_ADDRESS`.

## Full Example

This assumes that a PostgreSQL server is running and available at 192.168.1.100

```
> enigmatick
A federated communication platform server

Usage: enigmatick <COMMAND>

Commands:
  init         Initialize folder structure for media files
  template     Generate .env.template file
  migrate      Run database migrations
  cache        Manage cached media files
  system-user  Create or ensure system user exists
  server       Start the web server and background tasks
  instances    Manage federated instances
  send         Send various activities
  muted-terms  Manage user muted terms
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
  
> mkdir server
> cd server
> enigmatick init
creating folder structure...
complete.

> enigmatick template
> cp .env.template .env
> vi .env
-- Edit as needed. See the Configuration section above for details on key variables.
-- For this example, you would at least set the following (ensure 'dbpassword'
-- matches the password you set when running `createuser`):
SERVER_NAME=your.domain.name
ACME_EMAILS='["your@email.com"]'
DATABASE_URL=postgres://dbuser:dbpassword@192.168.1.100/enigmatick
SERVER_ADDRESS=127.0.0.1:8000
ACME_PORT=443

createuser -h 192.168.1.100 -U yourpsqluser -W -lP dbuser
> Password: dbpassword

createdb -h 192.168.1.100 -U yourpsqluser -W -O dbuser enigmatick
enigmatick migrate
RUST_LOG=debug enigmatick server
```

