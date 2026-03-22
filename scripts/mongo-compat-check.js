const dbx = db.getSiblingDB("revolt");

const report = {
  migration: {
    revision: (() => {
      const doc = dbx.migrations.findOne({});
      return doc ? doc.revision : null;
    })(),
  },
  totals: {
    users: dbx.users.countDocuments({}),
    servers: dbx.servers.countDocuments({}),
    server_members: dbx.server_members.countDocuments({}),
    user_settings: dbx.user_settings.countDocuments({}),
  },
  compatibility: {
    users_profile_cosmetics_missing: dbx.users.countDocuments({
      "profile.cosmetics": { $exists: false },
    }),
    users_profile_cosmetics_non_object: dbx.users.countDocuments({
      "profile.cosmetics": { $exists: true, $not: { $type: "object" } },
    }),
    servers_theme_accent_non_string_or_null: dbx.servers.countDocuments({
      theme_accent: { $exists: true, $not: { $type: ["string", "null"] } },
    }),
    servers_theme_preset_non_string_or_null: dbx.servers.countDocuments({
      theme_preset: { $exists: true, $not: { $type: ["string", "null"] } },
    }),
    server_members_nickname_non_string_or_null: dbx.server_members.countDocuments({
      nickname: { $exists: true, $not: { $type: ["string", "null"] } },
    }),
    user_settings_with_admin_restriction_keys: dbx.user_settings.countDocuments({
      $or: [
        { "admin:restriction:profile_locked": { $exists: true } },
        { "admin:restriction:media_quarantined": { $exists: true } },
        { "admin:restriction:profile_visibility_limited": { $exists: true } },
      ],
    }),
    users_platform_permissions_non_array: dbx.users.countDocuments({
      platform_permissions: { $exists: true, $not: { $type: "array" } },
    }),
    safety_reports_missing_v2_fields: dbx.safety_reports.countDocuments({
      $or: [
        { severity: { $exists: false } },
        { priority: { $exists: false } },
        { breach_state: { $exists: false } },
        { created_at: { $exists: false } },
        { last_transition_at: { $exists: false } },
        { related_report_ids: { $exists: false } },
      ],
    }),
  },
};

printjson(report);
