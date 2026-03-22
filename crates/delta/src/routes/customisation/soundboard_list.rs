use revolt_database::{util::reference::Reference, Database, User};
use revolt_models::v0;
use revolt_result::Result;

use rocket::{serde::json::Json, State};

use crate::util::rollout;

#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct SoundboardListResponse {
    pub clips: Vec<v0::SoundboardClip>,
}

/// # List server soundboard clips
#[openapi(tag = "Soundboard")]
#[get("/soundboard/server/<server_id>")]
pub async fn list_soundboard_for_server(
    db: &State<Database>,
    user: User,
    server_id: Reference<'_>,
) -> Result<Json<SoundboardListResponse>> {
    if !rollout::user_has_feature(&user, "soundboard_v1") {
        return Ok(Json(SoundboardListResponse { clips: vec![] }));
    }

    let server = server_id.as_server(db).await?;
    let clips = db.fetch_soundboard_by_parent_id(&server.id).await?;
    Ok(Json(SoundboardListResponse {
        clips: clips.into_iter().map(Into::into).collect(),
    }))
}
