use revolt_database::User;
use revolt_result::Result;
use rocket::serde::json::Json;
use serde::Serialize;
use std::collections::HashMap;

use crate::util::rollout;

#[derive(Serialize, JsonSchema)]
pub struct FeaturesResponse {
    pub emergency_kill_switch: bool,
    pub global: HashMap<String, bool>,
    pub promoted: Vec<String>,
    pub effective: HashMap<String, bool>,
}

/// # Feature Rollout Matrix
///
/// Returns global rollout switches and user-specific promoted features.
#[openapi(tag = "Sync")]
#[get("/features")]
pub async fn features(user: User) -> Result<Json<FeaturesResponse>> {
    let global = rollout::global_features();
    let promoted = rollout::promoted_features(&user);
    let effective = rollout::effective_features(&user);
    let emergency_kill_switch = rollout::emergency_kill_switch();

    Ok(Json(FeaturesResponse {
        emergency_kill_switch,
        global,
        promoted,
        effective,
    }))
}
