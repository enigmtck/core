# Enigmatick Core

This is the core component of the Enigmatick social networking platform. Enigmatick implements functionality consistent with ActivityPub services; when everything is working properly, users may follow and interact with people hosted on services like Mastodon, Pleroma, Pixelfed, and others.

## Installation

The simplest way to install Enigmatick is to use `cargo`:

```
cargo install enigmatick
```

This will install a `postgres` server. To install the SQLite-based server, you'll need to use `cargo install enigmatick --no-default-features -F sqlite`. The `sqlite` and `pg` features are mutually exclusive. If both are enabled, the `pg` components will take precedence.

_Currently the `sqlite` server is non-functional, but I'll hopefully have time to integrate it again (with JSONB functionality) in the near future._

To configure the server, use these commands:

```
enigmatick init
```

This will create the directory structure for the Enigmatick server in the current folder.

```
enigmatick template
```

This will copy in the bundled `.env.template` file to use to configure the server. Copy this to `.env` and modify it according to your needs.

```
enigmatick migrate
```

If using SQLite, this will create the database. In both the SQLite and PostgreSQL configurations, this will also set up the tables necessary to support Enigmatick.

## Operation

`enigmatick server` will start the Enigmatick server from the current folder using the configuration you've set in `.env`. You can then use your browser to connect to the configured port.

Currently, you'll need to use a separate reverse proxy to handle the TLS necessary for ActivityPub to work properly. I plan to incorporate that configuration directly in to Enigmatick eventually.

## Full Example

This assumes that a PostgreSQL server is running and available at 192.168.1.100

```
> enigmatick
Enigmatick: A federated communication platform server.

Usage: enigmatick <COMMAND>

Commands:
  init         Initialize the necessary folder structure (e.g., for media). This should be run once before starting the server for the first time
  template     Generate a template .env file named '.env.template'. Copy this to '.env' and fill in your configuration values
  migrate      Run database migrations to set up or update the database schema. This is necessary before starting the server and after updates
  cache        Manage cached media files. Use subcommands like 'prune' or 'delete'
  system-user  Create or ensure the system user exists in the database. The system user is used for server-to-server activities and internal tasks
  server       Start the Enigmatick web server and background task runners
  instances    Manage known instances (other federated servers). Allows listing, blocking, unblocking, and viewing details of instances
  send         Send various types of activities
  muted-terms  Manage muted terms for users. Allows listing, adding, removing, and clearing muted terms
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
-- Edit as needed - particularly the ROCKET_DATABASES and DATABASE_URL items.
ROCKET_DATABASES='{enigmatick={url="postgres://enigmatick:enigmatick@192.168.1.100/enigmatick"}}'
DATABASE_URL=postgres://enigmatick:enigmatick@192.168.1.100/enigmatick

createuser -h 192.168.1.100 -U yourpsqluser -W -lP enigmatick
createdb -h 192.168.1.100 -U yourpsqluser -W -O enigmatick enigmatick
enigmatick migrate
RUST_LOG=debug enigmatick server
```

