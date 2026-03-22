const dbx = db.getSiblingDB("revolt");
const targetEmail = "pierce@sanic.one";

const account = dbx.accounts.findOne({ email: targetEmail });
if (!account) {
  printjson({ ok: false, error: "account_not_found", email: targetEmail });
  quit(1);
}

const userId = account._id;

const result = dbx.users.updateOne(
  { _id: userId },
  {
    $set: {
      platform_admin_role: "PlatformOwner",
      privileged: true,
    },
    $addToSet: {
      platform_permissions: { $each: ["*", "feature:*"] },
    },
  },
);

const user = dbx.users.findOne(
  { _id: userId },
  {
    projection: {
      _id: 1,
      username: 1,
      platform_admin_role: 1,
      privileged: 1,
      platform_permissions: 1,
    },
  },
);

printjson({
  ok: true,
  matched: result.matchedCount,
  modified: result.modifiedCount,
  user,
});
