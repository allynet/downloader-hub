import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type LogSettings } from "@/lib/api";
import { useAuthStore } from "@/stores/auth-store";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

const LABELS = {
  global: "Global",
  central: "Central",
  worker: "Worker",
  bot: "Bot",
  admin: "Admin",
} as const;

function SettingsCard({ settings }: { settings: LogSettings }) {
  const queryClient = useQueryClient();
  const readonly = useAuthStore((state) => state.me?.readonly ?? true);
  const [consoleFilter, setConsoleFilter] = useState(settings.console ?? "");
  const [fileFilter, setFileFilter] = useState(settings.file ?? "");
  const mutation = useMutation({
    mutationFn: () =>
      api.setLogSettings(settings.scope, {
        console: consoleFilter.trim() || null,
        file: fileFilter.trim() || null,
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["log-settings"] }),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>{LABELS[settings.scope]}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Console filter</span>
          <Input
            value={consoleFilter}
            onChange={(event) => setConsoleFilter(event.target.value)}
            placeholder="info,app_actions=debug"
            spellCheck={false}
            disabled={readonly}
          />
        </label>
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">File filter</span>
          <Input
            value={fileFilter}
            onChange={(event) => setFileFilter(event.target.value)}
            placeholder="warn,downloader_worker=trace"
            spellCheck={false}
            disabled={readonly}
          />
        </label>
        {mutation.isError && (
          <p className="text-sm text-destructive">
            {mutation.error instanceof Error
              ? mutation.error.message
              : "Unable to save log settings."}
          </p>
        )}
        <div className="flex items-center justify-between gap-3">
          <span className="text-xs text-muted-foreground">
            Empty fields inherit {settings.scope === "global" ? "startup settings" : "global settings"}.
          </span>
          <Button
            type="button"
            size="sm"
            disabled={readonly || mutation.isPending}
            onClick={() => mutation.mutate()}
          >
            {mutation.isPending ? "Saving..." : "Save"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function LoggingPage() {
  const settings = useQuery({
    queryKey: ["log-settings"],
    queryFn: api.listLogSettings,
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Logging</h1>
        <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
          EnvFilter directives apply independently to console and file output.
          Service settings override Global; clearing a field restores inheritance.
        </p>
      </div>
      {settings.isLoading ? (
        <p className="text-sm text-muted-foreground">Loading settings...</p>
      ) : settings.isError ? (
        <p className="text-sm text-destructive">Unable to load log settings.</p>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {settings.data?.map((setting) => (
            <SettingsCard
              key={`${setting.scope}:${setting.console}:${setting.file}`}
              settings={setting}
            />
          ))}
        </div>
      )}
    </div>
  );
}
