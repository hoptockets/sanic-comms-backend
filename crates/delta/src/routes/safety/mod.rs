use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod management;
mod report_content;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        // Reports
        report_content::report_content,
        management::list_reports,
        management::get_report,
        management::review_report,
        management::report_action,
        management::report_timeline,
        management::report_snapshots,
        management::reports_dashboard,
        management::list_users,
        management::inspect_user,
        management::set_user_flags,
        management::set_user_cosmetics,
        management::get_user_restrictions,
        management::set_user_restrictions,
        management::list_staff,
        management::staff_permissions_catalog,
        management::assign_staff,
        management::get_system_features,
        management::set_system_feature,
        management::set_system_kill_switch,
        management::get_system_email,
        management::set_system_email,
        management::get_audit_log,
        management::export_audit_log,
        management::sensitive_audit_feed,
    ]
}
