import { mutation, query } from "./_generated/server";
import { v } from "convex/values";
import { logSettings, logSettingsId, logSettingsScope } from "./schema";

export const list = query({
  args: {},
  returns: v.array(v.object(logSettings)),
  handler: async (ctx) => {
    const rows = await ctx.db.query(logSettingsId).collect();
    return rows.map(({ scope, console, file }) => ({ scope, console, file }));
  },
});

export const set = mutation({
  args: {
    scope: logSettingsScope,
    console: v.optional(v.string()),
    file: v.optional(v.string()),
  },
  returns: v.null(),
  handler: async (ctx, args) => {
    const existing = await ctx.db
      .query(logSettingsId)
      .withIndex("by_scope", (q) => q.eq("scope", args.scope))
      .unique();

    if (args.console === undefined && args.file === undefined) {
      if (existing) await ctx.db.delete(existing._id);
      return null;
    }

    const value = {
      scope: args.scope,
      console: args.console,
      file: args.file,
    };
    if (existing) await ctx.db.replace(existing._id, value);
    else await ctx.db.insert(logSettingsId, value);
    return null;
  },
});
