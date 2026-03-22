use revolt_config::config;
use revolt_database::{
    util::permissions::DatabasePermissionQuery, Database, EmojiParent, File, SoundboardClip, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::{serde::json::Json, State};

use crate::util::rollout;

/// # Create soundboard clip
#[openapi(tag = "Soundboard")]
#[put("/soundboard/<id>", data = "<data>")]
pub async fn create_soundboard_clip(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<v0::DataCreateSoundboardClip>,
) -> Result<Json<v0::SoundboardClip>> {
    if !rollout::user_has_feature(&user, "soundboard_v1") {
        return Err(create_error!(NotFound));
    }

    let config = config().await;
    let data = data.into_inner();

    if !revolt_models::v0::RE_EMOJI.is_match(&data.name) {
        return Err(create_error!(FailedValidation {
            error: "invalid soundboard name".to_owned()
        }));
    }

    let server_id = match &data.parent {
        v0::EmojiParent::Server { id } => id.as_str(),
        v0::EmojiParent::Detached => {
            return Err(create_error!(InvalidOperation));
        }
    };

    let server = db.fetch_server(server_id).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageCustomisation)?;

    let clips = db.fetch_soundboard_by_parent_id(&server.id).await?;
    if clips.len() >= config.features.limits.global.server_emoji {
        return Err(create_error!(TooManyEmoji {
            max: config.features.limits.global.server_emoji,
        }));
    }

    let _attachment = File::use_soundboard_clip(db, &id, &id, &user.id).await?;

    let clip = SoundboardClip {
        id,
        parent: EmojiParent::Server {
            id: server.id.clone(),
        },
        creator_id: user.id,
        name: data.name,
    };

    clip.create(db).await?;
    Ok(Json(clip.into()))
}
