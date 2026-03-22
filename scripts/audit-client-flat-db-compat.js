/* eslint-disable no-console */
/**
 * Audits live Mongo data compatibility for the client flat overhaul.
 *
 * Usage:
 *   MONGO_URI="mongodb://localhost:27017/revolt" node scripts/audit-client-flat-db-compat.js
 */
let MongoClient;
try {
  ({ MongoClient } = require("mongodb"));
} catch (error) {
  console.error(
    "Missing dependency: `mongodb`. Install it in this environment before running this audit script.",
  );
  process.exit(1);
}

const uri = process.env.MONGO_URI || "mongodb://localhost:27017/revolt";
const [, dbNameFromUri] = uri.split("/").slice(-1);
const dbName = dbNameFromUri || "revolt";

async function countTypeMismatch(collection, field, allowedTypes) {
  return collection.countDocuments({
    [field]: { $exists: true, $not: { $type: allowedTypes } },
  });
}

async function run() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db(dbName);

  const users = db.collection("users");
  const servers = db.collection("servers");
  const members = db.collection("server_members");
  const userSettings = db.collection("user_settings");

  const [
    userCount,
    serverCount,
    memberCount,
    settingsCount,
    cosmeticsMissing,
    cosmeticsTypeMismatch,
    themeAccentTypeMismatch,
    themePresetTypeMismatch,
    nicknameTypeMismatch,
    settingsKeyTypeMismatch,
    restrictionsDocuments,
  ] = await Promise.all([
    users.countDocuments({}),
    servers.countDocuments({}),
    members.countDocuments({}),
    userSettings.countDocuments({}),
    users.countDocuments({ "profile.cosmetics": { $exists: false } }),
    countTypeMismatch(users, "profile.cosmetics", "object"),
    countTypeMismatch(servers, "theme_accent", ["string", "null"]),
    countTypeMismatch(servers, "theme_preset", ["string", "null"]),
    countTypeMismatch(members, "nickname", ["string", "null"]),
    userSettings.countDocuments({
      $or: [
        {
          "appearance:compact_mode": {
            $exists: true,
            $not: { $type: "array" },
          },
        },
        {
          "appearance:unicode_emoji": {
            $exists: true,
            $not: { $type: "array" },
          },
        },
      ],
    }),
    userSettings.countDocuments({
      $or: [
        { "admin:restriction:profile_locked": { $exists: true } },
        { "admin:restriction:media_quarantined": { $exists: true } },
        { "admin:restriction:profile_visibility_limited": { $exists: true } },
      ],
    }),
  ]);

  const report = {
    database: dbName,
    totals: {
      users: userCount,
      servers: serverCount,
      server_members: memberCount,
      user_settings: settingsCount,
    },
    compatibility: {
      users_profile_cosmetics_missing: cosmeticsMissing,
      users_profile_cosmetics_type_mismatch: cosmeticsTypeMismatch,
      servers_theme_accent_type_mismatch: themeAccentTypeMismatch,
      servers_theme_preset_type_mismatch: themePresetTypeMismatch,
      server_members_nickname_type_mismatch: nicknameTypeMismatch,
      user_settings_known_keys_type_mismatch: settingsKeyTypeMismatch,
      user_restrictions_documents_present: restrictionsDocuments,
    },
    deployment_safe:
      cosmeticsTypeMismatch === 0 &&
      themeAccentTypeMismatch === 0 &&
      themePresetTypeMismatch === 0 &&
      nicknameTypeMismatch === 0 &&
      settingsKeyTypeMismatch === 0,
  };

  console.log(JSON.stringify(report, null, 2));
  await client.close();
}

run().catch((error) => {
  console.error("DB compatibility audit failed:", error);
  process.exit(1);
});
