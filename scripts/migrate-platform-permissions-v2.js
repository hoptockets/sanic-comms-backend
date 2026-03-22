/* eslint-disable no-console */
/**
 * Migrates legacy platform_permissions entries to catalog-based v2 entries.
 *
 * Usage:
 *   MONGO_URI="mongodb://localhost:27017/revolt" node scripts/migrate-platform-permissions-v2.js
 */
const { MongoClient } = require("mongodb");

const uri = process.env.MONGO_URI || "mongodb://localhost:27017/revolt";
const [, dbNameFromUri] = uri.split("/").slice(-1);
const dbName = dbNameFromUri || "revolt";

const LEGACY_TO_V2 = {
  "feature:admin_panel_v1": "feature:admin_panel_v1",
  "feature:profile_v3": "feature:profile_v3",
  "feature:profile_v2": "feature:profile_v2",
};

function normalizePermission(value) {
  if (!value) return null;
  if (value.startsWith("feature:")) return value;
  return LEGACY_TO_V2[value] || value;
}

async function run() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db(dbName);
  const users = db.collection("users");

  const cursor = users.find({
    platform_permissions: { $exists: true, $type: "array" },
  });

  let updated = 0;

  while (await cursor.hasNext()) {
    const user = await cursor.next();
    const nextPermissions = (user.platform_permissions || [])
      .map(normalizePermission)
      .filter(Boolean);

    const deduped = Array.from(new Set(nextPermissions));

    await users.updateOne(
      { _id: user._id },
      { $set: { platform_permissions: deduped } },
    );
    updated += 1;
  }

  console.log(`Migrated platform permissions for ${updated} users.`);
  await client.close();
}

run().catch((error) => {
  console.error("Permission migration failed:", error);
  process.exit(1);
});

