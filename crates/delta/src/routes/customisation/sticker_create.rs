use revolt_config::config;
use revolt_database::{
    util::permissions::DatabasePermissionQuery, Database, EmojiParent, File, Sticker, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use validator::Validate;

use rocket::{serde::json::Json, State};

use crate::util::rollout;

/// # Create sticker
#[openapi(tag = "Stickers")]
#[put("/stickers/<id>", data = "<data>")]
pub async fn create_sticker(
    db: &State<Database>,
    user: User,
    id: String,
    data: Json<v0::DataCreateEmoji>,
) -> Result<Json<v0::Emoji>> {
    if !rollout::user_has_feature(&user, "stickers_v1") {
        return Err(create_error!(NotFound));
    }

    let config = config().await;
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

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

    let stickers = db.fetch_stickers_by_parent_id(&server.id).await?;
    if stickers.len() >= config.features.limits.global.server_emoji {
        return Err(create_error!(TooManyEmoji {
            max: config.features.limits.global.server_emoji,
        }));
    }

    let attachment = File::use_sticker(db, &id, &id, &user.id).await?;

    let sticker = Sticker {
        id,
        parent: EmojiParent::Server {
            id: server.id.clone(),
        },
        creator_id: user.id,
        name: data.name,
        animated: attachment.content_type == "image/gif",
        nsfw: data.nsfw,
    };

    sticker.create(db).await?;
    Ok(Json(sticker.into()))
}
