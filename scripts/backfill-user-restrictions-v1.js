/* eslint-disable no-console */
/**
 * Backfills DB-backed admin restriction keys inside user_settings.
 *
 * Usage:
 *   MONGO_URI="mongodb://localhost:27017/revolt" node scripts/backfill-user-restrictions-v1.js
 */
let MongoClient;
try {
  ({ MongoClient } = require("mongodb"));
} catch (error) {
  console.error(
    "Missing dependency: `mongodb`. Install it in this environment before running this backfill script.",
  );
  process.exit(1);
}

const uri = process.env.MONGO_URI || "mongodb://localhost:27017/revolt";
const [, dbNameFromUri] = uri.split("/").slice(-1);
const dbName = dbNameFromUri || "revolt";

const DEFAULT_RESTRICTIONS = {
  "admin:restriction:profile_locked": [Date.now(), "false"],
  "admin:restriction:username_frozen_until": [Date.now(), ""],
  "admin:restriction:display_name_frozen_until": [Date.now(), ""],
  "admin:restriction:media_quarantined": [Date.now(), "false"],
  "admin:restriction:profile_visibility_limited": [Date.now(), "false"],
};

async function run() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db(dbName);
  const users = db.collection("users");
  const userSettings = db.collection("user_settings");

  const cursor = users.find({}, { projection: { _id: 1 } });
  let updated = 0;

  while (await cursor.hasNext()) {
    const user = await cursor.next();
    const set = {};
    for (const [key, value] of Object.entries(DEFAULT_RESTRICTIONS)) {
      set[key] = value;
    }

    await userSettings.updateOne(
      { _id: user._id },
      { $setOnInsert: { _id: user._id }, $set: set },
      { upsert: true },
    );
    updated += 1;
  }

  console.log(`Backfilled restriction keys for ${updated} users.`);
  await client.close();
}

run().catch((error) => {
  console.error("Restriction backfill failed:", error);
  process.exit(1);
});
