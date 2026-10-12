import { useEffect, useId, useMemo, useState, type ReactNode } from "react";
import { Input } from "@/components/ui/input";
import { catalogModels, type LlmModel, type LlmProvider, type LlmSettings } from "@/lib/backend";

export function ModelPicker({
  saved,
  providers,
  providersNote = "Loading providers…",
  apiKey,
  formId = "llm-model-form",
  onApiKey,
  onCanSave,
  onSave,
}: {
  saved: LlmSettings;
  providers: LlmProvider[];
  providersNote?: string;
  apiKey: string;
  formId?: string;
  onApiKey: (value: string) => void;
  onCanSave?: (value: boolean) => void;
  onSave: (draft: { provider: string; baseUrl: string; model: string; apiKey: string }) => void;
}) {
  const companyListId = useId();
  const modelListId = useId();
  const [provider, setProvider] = useState(saved.provider);
  const [baseUrl, setBaseUrl] = useState(saved.baseUrl);
  const [model, setModel] = useState(saved.model);
  const [catalog, setCatalog] = useState<LlmModel[]>([]);
  const [catalogNote, setCatalogNote] = useState("Loading models…");
  const [companyOpen, setCompanyOpen] = useState(false);
  const [companyActive, setCompanyActive] = useState(0);
  const [companyEditing, setCompanyEditing] = useState(false);
  const [companyQuery, setCompanyQuery] = useState("");
  const [modelOpen, setModelOpen] = useState(false);
  const [modelActive, setModelActive] = useState(0);
  const [modelEditing, setModelEditing] = useState(false);

  useEffect(() => {
    setProvider(saved.provider);
    setBaseUrl(saved.baseUrl);
    setModel(saved.model);
    setCompanyEditing(false);
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
  const companyLabel = selected?.label ?? provider;

  const companyOptions = useMemo(
    () => filterNamed(providers.map((item) => ({ id: item.id, name: item.label })), companyEditing ? companyQuery : ""),
    [providers, companyEditing, companyQuery],
  );
  const modelOptions = useMemo(
    () => filterModels(catalog, modelEditing ? model : ""),
    [catalog, model, modelEditing],
  );
  const companyIndex = Math.min(companyActive, Math.max(companyOptions.length - 1, 0));
  const modelIndex = Math.min(modelActive, Math.max(modelOptions.length - 1, 0));

  function chooseCompany(id: string) {
    const option = providers.find((item) => item.id === id);
    setProvider(id);
    setCompanyEditing(false);
    setCompanyOpen(false);
    setCompanyQuery("");
    if (option) {
      setBaseUrl(option.baseUrl);
    }
  }

  function chooseModel(id: string) {
    setModel(id);
    setModelEditing(false);
    setModelOpen(false);
  }

  function save(nextModel = model) {
    onSave({ provider, baseUrl, model: nextModel, apiKey });
  }

  const canSave = model.trim().length > 0 && (!needsKey || saved.hasApiKey || apiKey.trim().length > 0);

  useEffect(() => {
    onCanSave?.(canSave);
  }, [canSave, onCanSave]);

  return (
    <form
      id={formId}
      className="grid gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (canSave) {
          save();
        }
      }}
    >
      <div className="grid gap-2 md:grid-cols-[minmax(0,16rem)_minmax(0,1fr)]">
        <div className="relative">
          <Input
            role="combobox"
            aria-label="Provider"
            aria-expanded={companyOpen}
            aria-controls={companyListId}
            aria-autocomplete="list"
            aria-activedescendant={companyOpen && companyOptions[companyIndex] ? `${companyListId}-${companyIndex}` : undefined}
            value={companyEditing ? companyQuery : companyLabel}
            placeholder="Provider"
            spellCheck={false}
            autoCapitalize="off"
            onChange={(event) => {
              setCompanyQuery(event.target.value);
              setCompanyEditing(true);
              setCompanyOpen(true);
              setCompanyActive(0);
            }}
            onFocus={(event) => {
              setCompanyEditing(false);
              setCompanyQuery("");
              setCompanyOpen(true);
              const index = providers.findIndex((item) => item.id === provider);
              setCompanyActive(index >= 0 ? index : 0);
              event.currentTarget.select();
            }}
            onBlur={() => {
              window.setTimeout(() => {
                setCompanyOpen(false);
                setCompanyEditing(false);
              }, 120);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setCompanyOpen(true);
                setCompanyActive((index) => Math.min(index + 1, Math.max(companyOptions.length - 1, 0)));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setCompanyActive((index) => Math.max(index - 1, 0));
              } else if (event.key === "Enter" && companyOpen && companyOptions[companyIndex]) {
                event.preventDefault();
                chooseCompany(companyOptions[companyIndex].id);
              } else if (event.key === "Escape") {
                setCompanyOpen(false);
                setCompanyEditing(false);
              }
            }}
          />
          {companyOpen ? (
            <OptionList id={companyListId} label="Providers">
              {providers.length === 0 ? (
                <li className="px-2 py-2 text-sm text-muted-foreground">{providersNote}</li>
              ) : companyOptions.length === 0 ? (
                <li className="px-2 py-2 text-sm text-muted-foreground">No provider match.</li>
              ) : (
                companyOptions.map((item, index) => (
                  <Option
                    key={item.id}
                    id={`${companyListId}-${index}`}
                    selected={item.id === provider}
                    active={index === companyIndex}
                    onHover={() => setCompanyActive(index)}
                    onChoose={() => chooseCompany(item.id)}
                    title={item.name}
                  />
                ))
              )}
            </OptionList>
          ) : null}
        </div>
        <div className="relative">
          <Input
            role="combobox"
            aria-label="Model"
            aria-expanded={modelOpen}
            aria-controls={modelListId}
            aria-autocomplete="list"
            aria-activedescendant={modelOpen && modelOptions[modelIndex] ? `${modelListId}-${modelIndex}` : undefined}
            value={model}
            placeholder="Model"
            spellCheck={false}
            autoCapitalize="off"
            onChange={(event) => {
              setModel(event.target.value);
              setModelEditing(true);
              setModelOpen(true);
              setModelActive(0);
            }}
            onFocus={(event) => {
              setModelEditing(false);
              setModelOpen(true);
              const index = catalog.findIndex((item) => item.id === model);
              setModelActive(index >= 0 ? index : 0);
              event.currentTarget.select();
            }}
            onBlur={() => {
              window.setTimeout(() => {
                setModelOpen(false);
                setModelEditing(false);
              }, 120);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setModelOpen(true);
                setModelActive((index) => Math.min(index + 1, Math.max(modelOptions.length - 1, 0)));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setModelActive((index) => Math.max(index - 1, 0));
              } else if (event.key === "Enter" && modelOpen && modelOptions[modelIndex]) {
                event.preventDefault();
                chooseModel(modelOptions[modelIndex].id);
              } else if (event.key === "Escape") {
                setModelOpen(false);
                setModelEditing(false);
              }
            }}
          />
          {modelOpen ? (
            <OptionList id={modelListId} label="Models">
              {modelOptions.length === 0 ? (
                <li className="px-2 py-2 text-sm text-muted-foreground">
                  {catalogNote || "No catalog match. The typed id is still used."}
                </li>
              ) : (
                modelOptions.map((item, index) => (
                  <Option
                    key={item.id}
                    id={`${modelListId}-${index}`}
                    selected={item.id === model}
                    active={index === modelIndex}
                    onHover={() => setModelActive(index)}
                    onChoose={() => chooseModel(item.id)}
                    title={item.name}
                    detail={item.name !== item.id ? item.id : undefined}
                    badge={item.image ? "Vision" : undefined}
                  />
                ))
              )}
            </OptionList>
          ) : null}
        </div>
      </div>
      <label className="grid gap-1.5">
        <span className="text-sm font-medium">API key</span>
        <Input
          type="password"
          aria-label="API key"
          autoComplete="off"
          placeholder={saved.hasApiKey ? "Key saved on this Mac" : "Paste an API key"}
          value={apiKey}
          onChange={(event) => onApiKey(event.target.value)}
        />
      </label>
      <label className="grid gap-1.5">
        <span className="text-sm font-medium">Endpoint</span>
        <Input
          id="llm-endpoint"
          aria-label="Endpoint"
          spellCheck={false}
          value={baseUrl}
          onChange={(event) => setBaseUrl(event.target.value)}
        />
        <span className="text-xs text-muted-foreground">
          Address for this company. Change it only for a proxy or a local server.
        </span>
      </label>
    </form>
  );
}

function OptionList({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <ul
      id={id}
      role="listbox"
      aria-label={label}
      className="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground"
    >
      {children}
    </ul>
  );
}

function Option({
  id,
  selected,
  active,
  title,
  detail,
  badge,
  onHover,
  onChoose,
}: {
  id: string;
  selected: boolean;
  active: boolean;
  title: string;
  detail?: string;
  badge?: string;
  onHover: () => void;
  onChoose: () => void;
}) {
  return (
    <li role="presentation">
      <button
        id={id}
        type="button"
        role="option"
        aria-selected={selected}
        className={optionClass(active)}
        onMouseDown={(event) => event.preventDefault()}
        onMouseEnter={onHover}
        onClick={onChoose}
      >
        <span className="min-w-0">
          <span className="block truncate">{title}</span>
          {detail ? <span className="block truncate text-xs text-muted-foreground">{detail}</span> : null}
        </span>
        {badge ? <span className="shrink-0 text-xs text-muted-foreground">{badge}</span> : null}
      </button>
    </li>
  );
}

function optionClass(active: boolean) {
  const layout = "flex w-full items-center justify-between gap-3 rounded-md px-2 py-1.5 text-left text-sm";
  if (active) {
    return `${layout} bg-accent`;
  }
  return `${layout} hover:bg-accent`;
}

function filterNamed(items: { id: string; name: string }[], query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return items;
  }
  return items.filter(
    (item) => item.id.toLowerCase().includes(needle) || item.name.toLowerCase().includes(needle),
  );
}

function filterModels(catalog: LlmModel[], query: string): LlmModel[] {
  const needle = query.trim().toLowerCase();
  return catalog
    .filter((item) => {
      if (!needle) {
        return true;
      }
      return item.id.toLowerCase().includes(needle) || item.name.toLowerCase().includes(needle);
    })
    .sort((left, right) => rank(left) - rank(right) || left.name.localeCompare(right.name));
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
