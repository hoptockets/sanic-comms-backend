use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod emoji_create;
mod emoji_delete;
mod emoji_fetch;
mod soundboard_create;
mod soundboard_delete;
mod soundboard_list;
mod sticker_create;
mod sticker_delete;
mod sticker_list;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        emoji_create::create_emoji,
        emoji_delete::delete_emoji,
        emoji_fetch::fetch_emoji,
        sticker_create::create_sticker,
        sticker_delete::delete_sticker,
        sticker_list::list_stickers_for_server,
        soundboard_create::create_soundboard_clip,
        soundboard_delete::delete_soundboard_clip,
        soundboard_list::list_soundboard_for_server
    ]
}
