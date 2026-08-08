import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { useAuthStore } from "@/stores/auth-store";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

export function SecretsPage() {
  const qc = useQueryClient();
  const readonly = useAuthStore((s) => s.me?.readonly ?? false);
  const list = useQuery({
    queryKey: ["secrets"],
    queryFn: api.listSecrets,
  });

  const [name, setName] = useState("");
  const [value, setValue] = useState("");

  const create = useMutation({
    mutationFn: () => api.setSecret({ name: name.trim(), value }),
    onSuccess: () => {
      setName("");
      setValue("");
      qc.invalidateQueries({ queryKey: ["secrets"] });
    },
  });

  function removeSecret(secretName: string) {
    if (window.confirm(`Delete secret "${secretName}"?`)) {
      remove.mutate(secretName);
    }
  }
  const remove = useMutation({
    mutationFn: (secretName: string) => api.removeSecret(secretName),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secrets"] }),
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Secrets</h1>
        <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
          Per-platform cookies used by workers for authenticated downloads
          (e.g. age-gated Instagram posts). Named by platform; workers match by
          URL host.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{list.data?.find((s) => s.name === name.trim()) ? "Update" : "Create"} secret</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            className="space-y-3"
            onSubmit={(e) => {
              e.preventDefault();
              if (name.trim() && value) create.mutate();
            }}
          >
            <label className="block space-y-1.5">
              <span className="text-sm font-medium">Name</span>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="instagram"
                spellCheck={false}
                disabled={readonly}
              />
            </label>
            <label className="block space-y-1.5">
              <span className="text-sm font-medium">Cookie value</span>
              <textarea
                value={value}
                onChange={(e) => setValue(e.target.value)}
                placeholder="sessionid=...; ds_user_id=..."
                spellCheck={false}
                disabled={readonly}
                className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
              />
            </label>
            {create.isError && (
              <p className="text-sm text-destructive">
                {create.error instanceof Error
                  ? create.error.message
                  : "Unable to save secret."}
              </p>
            )}
            <Button
              type="submit"
              disabled={readonly || create.isPending || !name.trim() || !value}
            >
              {create.isPending ? "Saving..." : "Save"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Stored secrets</CardTitle>
        </CardHeader>
        <CardContent>
          {list.isLoading ? (
            <p className="text-muted-foreground">Loading…</p>
          ) : list.data && list.data.length > 0 ? (
            <div className="space-y-3">
              {list.data.map((secret) => (
                <div
                  key={secret.name}
                  className="space-y-1 rounded-md border p-3"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium">{secret.name}</span>
                    <div className="flex items-center gap-1">
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={readonly}
                        onClick={() => {
                          setName(secret.name);
                          setValue(secret.value);
                        }}
                      >
                        Edit
                      </Button>
                      <Button
                        size="sm"
                        variant="destructive"
                        disabled={readonly || remove.isPending}
                        onClick={() => removeSecret(secret.name)}
                      >
                        Delete
                      </Button>
                    </div>
                  </div>
                  <code className="block break-all rounded-md bg-muted p-2 text-xs">
                    {secret.value}
                  </code>
                  <span className="text-xs text-muted-foreground">
                    Updated {new Date(Number(secret.updatedAt)).toLocaleString()}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-muted-foreground">No secrets stored.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
