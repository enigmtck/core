# Enigmatick Core

This is the core component of the Enigmatick social networking platform. Enigmatick implements functionality consistent with ActivityPub services; when everything is working properly, users may follow and interact with people hosted on services like Mastodon, Pleroma, Pixelfed, and others.

## Installation

The simplest way to install Enigmatick is to use `cargo`:

```
cargo install enigmatick
```

This will install a `sqlite` server. To install the PostgreSQL-based server, you'll need to use `cargo install enigmatick --no-default-features -F pg`. The `sqlite` and `pg` features are mutually exclusive. If both are enabled, the `pg` components will take precedence.

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
