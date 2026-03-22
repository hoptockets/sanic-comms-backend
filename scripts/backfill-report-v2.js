/* eslint-disable no-console */
/**
 * Backfill report v2 fields for compliance-first moderation rollout.
 *
 * Usage:
 *   MONGO_URI="mongodb://localhost:27017/revolt" node scripts/backfill-report-v2.js
 */
const { MongoClient } = require("mongodb");

const uri = process.env.MONGO_URI || "mongodb://localhost:27017/revolt";
const [, dbNameFromUri] = uri.split("/").slice(-1);
const dbName = dbNameFromUri || "revolt";

async function run() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db(dbName);
  const col = db.collection("safety_reports");

  const now = new Date().toISOString();

  // 1) Normalize missing v2 fields.
  await col.updateMany(
    {},
    {
      $set: {
        severity: { $ifNull: ["$severity", "Medium"] },
        priority: { $ifNull: ["$priority", "Normal"] },
        breach_state: { $ifNull: ["$breach_state", false] },
      },
    },
  );

  // 2) Ensure timestamps exist for old reports.
  await col.updateMany(
    { created_at: { $exists: false } },
    {
      $set: {
        created_at: now,
        last_transition_at: now,
      },
    },
  );

  // 3) Ensure array/object defaults.
  await col.updateMany(
    { related_report_ids: { $exists: false } },
    { $set: { related_report_ids: [] } },
  );

  const count = await col.countDocuments();
  console.log(`Backfill complete for ${count} safety_reports documents.`);

  await client.close();
}

run().catch((error) => {
  console.error("Backfill failed:", error);
  process.exit(1);
});

