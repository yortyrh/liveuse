import { useEffect, useId, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { catalogModels, listLlmModels, type LlmModel, type LlmProvider, type LlmSettings } from "@/lib/backend";

export function ModelPicker({
  saved,
  providers,
  apiKey,
  onApiKey,
  onSave,
}: {
  saved: LlmSettings;
  providers: LlmProvider[];
  apiKey: string;
  onApiKey: (value: string) => void;
  onSave: (draft: { provider: string; baseUrl: string; model: string; apiKey: string }) => void;
}) {
  const listId = useId();
  const [provider, setProvider] = useState(saved.provider);
  const [baseUrl, setBaseUrl] = useState(saved.baseUrl);
  const [model, setModel] = useState(saved.model);
  const [catalog, setCatalog] = useState<LlmModel[]>([]);
  const [catalogNote, setCatalogNote] = useState("Loading models…");
  const [account, setAccount] = useState<LlmModel[]>([]);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [editing, setEditing] = useState(false);
  const [loadingAccount, setLoadingAccount] = useState(false);
  const [accountNote, setAccountNote] = useState("");
  const [endpointOpen, setEndpointOpen] = useState(false);

  useEffect(() => {
    setProvider(saved.provider);
    setBaseUrl(saved.baseUrl);
    setModel(saved.model);
  }, [saved.provider, saved.baseUrl, saved.model]);

  useEffect(() => {
    let alive = true;
    setCatalogNote("Loading models…");
    void catalogModels(provider)
      .then((models) => {
        if (alive) {
          setCatalog(models);
          setCatalogNote("");
        }
      })
      .catch(() => {
        if (alive) {
          setCatalog([]);
          setCatalogNote("Couldn't load the catalog.");
        }
      });
    return () => {
      alive = false;
    };
  }, [provider]);

  const selected = providers.find((item) => item.id === provider);
  const needsKey = selected?.needsKey ?? saved.needsKey;

  useEffect(() => {
    const option = providers.find((item) => item.id === saved.provider);
    if (option && saved.baseUrl.replace(/\/$/, "") !== option.baseUrl.replace(/\/$/, "")) {
      setEndpointOpen(true);
    }
  }, [providers, saved.provider, saved.baseUrl]);
  const dirty =
    provider !== saved.provider || baseUrl !== saved.baseUrl || model !== saved.model || apiKey.trim().length > 0;

  const suggestions = useMemo(
    () => mergeModels(catalog, account, editing ? model : ""),
    [catalog, account, model, editing],
  );
  const activeIndex = Math.min(active, Math.max(suggestions.length - 1, 0));

  function choose(id: string) {
    setModel(id);
    setEditing(false);
    setOpen(false);
  }

  function save(nextModel = model) {
    onSave({ provider, baseUrl, model: nextModel, apiKey });
  }

  const canSave = model.trim().length > 0 && (!needsKey || saved.hasApiKey || apiKey.trim().length > 0);
  const status = dirty
    ? "Not saved yet."
    : needsKey && !saved.hasApiKey
      ? "Add an API key. It stays in the app config on this Mac."
      : "Saved on this Mac.";

  return (
    <div className="grid gap-2">
      <div className="grid gap-2 md:grid-cols-[11rem_minmax(0,1fr)_minmax(0,12rem)_auto]">
        <select
          aria-label="Provider"
          className="h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
          value={provider}
          onChange={(event) => {
            const next = event.target.value;
            const option = providers.find((item) => item.id === next);
            setProvider(next);
            setAccount([]);
            setAccountNote("");
            if (option) {
              setBaseUrl(option.baseUrl);
            }
          }}
        >
          {providers.length === 0 ? <option value={provider}>{saved.provider}</option> : null}
          {providers.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
        <div className="relative">
          <Input
            role="combobox"
            aria-label="Model"
            aria-expanded={open}
            aria-controls={listId}
            aria-autocomplete="list"
            aria-activedescendant={open && suggestions[activeIndex] ? `${listId}-${activeIndex}` : undefined}
            value={model}
            placeholder="Model"
            onChange={(event) => {
              setModel(event.target.value);
              setEditing(true);
              setOpen(true);
              setActive(0);
            }}
            onFocus={() => setOpen(true)}
            onBlur={() => {
              window.setTimeout(() => setOpen(false), 120);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setOpen(true);
                setActive((index) => Math.min(index + 1, Math.max(suggestions.length - 1, 0)));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActive((index) => Math.max(index - 1, 0));
              } else if (event.key === "Enter" && open && suggestions[activeIndex]) {
                event.preventDefault();
                choose(suggestions[activeIndex].id);
              } else if (event.key === "Escape") {
                setOpen(false);
              }
            }}
          />
          {open ? (
            <ul
              id={listId}
              role="listbox"
              aria-label="Models"
              className="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground"
            >
              {suggestions.length === 0 ? (
                <li className="px-2 py-2 text-sm text-muted-foreground">
                  {catalogNote || "No catalog match. The typed id is still used."}
                </li>
              ) : (
                suggestions.map((item, index) => (
                  <li key={item.id} role="presentation">
                    <button
                      id={`${listId}-${index}`}
                      type="button"
                      role="option"
                      aria-selected={item.id === model}
                      className={
                        index === activeIndex
                          ? "flex w-full items-center justify-between gap-3 rounded-md bg-accent px-2 py-1.5 text-left text-sm"
                          : "flex w-full items-center justify-between gap-3 rounded-md px-2 py-1.5 text-left text-sm hover:bg-accent"
                      }
                      onMouseDown={(event) => event.preventDefault()}
                      onMouseEnter={() => setActive(index)}
                      onClick={() => choose(item.id)}
                    >
                      <span className="min-w-0">
                        <span className="block truncate">{item.name}</span>
                        {item.name !== item.id ? (
                          <span className="block truncate text-xs text-muted-foreground">{item.id}</span>
                        ) : null}
                      </span>
                      {item.image ? <span className="shrink-0 text-xs text-muted-foreground">Vision</span> : null}
                    </button>
                  </li>
                ))
              )}
            </ul>
          ) : null}
        </div>
        <Input
          type="password"
          aria-label="API key"
          autoComplete="off"
          placeholder={saved.hasApiKey ? "Key saved on this Mac" : "API key"}
          value={apiKey}
          onChange={(event) => onApiKey(event.target.value)}
        />
        <Button type="button" variant="outline" disabled={!canSave} onClick={() => save()}>
          Save
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-expanded={endpointOpen}
          aria-controls="llm-endpoint"
          onClick={() => setEndpointOpen((open) => !open)}
        >
          Endpoint
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={loadingAccount || (needsKey && !saved.hasApiKey && !apiKey.trim())}
          onClick={() => {
            setLoadingAccount(true);
            setAccountNote("");
            void listLlmModels(provider, baseUrl, apiKey)
              .then((models) => {
                setAccount(models);
                setAccountNote(
                  models.length === 0 ? "This account returned no chat models." : `Loaded ${models.length} models from this account.`,
                );
                setOpen(true);
              })
              .catch((error: unknown) => {
                setAccountNote(error instanceof Error ? error.message : String(error));
              })
              .finally(() => setLoadingAccount(false));
          }}
        >
          {loadingAccount ? "Loading" : "Load models"}
        </Button>
      </div>
      {endpointOpen ? (
        <Input
          id="llm-endpoint"
          aria-label="Endpoint"
          value={baseUrl}
          onChange={(event) => setBaseUrl(event.target.value)}
        />
      ) : null}
      <p className="text-xs text-muted-foreground" role="status">
        {accountNote || `${status} Search the model field. Vision models can see the screen.`}
      </p>
    </div>
  );
}

function mergeModels(catalog: LlmModel[], account: LlmModel[], query: string): LlmModel[] {
  const byId = new Map<string, LlmModel>();
  for (const model of account) {
    byId.set(model.id, model);
  }
  for (const model of catalog) {
    byId.set(model.id, model);
  }
  const needle = query.trim().toLowerCase();
  return [...byId.values()]
    .filter((model) => {
      if (!needle) {
        return true;
      }
      return model.id.toLowerCase().includes(needle) || model.name.toLowerCase().includes(needle);
    })
    .sort((left, right) => rank(left) - rank(right) || left.name.localeCompare(right.name))
    .slice(0, 12);
}

function rank(model: LlmModel) {
  if (model.image && model.tools) {
    return 0;
  }
  if (model.tools) {
    return 1;
  }
  if (model.image) {
    return 2;
  }
  return 3;
}
