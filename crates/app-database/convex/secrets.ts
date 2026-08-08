import { mutation, query } from "./_generated/server";
import { v } from "convex/values";
import { secrets, secretsId, accountUserRef, accountPlaceRef } from "./schema";

export const list = query({
  args: {},
  returns: v.array(v.object(secrets)),
  handler: async (ctx) => {
    const rows = await ctx.db.query(secretsId).collect();
    return rows.map(
      ({ name, value, updatedAt, allowedUsers, allowedPlaces }) => ({
        name,
        value,
        updatedAt,
        allowedUsers,
        allowedPlaces,
      }),
    );
  },
});

export const set = mutation({
  args: {
    name: v.string(),
    value: v.string(),
    allowedUsers: v.optional(v.array(accountUserRef)),
    allowedPlaces: v.optional(v.array(accountPlaceRef)),
  },
  returns: v.null(),
  handler: async (ctx, args) => {
    const existing = await ctx.db
      .query(secretsId)
      .withIndex("by_name", (q) => q.eq("name", args.name))
      .unique();

    const now = BigInt(Date.now());
    const value = {
      name: args.name,
      value: args.value,
      updatedAt: now,
      allowedUsers: args.allowedUsers,
      allowedPlaces: args.allowedPlaces,
    };
    if (existing) await ctx.db.replace(existing._id, value);
    else await ctx.db.insert(secretsId, value);
    return null;
  },
});

export const remove = mutation({
  args: {
    name: v.string(),
  },
  returns: v.union(
    v.object({ ok: v.literal(true) }),
    v.object({ ok: v.literal(false) }),
  ),
  handler: async (ctx, args) => {
    const existing = await ctx.db
      .query(secretsId)
      .withIndex("by_name", (q) => q.eq("name", args.name))
      .unique();
    if (!existing) return { ok: false } as const;
    await ctx.db.delete(existing._id);
    return { ok: true } as const;
  },
});
