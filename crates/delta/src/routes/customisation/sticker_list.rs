use revolt_database::{util::reference::Reference, Database, User};
use revolt_models::v0;
use revolt_result::Result;

use rocket::{serde::json::Json, State};

use crate::util::rollout;

#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct StickerListResponse {
    pub stickers: Vec<v0::Emoji>,
}

/// # List server stickers
#[openapi(tag = "Stickers")]
#[get("/stickers/server/<server_id>")]
pub async fn list_stickers_for_server(
    db: &State<Database>,
    user: User,
    server_id: Reference<'_>,
) -> Result<Json<StickerListResponse>> {
    if !rollout::user_has_feature(&user, "stickers_v1") {
        return Ok(Json(StickerListResponse { stickers: vec![] }));
    }

    let server = server_id.as_server(db).await?;
    let stickers = db.fetch_stickers_by_parent_id(&server.id).await?;
    Ok(Json(StickerListResponse {
        stickers: stickers.into_iter().map(Into::into).collect(),
    }))
}
