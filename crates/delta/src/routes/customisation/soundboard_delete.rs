use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, EmojiParent, User,
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use rocket::State;
use rocket_empty::EmptyResponse;

use crate::util::rollout;

/// # Delete soundboard clip
#[openapi(tag = "Soundboard")]
#[delete("/soundboard/<clip_id>")]
pub async fn delete_soundboard_clip(
    db: &State<Database>,
    user: User,
    clip_id: Reference<'_>,
) -> Result<EmptyResponse> {
    if !rollout::user_has_feature(&user, "soundboard_v1") {
        return Err(create_error!(NotFound));
    }

    let clip = clip_id.as_soundboard_clip(db).await?;

    if clip.creator_id != user.id {
        match &clip.parent {
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

    clip.delete(db).await.map(|_| EmptyResponse)
}
