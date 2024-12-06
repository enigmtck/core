# Enigmatick End-to-End Encryption Design

Enigmatick is an ActivityPub server designed as a testbed for implementing end-to-end encryption using the ActivityStreams vocabulary (to the extent reasonable). Some liberties have been taken in the current implementation to enable a server to identify and appropriately handle encrypted notes (`EncryptedNote` type) separately from plaintext notes (`Note` type). I've tried to keep those idiosyncrasies to a minimum and I'm open to adjusting the implementation to better align with the standard. In all cases where a change might make sense, please open an Issue on this repository.

This document is a stream-of-consciousness dump of the technical details that have been swirling around in my head. I'll evolve it over time to be increasingly structured and useful.

## Overview

I've integrated the [`vodozemac`](https://github.com/matrix-org/vodozemac) Rust library as the primary mechanism used to provide asymmetric encryption between ActivityPub participants. Design decisions have been made to accommodate the way that library operates.

Additionally, symmetric encryption is handled by the [`orion`](https://github.com/orion-rs/orion) cryptographic suite. As I've built out this service, this library has somewhat lagged other options and is not as thoroughly vetted as I'd like; I may change this. Orion provides encryption of data stored on the server to facilitate client (browser) access and is not critical to the fundamental design of the cryptographic system described here.

## ActivityPub Structures

These are the fundamental communication mechanisms used to faclitate encrypted communication between actors in Enigmatick.

### Server-to-Server

Recipient `OlmOneTimeKey` and `OlmIdentityKey` retrieval by initiating sender.

```json
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Collection",
  "id": "https://enigmatick.social/user/clark/keys",
  "totalItems": 2,
  "items": [
    {
      "type": "OlmOneTimeKey",
      "id": "https://enigmatick.social/instruments/da267575-3787-4df2-ac11-e734f9f32d38",
      "content": "GVGzbKaznnKzgV5+/xRDfdx2lbMP0qBfcTaSV1+BATY"
    },
    {
      "type": "OlmIdentityKey",
      "id": "https://enigmatick.social/user/clark#identity-key",
      "content": "7jImYabiufIUOE4QjCfabJT/h1OhoMLztxqzEBqyn0w"
    }
  ]
}
```

Initial message to recipient initiating the session.

```json
{
    "@context": ["https://www.w3.org/ns/activitystreams"],
    "type": "Create",
    "to": ["https://enigmatick.social/user/clark"],
    "actor": "https://enigmatick.social/user/jdt",
    "id": "https://enigmatick.social/activities/b680adf8-9408-482d-9387-6dfa7904df67",
    "object": {
        "cc": [],
        "id": "https://enigmatick.social/objects/88cb00b5-5fec-49a2-a895-bca156ece754",
        "to": [
            "https://enigmatick.social/user/clark"
        ],
        "conversation": "https://enigmatick.social/conversations/195f61c3-7098-4736-a837-1fab29e42699",
        "tag": [
            {
                "href": "https://enigmatick.social/user/clark",
                "name": "@clark@enigmatick.social",
                "type": "Mention"
            }
        ],
        "type": "EncryptedNote",
        "content": "{\"type\":0,\"body\":\"AwogZNVPkw0ZXxKnhDU0Tjf4NWX/2OvFTSx2IWxS1S1VK3gSINVvl3MHxmTBwyGVO+Bc8QqAP
ARVDTsPoCuRrrcEn7hxGiAFhZIDySEb5XYDxjEtbkueCuMfNs/7plG3FBweZdUfHiJnBAogDJQPFOjpBU81Hk4xildg7kNyTwn5Ntotk22UOEJ
4vUg8QACIghqtjcoYnNKMk5unzO3qa0ckq/WihX18uwwbGN44Wm/8Kk6xgFQ7Cbl5q1DbNcPm6QlFnaJtyMveLLa7EzAJuUg\"}",
        "published": "2024-11-24T23:23:14.252Z",
        "attachment": [],
        "attributedTo": "https://enigmatick.social/user/jdt"
    },
    "instrument": [{
        "id": "https://enigmatick.social/instruments/112b6598-a23a-48cd-aeb0-721ab363413d",
        "type": "OlmIdentityKey",
        "content": "BYWSA8khG+V2A8YxLW5LngrjHzbP+6ZRtxQcHmXVHx4"
    }]
}
```

Additional messages (in either direction) after the initial.

```json
{
    "@context": ["https://www.w3.org/ns/activitystreams"],
    "type": "Create",
    "to": ["https://enigmatick.social/user/jdt"],
    "actor": "https://enigmatick.social/user/clark",
    "id": "https://enigmatick.social/activities/1bb8c2c6-1181-401f-b17d-c7b849b63b3d",
    "object": {
        "cc": [],
        "id": "https://enigmatick.social/objects/0e9d572f-7e6f-4201-83a3-2d8137a90e62",
        "to": [
            "https://enigmatick.social/user/jdt"
        ],
        "conversation": "https://enigmatick.social/conversations/195f61c3-7098-4736-a837-1fab29e42699",
        "tag": [
            {
                "href": "https://enigmatick.social/user/jdt",
                "name": "@jdt@enigmatick.social",
                "type": "Mention"
            }
        ],
        "type": "EncryptedNote",
        "content": "{\"type\":0,\"body\":\"AwogZNVPkw0ZXxKnhDU0Tjf4NWX/2OvFTSx2IWxS1S1VK3gSINVvl3MHxmTBwyGVO+Bc8QqAP
ARVDTsPoCuRrrcEn7hxGiAFhZIDySEb5XYDxjEtbkueCuMfNs/7plG3FBweZdUfHiJnBAogDJQPFOjpBU81Hk4xildg7kNyTwn5Ntotk22UOEJ
vUg8QACIghqtjcoYnNKMk5unzO3qa0ckq/WihX18uwwbGN44Wm/8Kk6xgFQ7Cbl5q1DbNcPm6QlFnaJtyMveLLa7EzAJuUg\"}",
        "published": "2024-11-24T23:23:14.252Z",
        "attachment": [],
        "attributedTo": "https://enigmatick.social/user/jdt"
    }
}
```

Encrypted sessions are tied to conversations. As such, there's no need to provide any link to a session or anything like that when sending messages between servers. The receiving server will know which session to retrieve based on the `to` and `conversation` field combined with the `EncryptedNote` type.

## System Design

The overall system is comprised of these components:

1. Individual Olm accounts created for the sender and the receiver. In Enigmatick, these exist as symmetrically encrypted blobs stored in the `core` database(s).
2. New `Instrument` types for: `OlmIdentityKey` and `OlmOneTimeKey`. There are also other new instruments defined (e.g., `OlmSession`, `VaultItem`, etc.) that are specific to the Enigmatick client implementation and are not specifically relevant to the ActivityPub exchanges. While Enigmatick stores and manages those as symmetrically-encrypted `Instrument` objects, it's plausible that a fat client, for example, would handle the associated requirements differently.
3. A new `/keys` endpoint for all encryption-enabled `Actor` objects. This endpoint is used to deliver `OlmIdentityKey` and `OlmOneTimeKey` instruments to message-sending actors.
4. API methods and associated storage facilities to manage `OlmOneTimeKey` records generated by the client (the browser in the case of Enigmatick).
4. A new object type, `EncryptedNote`. This object currently mirrors all of the characteristics of `Note` with the only difference being that the `content` field is intended to be unreadable without the aid of an instrument.

### Account Creation

An Olm account is created when an Enigmatick user is created. This account is maintained within Enigmatick as a part of the `Actor` object for local actors. It is a private field (symmetrically encrypted by the client) and should not be exposed as a general ActivityPub attribute.

```sql
enigmatick=> select ek_olm_pickled_account, ek_olm_identity_key from actors where ek_username='clark';
-[ RECORD 1 ]----------+-------------------------------------------------------------------------------------
ek_olm_pickled_account | g6m36NbXJ6kzgWtwYZ9J9v1pm1rfBPMGZpL7OeMfOJN4ND3ZnF9NGqxvJwnTaCIxTcpkZrSqxkZxK2YEGvnN.
                       |.vEW//xSES/twWqbVx01kDhEm1HyY9uVqDOqQr1HL3XPs3y3ZxpGVk2m2DfbO6o7hj+OHzys4js799h4IuKsj.
...
ek_olm_identity_key    | 7jImYabiufIUOE4QjCfabJT/h1OhoMLztxqzEBqyn0w
```

The bulk of the user data is set by the client when it's created. These are the code lines that create the Olm account:

```rust
use vodozemac::olm::Account;

let account = Account::new();
let olm_identity_key = Some(account.curve25519_key().to_base64());
let olm_pickled_account = serde_json::to_string(&account.pickle()).unwrap();
```

### Olm One-Time-Key Generation

A one-time-key is needed to start an encrypted session with another user. To facilitate this, I've added a `/keys` endpoint to the Enigmatick actor objects.

```bash
> curl -sH "Accept: application/activity+json" "https://enigmatick.social/user/clark/keys" | jq
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Collection",
  "id": "https://enigmatick.social/user/clark/keys",
  "totalItems": 25
}
```

A signed request to this endpoint with `?otk=true` will provide a collection with the remote user's `OlmIdentityKey` and an `OlmOneTimeKey` to use to start an encrypted conversation.

```bash
> curl -H "Accept: application/activity+json" "https://enigmatick.social/user/clark/keys?otk=true" | jq
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Collection",
  "id": "https://enigmatick.social/collections/84567924-0848-4323-b7cd-0273f13030d7",
  "totalItems": 2,
  "items": [
    {
      "type": "OlmOneTimeKey",
      "id": "https://enigmatick.social/instruments/da267575-3787-4df2-ac11-e734f9f32d38",
      "content": "GVGzbKaznnKzgV5+/xRDfdx2lbMP0qBfcTaSV1+BATY"
    },
    {
      "type": "OlmIdentityKey",
      "id": "https://enigmatick.social/user/clark#identity-key",
      "content": "7jImYabiufIUOE4QjCfabJT/h1OhoMLztxqzEBqyn0w"
    }
  ]
}
```

The `OlmOneTimeKey` keys are created by the client and pushed to the server periodically. The frequency and nature of that process is an arbitrary implementation detail and is not critical to the ActivityPub exchange. What is important is that those keys be generated by the client and stored on the server in some way.

```rust
#[wasm_bindgen]
pub async fn add_one_time_keys(params: OtkUpdateParams) -> Option<String> {
    authenticated(move |_: EnigmatickState, profile: Profile| async move {
        let username = profile.username;
        let url = format!("/api/user/{username}/otk");

        let data = serde_json::to_string(&params).unwrap();
        send_post(url, data, "application/json".to_string()).await
    })
    .await
}
```

### Olm Session Initiation

Using the keys provided by the `/keys` endpoint, an actor may create a new Olm session with the recipient.

Below is a snippet from the `enigmatick_wasm` module in the `enigmatick-web` repository.

```rust
pub async fn encrypt_note(params: &mut SendParams) -> Result<()> {
    let mut session = if params.conversation.is_some() {
        get_olm_session(params.conversation.clone().unwrap()).await?
    } else {
        create_olm_session(params).await?
    };

    params.set_vault_item(params.get_content().clone().try_into()?);
    params.set_content(
        serde_json::to_string(&session.encrypt(params.get_content()))
            .map_err(anyhow::Error::msg)?,
    );
    params.set_olm_session(ApInstrument::try_from(session)?);

    Ok(())
}
```

There is more happening than meets the eye (review the repository for details). The end result is an `EncryptedNote` object wrapped in a `Create` activitiy that is passed to the recipient.

```json
{
    "@context": ["https://www.w3.org/ns/activitystreams"],
    "type": "Create",
    "to": ["https://enigmatick.social/user/clark"],
    "actor": "https://enigmatick.social/user/jdt",
    "id": "https://enigmatick.social/activities/b680adf8-9408-482d-9387-6dfa7904df67",
    "object": {
        "cc": [],
        "id": "https://enigmatick.social/objects/[some-uuid]",
        "to": [
            "https://enigmatick.social/user/clark"
        ],
        "conversation": "https://enigmatick.social/conversations/195f61c3-7098-4736-a837-1fab29e42699",
        "tag": [
            {
                "href": "https://enigmatick.social/user/clark",
                "name": "@clark@enigmatick.social",
                "type": "Mention"
            }
        ],
        "type": "EncryptedNote",
        "content": "{\"type\":0,\"body\":\"AwogZNVPkw0ZXxKnhDU0Tjf4NWX/2OvFTSx2IWxS1S1VK3gSINVvl3MHxmTBwyGVO+Bc8QqAP
ARVDTsPoCuRrrcEn7hxGiAFhZIDySEb5XYDxjEtbkueCuMfNs/7plG3FBweZdUfHiJnBAogDJQPFOjpBU81Hk4xildg7kNyTwn5Ntotk22UOEJ
vUg8QACIghqtjcoYnNKMk5unzO3qa0ckq/WihX18uwwbGN44Wm/8Kk6xgFQ7Cbl5q1DbNcPm6QlFnaJtyMveLLa7EzAJuUg\"}",
        "published": "2024-11-24T23:23:14.252Z",
        "attachment": [],
        "attributedTo": "https://enigmatick.social/user/jdt"
    },
    "instrument": [{
        "id": "https://enigmatick.social/instruments/112b6598-a23a-48cd-aeb0-721ab363413d",
        "type": "OlmIdentityKey",
        "content": "BYWSA8khG+V2A8YxLW5LngrjHzbP+6ZRtxQcHmXVHx4"
    }]
}
```

The above example is from my database, but it's based on the raw JSON sent by the client (hence the IDs aren't set). I removed the extraneous data the client also sends (`OlmAccount`, `OlmSession`, and `VaultItem` instruments). Those are stripped by the server before sending to the recipient's server, but are captured on the local server for persistence. That is an implementation detail specific to Enigmatick.

I'm not committed to transferring the `OlmIdentityKey` in this exchange. It may be more powerful to require the receiver to retrieve that key from the source explicitly, in a similar manner to how ActivityPub uses HTTP signatures to validate `POST` messages. Initially, I used a type of `Note` and the presence of the `OlmIdentityKey` to signal to the client that decryption was required. But with the dedicated `EncryptedNote` type, that's no longer a concern.

### Olm Session Acceptance

A fully operational communications channel requires that both sides have an Olm session. When a message such as that illustrated above is received, the recipient can initiate their side of the encryption channel using the `OlmIdentityKey` provided (or by reaching out an retrieving it from the sender's server).

Enigmatick uses this function to check if there is an existing Olm session or if one must be created.

```rust
fn transform_asymmetric_activity(
    account: &mut Account,
    sessions: &mut HashMap<String, String>,
    create: ApCreate,
    note: ApNote,
) -> Option<Vec<ApInstrument>> {
    find_session_instrument(&create)
        .and_then(|instrument| {
            use_session(instrument, sessions, create.clone(), &note)
                .map(|instruments| instruments)
            })
        .or_else(|| {
            find_identity_key_instrument(&create).and_then(|instrument| {
                create_session(account, instrument, create.clone(), &note)
                    .map(|(instruments, _message)| instruments)
            })
    })
}
```

When the activity is provided to the client, instruments are attached as they exist. In this case, only the `OlmIdentityKey` would exist for the receiving user (the existing session belongs to the sending user). As such, the `create_session` function is called:

```rust
fn create_session(
    account: &mut Account,
    idk: ApInstrument,
    create: ApCreate,
    note: &ApNote,
) -> Option<(Vec<ApInstrument>, String)> {
    let identity_key = Curve25519PublicKey::from_base64(&idk.content.unwrap()).ok()?;

    if let OlmMessage::PreKey(m) = serde_json::from_str(&note.content).ok()? {
        let inbound = account.create_inbound_session(identity_key, &m).ok()?;

        let message = String::from_utf8(inbound.plaintext).ok()?;

        let mut session_instrument = ApInstrument::try_from(inbound.session).ok()?;
        session_instrument.conversation = note.conversation.clone();

        let mut vault_instrument = ApInstrument::try_from(message.clone()).ok()?;
        vault_instrument.activity = create.id;

        Some((vec![session_instrument, vault_instrument], message))
    } else {
        None
    }
}
```

In this function, a new Olm session is created and used to decrypt the message. A vaulted version of the message is encrypted to be saved on the server (so that it can be retrieved and read later; the Olm operations can generally not be repeated).

The `OlmAccount`, `OlmSession`, and `VaultItem` records created in the above function will need to be sent to the server for update and storage (in the Enigmatick use-case; other clients may handle that differently).

### Established Olm Session Usage

Once established, the Olm sessions can be called on to encrypt and decrypt messages without too much fuss. In Enigmatick, an established session is used when the ID of a session instrument is included with the activity.

```rust
fn use_session(
    instrument: ApInstrument,
    sessions: &mut HashMap<String, String>,
    create: ApCreate,
    note: &ApNote,
) -> Option<Vec<ApInstrument>> {
    let decrypted_instrument_content = &decrypt(None, instrument.content?).ok()?;
    let pickled_string = sessions
        .entry(instrument.id?)
        .or_insert_with(|| decrypted_instrument_content.clone());

    let pickle = serde_json::from_str::<SessionPickle>(pickled_string).ok()?;

    let mut session = Session::from_pickle(pickle);

    if let OlmMessage::Normal(m) = serde_json::from_str(&note.content).ok()? {
        let bytes = session.decrypt(&m.into()).ok()?;
        let message = String::from_utf8(bytes).ok()?;

        let mut session_instrument = ApInstrument::try_from(session).ok()?;
        session_instrument.conversation = note.conversation.clone();

        let mut vault_instrument = ApInstrument::try_from(message.clone()).ok()?;
        vault_instrument.activity = create.id;

        Some(vec![session_instrument, vault_instrument])
    } else {
        None
    }
}
```

Again, the session is used only the first time that a message is decrypted. Once decrypted by the session, it is symmetrically re-encrypted as a `VaultItem` to be included with future retrievals.

