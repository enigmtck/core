use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ActivityPub, ApAddress, ApContext, ApInstrument, ApNote, ApObject, Temporal},
    models::{
        activities::EncryptedActivity,
        activities::{ActivityType, ExtendedActivity},
        coalesced_activity::CoalescedActivity,
        objects::ObjectType,
    },
    MaybeMultiple, MaybeReference,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{question::ApQuestion, signature::ApSignature, Ephemeral};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCreateType {
    #[default]
    #[serde(alias = "create")]
    Create,
}

impl fmt::Display for ApCreateType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl TryFrom<String> for ApCreateType {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.to_lowercase() == "create" {
            Ok(ApCreateType::Create)
        } else {
            Err(anyhow!("not a create type"))
        }
    }
}

impl TryFrom<ActivityType> for ApCreateType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Create => Ok(ApCreateType::Create),
            _ => Err(anyhow!("invalid ActivityType")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCreate {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApCreateType,
    pub actor: ApAddress,
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<ApSignature>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub instrument: MaybeMultiple<ApInstrument>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl TryFrom<CoalescedActivity> for ApCreate {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced
            .clone()
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
        {
            ObjectType::Note => Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into()),
            ObjectType::EncryptedNote => {
                Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into())
            }
            ObjectType::Question => {
                Ok(ApObject::Question(ApQuestion::try_from(coalesced.clone())?).into())
            }
            _ => Err(anyhow!("invalid type")),
        }?;
        let kind = coalesced.kind.clone().try_into()?;
        let actor = ApAddress::Address(coalesced.actor.clone());
        let id = coalesced.ap_id.clone();
        let context = Some(ApContext::default());
        let to = coalesced.ap_to.clone().into();
        let cc = coalesced.cc.clone().into();
        let signature = None;
        let published = Some(ActivityPub::time(coalesced.created_at));
        let ephemeral = Some(Ephemeral {
            created_at: Some(coalesced.created_at),
            updated_at: Some(coalesced.updated_at),
            ..Default::default()
        });

        let mut instrument: MaybeMultiple<ApInstrument> = coalesced.instrument.clone().into();

        if let Ok(instruments) = Vec::<ApInstrument>::try_from(coalesced) {
            instrument = instrument.extend(instruments);
        }

        Ok(ApCreate {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            signature,
            published,
            ephemeral,
            instrument,
        })
    }
}

impl TryFrom<EncryptedActivity> for ApCreate {
    type Error = anyhow::Error;

    fn try_from((activity, object, session): EncryptedActivity) -> Result<Self, Self::Error> {
        let note = ApObject::Note(ApNote::try_from(object)?);

        let instrument: MaybeMultiple<ApInstrument> = activity.instrument.into();
        let mut instrument = match instrument {
            MaybeMultiple::Single(instrument) => {
                if instrument.is_olm_identity_key() {
                    vec![instrument].into()
                } else {
                    MaybeMultiple::None
                }
            }
            MaybeMultiple::Multiple(instruments) => instruments
                .into_iter()
                .filter(|x| x.is_olm_identity_key())
                .collect::<Vec<ApInstrument>>()
                .into(),
            _ => MaybeMultiple::None,
        };

        if let Some(session) = session {
            instrument = instrument.extend(vec![session.into()]);
        }

        Ok(ApCreate {
            context: Some(ApContext::default()),
            kind: ApCreateType::default(),
            actor: ApAddress::Address(activity.actor.clone()),
            id: activity.ap_id,
            object: note.into(),
            to: activity.ap_to.clone().into(),
            cc: activity.cc.into(),
            signature: None,
            published: Some(ActivityPub::time(activity.created_at)),
            ephemeral: Some(Ephemeral {
                created_at: Some(activity.created_at),
                updated_at: Some(activity.updated_at),
                ..Default::default()
            }),
            instrument,
        })
    }
}
// I suspect that any uses of this have now been redirected to the CoalescedActivity above,
// even if functions are still calling this impl. It would be good to remove this and clean
// up the function chains.
impl TryFrom<ExtendedActivity> for ApCreate {
    type Error = anyhow::Error;
    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let note = {
            if let Some(object) = target_object.clone() {
                ApObject::Note(ApNote::try_from(object)?)
            } else {
                return Err(anyhow!("ACTIVITY MUST INCLUDE A NOTE OR REMOTE_NOTE"));
            }
        };

        let instrument: MaybeMultiple<ApInstrument> = activity.instrument.into();
        let instrument = match instrument {
            MaybeMultiple::Single(instrument) => {
                if instrument.is_olm_identity_key() {
                    vec![instrument].into()
                } else {
                    MaybeMultiple::None
                }
            }
            MaybeMultiple::Multiple(instruments) => instruments
                .into_iter()
                .filter(|x| x.is_olm_identity_key())
                .collect::<Vec<ApInstrument>>()
                .into(),
            _ => MaybeMultiple::None,
        };

        Ok(ApCreate {
            context: Some(ApContext::default()),
            kind: ApCreateType::default(),
            actor: ApAddress::Address(activity.actor.clone()),
            id: activity.ap_id,
            object: note.into(),
            to: activity.ap_to.clone().into(),
            cc: activity.cc.into(),
            signature: None,
            published: Some(ActivityPub::time(activity.created_at)),
            ephemeral: Some(Ephemeral {
                created_at: Some(activity.created_at),
                updated_at: Some(activity.updated_at),
                ..Default::default()
            }),
            instrument,
        })
    }
}

impl Temporal for ApCreate {
    fn published(&self) -> String {
        if let Some(published) = &self.published {
            published.to_string()
        } else if let MaybeReference::Actual(ApObject::Note(note)) = &self.object {
            note.published.clone()
        } else {
            ActivityPub::time(Utc::now())
        }
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral.clone().and_then(|x| x.created_at)
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral.clone().and_then(|x| x.updated_at)
    }
}

impl TryFrom<ApObject> for ApCreate {
    type Error = anyhow::Error;

    fn try_from(object: ApObject) -> Result<ApCreate> {
        match object.clone() {
            ApObject::Note(note) => {
                let context = Some(ApContext::default());
                let kind = ApCreateType::default();
                let actor = note.attributed_to;
                let id = None; // The ID is assigned in NewActivity
                let object = object.into();
                let to = note.to;
                let cc = note.cc;
                let signature = None;
                let published = Some(note.published);
                let ephemeral = None;

                let instrument: MaybeMultiple<ApInstrument> =
                    note.instrument.map_or(MaybeMultiple::None, |x| x);

                let instrument = match instrument {
                    MaybeMultiple::Single(instrument) => {
                        if instrument.is_olm_identity_key() {
                            vec![instrument].into()
                        } else {
                            MaybeMultiple::None
                        }
                    }
                    MaybeMultiple::Multiple(instruments) => instruments
                        .into_iter()
                        .filter(|x| x.is_olm_identity_key())
                        .collect::<Vec<ApInstrument>>()
                        .into(),
                    _ => MaybeMultiple::None,
                };

                Ok(ApCreate {
                    context,
                    kind,
                    actor,
                    id,
                    object,
                    to,
                    cc,
                    signature,
                    published,
                    ephemeral,
                    instrument,
                })
            }
            _ => Err(anyhow!("unimplemented object type")),
        }
    }
}
