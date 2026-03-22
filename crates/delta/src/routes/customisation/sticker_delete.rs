use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, EmojiParent, User,
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::rollout;

/// # Delete sticker
#[openapi(tag = "Stickers")]
#[delete("/stickers/<sticker_id>")]
pub async fn delete_sticker(
    db: &State<Database>,
    user: User,
    sticker_id: Reference<'_>,
) -> Result<EmptyResponse> {
    if !rollout::user_has_feature(&user, "stickers_v1") {
        return Err(create_error!(NotFound));
    }

    let sticker = sticker_id.as_sticker(db).await?;

    if sticker.creator_id != user.id {
        match &sticker.parent {
            EmojiParent::Server { id } => {
                let server = db.fetch_server(id.as_str()).await?;
                let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
                calculate_server_permissions(&mut query)
                    .await
                    .throw_if_lacking_channel_permission(ChannelPermission::ManageCustomisation)?;
            }
            EmojiParent::Detached => return Ok(EmptyResponse),
        };
    }

    sticker.delete(db).await.map(|_| EmptyResponse)
}
