use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{
        ApActor, ApAttachment, ApCollection, ApContext, ApImage, ApInstruments, ApTag, Outbox,
    },
    db::Db,
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::{
        get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
        get_note_url_from_uuid,
    },
    models::{
        activities::{create_activity, Activity, ActivityType, NewActivity},
        cache::{cache_content, Cache},
        notes::{create_note, NewNote, Note, NoteType},
        profiles::Profile,
        remote_notes::RemoteNote,
        timeline::{TimelineItem, TimelineItemCc},
        vault::VaultItem,
    },
    runner, MaybeMultiple, ANCHOR_RE,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use webpage::{Webpage, WebpageOptions};

use super::actor::ApAddress;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApNoteType {
    #[default]
    Note,
    EncryptedNote,
    VaultNote,
    Question,
}

impl fmt::Display for ApNoteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<NoteType> for ApNoteType {
    fn from(kind: NoteType) -> Self {
        match kind {
            NoteType::Note => ApNoteType::Note,
            NoteType::EncryptedNote => ApNoteType::EncryptedNote,
            NoteType::VaultNote => ApNoteType::VaultNote,
            NoteType::Question => ApNoteType::Question,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub twitter_title: Option<String>,
    pub description: Option<String>,
    pub og_description: Option<String>,
    pub og_title: Option<String>,
    pub og_image: Option<String>,
    pub og_site_name: Option<String>,
    pub twitter_image: Option<String>,
    pub og_url: Option<String>,
    pub twitter_description: Option<String>,
    pub published: Option<String>,
    pub twitter_site: Option<String>,
    pub og_type: Option<String>,
}

impl From<HashMap<String, String>> for Metadata {
    fn from(meta: HashMap<String, String>) -> Self {
        Metadata {
            twitter_title: meta.get("twitter:title").cloned(),
            description: meta.get("description").cloned(),
            og_description: meta.get("og:description").cloned(),
            og_title: meta.get("og:title").cloned(),
            og_image: meta.get("og:image").cloned(),
            og_site_name: meta.get("og:site_name").cloned(),
            twitter_image: meta.get("twitter:image").cloned(),
            og_url: meta.get("og:url").cloned(),
            twitter_description: meta.get("twitter:description").cloned(),
            published: meta.get("article:published").cloned(),
            twitter_site: meta.get("twitter:site").cloned(),
            og_type: meta.get("og:type").cloned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    pub tag: Option<Vec<ApTag>>,
    pub attributed_to: ApAddress,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApNoteType,
    //pub to: Vec<String>,
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<MaybeMultiple<ApAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApCollection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Vec<ApAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atom_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to_atom_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_map: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<ApInstruments>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_announces: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_actors: Option<Vec<ApActor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_liked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_announced: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_targeted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_metadata: Option<Vec<Metadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_likes: Option<Vec<String>>,
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        if let MaybeMultiple::Multiple(v) = self.to {
            let mut t = v;
            t.push(ApAddress::Address(to));
            self.to = MaybeMultiple::Multiple(t);
        }
        self
    }

    pub fn content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn tag(mut self, tag: ApTag) -> Self {
        self.tag.as_mut().expect("unwrap failed").push(tag);
        self
    }
}

impl Cache for ApNote {
    async fn cache(&self, conn: &Db) -> &Self {
        if let Some(attachments) = self.attachment.clone() {
            for attachment in attachments {
                cache_content(conn, attachment.clone().try_into()).await;
            }
        }

        if let Some(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.clone().try_into()).await;
            }
        }

        if let Some(metadata_vec) = self.ephemeral_metadata.clone() {
            for metadata in metadata_vec {
                if let Some(og_image) = metadata.og_image.clone() {
                    cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                }

                if let Some(twitter_image) = metadata.twitter_image.clone() {
                    cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                }
            }
        }

        self
    }
}

impl Outbox for ApNote {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        match self.kind {
            ApNoteType::Note => handle_note(conn, faktory, events, self.clone(), profile).await,
            ApNoteType::EncryptedNote => {
                handle_encrypted_note(conn, faktory, events, self.clone(), profile).await
            }
            _ => Err(Status::NoContent),
        }
    }
}

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Some(ApContext::default()),
            tag: None,
            attributed_to: ApAddress::default(),
            id: None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            url: None,
            published: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            cc: None,
            replies: None,
            attachment: None,
            in_reply_to: None,
            content: String::new(),
            summary: None,
            sensitive: None,
            atom_uri: None,
            in_reply_to_atom_uri: None,
            conversation: None,
            content_map: None,
            instrument: None,
            ephemeral_announces: None,
            ephemeral_actors: None,
            ephemeral_liked: None,
            ephemeral_announced: None,
            ephemeral_targeted: None,
            ephemeral_timestamp: None,
            ephemeral_metadata: None,
            ephemeral_likes: None,
        }
    }
}

type IdentifiedVaultItem = (VaultItem, Profile);

impl From<IdentifiedVaultItem> for ApNote {
    fn from((vault, profile): IdentifiedVaultItem) -> Self {
        ApNote {
            kind: ApNoteType::VaultNote,
            attributed_to: {
                if vault.outbound {
                    ApAddress::Address(get_ap_id_from_username(profile.clone().username))
                } else {
                    ApAddress::Address(vault.clone().remote_actor)
                }
            },
            to: {
                if vault.outbound {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(vault.remote_actor)])
                } else {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(get_ap_id_from_username(
                        profile.username,
                    ))])
                }
            },
            id: Some(format!(
                "https://{}/vault/{}",
                *crate::SERVER_NAME,
                vault.uuid
            )),
            content: vault.encrypted_data,
            published: vault.created_at.to_rfc3339(),
            ..Default::default()
        }
    }
}

impl From<&TimelineItem> for ApNote {
    fn from(timeline: &TimelineItem) -> Self {
        ApNote::from(((timeline.clone(), None, None), None, None))
    }
}

impl From<TimelineItem> for ApNote {
    fn from(timeline: TimelineItem) -> Self {
        ApNote::from(((timeline, None, None), None, None))
    }
}

// we're pre-loading the ApActor objects here so that we don't have to make
// separate calls to retrieve that data at the client; making those extra calls
// is particularly problematic for unauthenticated retrieval as it would require
// that we expose the endpoint for retreiving RemoteActor data to the world
pub type QualifiedTimelineItem = (TimelineItem, Option<Vec<ApActor>>);

impl From<QualifiedTimelineItem> for ApNote {
    fn from((timeline, actors): QualifiedTimelineItem) -> Self {
        ApNote::from(((timeline, None, None), actors, None))
    }
}

pub type FullyQualifiedTimelineItem = (
    (TimelineItem, Option<Activity>, Option<TimelineItemCc>),
    Option<Vec<ApActor>>,
    Option<Profile>,
);

impl From<FullyQualifiedTimelineItem> for ApNote {
    fn from(((timeline, activity, cc), actors, profile): FullyQualifiedTimelineItem) -> Self {
        ApNote {
            context: Some(ApContext::default()),
            to: MaybeMultiple::Multiple(vec![]),
            cc: None,
            instrument: None,
            kind: ApNoteType::Note,
            tag: {
                if let Some(x) = timeline.tag {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            attributed_to: ApAddress::Address(timeline.attributed_to),
            id: Some(timeline.ap_id),
            url: timeline.url,
            published: timeline.published.unwrap_or("".to_string()),
            replies: Option::None,
            in_reply_to: timeline.in_reply_to,
            content: timeline.content.unwrap_or_default(),
            summary: timeline.summary,
            sensitive: timeline.ap_sensitive,
            atom_uri: timeline.atom_uri,
            in_reply_to_atom_uri: timeline.in_reply_to_atom_uri,
            conversation: timeline.conversation,
            content_map: {
                if let Some(x) = timeline.content_map {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            attachment: {
                if let Some(x) = timeline.attachment {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            ephemeral_announces: activity
                .clone()
                .filter(|activity| activity.kind == ActivityType::Announce && !activity.revoked)
                .map(|announce| vec![announce.actor]),
            ephemeral_announced: activity.clone().and_then(|x| {
                if let Some(profile) = profile.clone() {
                    if x.kind == ActivityType::Announce
                        && !x.revoked
                        && x.actor == get_ap_id_from_username(profile.username)
                    {
                        Some(get_activity_ap_id_from_uuid(x.uuid))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            ephemeral_actors: actors,
            ephemeral_liked: activity.clone().and_then(|x| {
                if let Some(profile) = profile {
                    if x.kind == ActivityType::Like
                        && !x.revoked
                        && x.actor == get_ap_id_from_username(profile.username)
                    {
                        Some(get_activity_ap_id_from_uuid(x.uuid))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            ephemeral_likes: activity
                .filter(|activity| activity.kind == ActivityType::Like && !activity.revoked)
                .map(|like| vec![like.actor]),
            ephemeral_targeted: Some(cc.is_some()),
            ephemeral_timestamp: Some(timeline.created_at),
            ephemeral_metadata: {
                if let Some(x) = timeline.metadata {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
        }
    }
}

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            tag: Option::from(vec![]),
            attributed_to: actor.id.unwrap(),
            id: Option::None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            content: String::new(),
            ..Default::default()
        }
    }
}

impl From<NewNote> for ApNote {
    fn from(note: NewNote) -> Self {
        ApNote {
            tag: match serde_json::from_value(note.tag.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            attributed_to: ApAddress::Address(note.attributed_to),
            published: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            id: Option::from(format!(
                "https://{}/notes/{}",
                *crate::SERVER_NAME,
                note.uuid
            )),
            kind: note.kind.into(),
            to: match serde_json::from_value(note.ap_to) {
                Ok(x) => x,
                Err(_) => MaybeMultiple::Multiple(vec![]),
            },
            content: note.content,
            cc: match serde_json::from_value(note.cc.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            in_reply_to: note.in_reply_to,
            conversation: note.conversation,
            attachment: note.attachment.map(|x| serde_json::from_value(x).unwrap()),
            ..Default::default()
        }
    }
}

impl From<Note> for ApNote {
    fn from(note: Note) -> Self {
        ApNote {
            tag: serde_json::from_value(note.tag.into()).ok(),
            attributed_to: ApAddress::Address(note.attributed_to),
            published: note.updated_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            id: note
                .ap_id
                .clone()
                .map_or(Some(get_note_ap_id_from_uuid(note.uuid.clone())), Some),
            url: Some(get_note_url_from_uuid(note.uuid)),
            kind: note.kind.into(),
            to: match serde_json::from_value(note.ap_to) {
                Ok(x) => x,
                Err(_) => MaybeMultiple::Multiple(vec![]),
            },
            content: note.content,
            cc: match serde_json::from_value(note.cc.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            in_reply_to: note.in_reply_to,
            conversation: note.conversation,
            attachment: note.attachment.map(|x| serde_json::from_value(x).unwrap()),
            ephemeral_metadata: Some(vec![]),
            ..Default::default()
        }
    }
}

// TODO: This is problematic for links that point to large files; the filter tries
// to account for some of that, but that's not really a solution. Maybe a whitelist?
// That would suck. I wish the Webpage crate had a size limit (i.e., load pages with
// a maximum size of 10MB or whatever a reasonable amount would be).
fn get_links(text: String) -> Vec<String> {
    ANCHOR_RE
        .captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_lowercase().contains("mention")
                && !cap[0].to_lowercase().contains("u-url")
                && !cap[0].to_lowercase().contains("hashtag")
                && !cap[0].to_lowercase().contains("download")
                && !cap[1].to_lowercase().contains(".pdf")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

fn metadata(remote_note: &RemoteNote) -> Vec<Metadata> {
    get_links(remote_note.content.clone())
        .iter()
        .map(|link| Webpage::from_url(link, WebpageOptions::default()))
        .filter(|metadata| metadata.is_ok())
        .map(|metadata| metadata.unwrap().html.meta.into())
        .collect()
}

impl From<RemoteNote> for ApNote {
    fn from(remote_note: RemoteNote) -> ApNote {
        let kind = match remote_note.kind {
            NoteType::Note => ApNoteType::Note,
            NoteType::EncryptedNote => ApNoteType::EncryptedNote,
            _ => ApNoteType::default(),
        };

        ApNote {
            id: Some(remote_note.ap_id.clone()),
            kind,
            published: remote_note.published.clone().unwrap_or("".to_string()),
            url: remote_note.url.clone(),
            to: match serde_json::from_value(remote_note.ap_to.clone().into()) {
                Ok(x) => x,
                Err(_) => MaybeMultiple::Multiple(vec![]),
            },
            cc: match serde_json::from_value(remote_note.cc.clone().into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            tag: match serde_json::from_value(remote_note.tag.clone().into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            attributed_to: ApAddress::Address(remote_note.attributed_to.clone()),
            content: remote_note.content.clone(),
            replies: match serde_json::from_value(remote_note.replies.clone().into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            in_reply_to: remote_note.in_reply_to.clone(),
            attachment: match serde_json::from_value(
                remote_note.attachment.clone().unwrap_or_default(),
            ) {
                Ok(x) => x,
                Err(_) => None,
            },
            conversation: remote_note.conversation.clone(),
            ephemeral_timestamp: Some(remote_note.created_at),
            ephemeral_metadata: Some(metadata(&remote_note)),
            ..Default::default()
        }
    }
}

async fn handle_note(
    conn: Db,
    faktory: FaktoryConnection,
    channels: EventChannels,
    mut note: ApNote,
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote

    let mut is_public = false;
    let mut followers_included = false;
    let mut addresses_cc: Vec<ApAddress> = note.cc.clone().unwrap_or(vec![].into()).multiple();
    let followers = ApActor::from(profile.clone()).followers;

    if let Some(followers) = followers {
        // look for the public and followers group address aliases in the to vec
        for to in note.to.multiple().iter() {
            if to.is_public() {
                is_public = true;
                if to.to_string().to_lowercase() == followers.to_lowercase() {
                    followers_included = true;
                }
            }
        }

        // look for the public and followers group address aliases in the cc vec
        for cc in addresses_cc.iter() {
            if cc.is_public() {
                is_public = true;
                if cc.to_string().to_lowercase() == followers.to_lowercase() {
                    followers_included = true;
                }
            }
        }

        // if the note is public and if it's not already included, add the sender's followers group
        if is_public && !followers_included {
            addresses_cc.push(followers.into());
            note.cc = Some(MaybeMultiple::Multiple(addresses_cc));
        }
    }

    let created_note = create_note(&conn, NewNote::from((note.clone(), profile.id)))
        .await
        .ok_or(Status::new(520))?;

    let activity = create_activity(
        Some(&conn),
        NewActivity::from((
            Some(created_note.clone()),
            None,
            ActivityType::Create,
            ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
        ))
        .link_profile(&conn)
        .await,
    )
    .await
    .map_err(|_| Status::new(521))?;

    runner::run(
        runner::note::outbound_note_task,
        Some(conn),
        Some(channels),
        vec![activity.uuid.clone()],
    )
    .await;
    Ok(activity.uuid)

    // if assign_to_faktory(
    //     faktory,
    //     String::from("process_outbound_note"),
    //     vec![activity.uuid.clone()],
    // )
    // .is_ok()
    // {
    //     Ok(activity.uuid)
    // } else {
    //     Err(Status::new(522))
    // }
}

async fn handle_encrypted_note(
    conn: Db,
    faktory: FaktoryConnection,
    channels: EventChannels,
    note: ApNote,
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));

    if let Some(created_note) = create_note(&conn, n.clone()).await {
        log::debug!("created_note\n{created_note:#?}");

        // let ap_note = ApNote::from(created_note.clone());
        // let mut events = events;
        // events.send(serde_json::to_string(&ap_note).unwrap());

        runner::run(
            runner::note::outbound_note_task,
            Some(conn),
            Some(channels),
            vec![created_note.uuid.clone()],
        )
        .await;
        Ok(created_note.uuid)

        // if assign_to_faktory(
        //     faktory,
        //     String::from("process_outbound_note"),
        //     vec![created_note.uuid.clone()],
        // )
        // .is_ok()
        // {
        //     Ok(created_note.uuid)
        // } else {
        //     Err(Status::NoContent)
        // }
    } else {
        Err(Status::NoContent)
    }
}
