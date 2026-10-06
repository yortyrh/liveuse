import { useEffect, useState } from "react";

import { ApiError, SessionExpired, api, apiJson } from "./api";

type ConfigView = { name: string; mcpUrl: string; token: string };
type Lines = { lines: string[] };

async function copy(text: string) {
  await navigator.clipboard.writeText(text);
}

function post(path: string, body: unknown) {
  return apiJson<Lines>(path, { method: "POST", body: JSON.stringify(body) });
}

export function App() {
  const [authed, setAuthed] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [config, setConfig] = useState<ConfigView | null>(null);
  const [name, setName] = useState("");
  const [log, setLog] = useState<string[]>([]);
  const [shot, setShot] = useState("");
  const [error, setError] = useState("");

  function note(line: string) {
    setLog((prev) => [line, ...prev].slice(0, 40));
  }

  async function run(label: string, action: () => Promise<unknown>) {
    setError("");
    try {
      const result = await action();
      note(`${label} ${JSON.stringify(result)}`);
    } catch (err) {
      if (err instanceof SessionExpired) {
        setAuthed(false);
        return;
      }
      const message = err instanceof ApiError ? err.message : "error";
      setError(message);
      note(`${label} ${message}`);
    }
  }

  async function load() {
    const session = await apiJson<{ username: string; name: string }>("/api/session");
    const next = await apiJson<ConfigView>("/api/config");
    setAuthed(true);
    setUsername(session.username);
    setConfig(next);
    setName(next.name);
  }

  useEffect(() => {
    load().catch(() => setAuthed(false));
  }, []);

  if (!authed) {
    return (
      <main className="sheet">
        <h1>Live Use</h1>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void run("login", async () => {
              await apiJson("/api/login", { method: "POST", body: JSON.stringify({ username, password }) });
              await load();
            });
          }}
        >
          <label>
            Usuario
            <input value={username} onChange={(event) => setUsername(event.target.value)} />
          </label>
          <label>
            Contraseña
            <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} />
          </label>
          <button type="submit">Entrar</button>
          {error ? <p className="error">{error}</p> : null}
        </form>
      </main>
    );
  }

  return (
    <main className="sheet">
      <header>
        <h1>Live Use · {username}</h1>
        <button
          type="button"
          onClick={() =>
            void run("logout", async () => {
              await apiJson("/api/logout", { method: "POST" });
              setAuthed(false);
            })
          }
        >
          Salir
        </button>
      </header>
      {error ? <p className="error">{error}</p> : null}
      <section>
        <h2>Dispositivo</h2>
        <label>
          Nombre
          <input value={name} onChange={(event) => setName(event.target.value)} />
        </label>
        <button type="button" onClick={() => void run("nombre", async () => {
          const next = await apiJson<ConfigView>("/api/config", { method: "PUT", body: JSON.stringify({ name }) });
          setConfig(next);
          return next;
        })}>
          Guardar
        </button>
        <p>MCP {config?.mcpUrl}</p>
        <p>Token {config?.token}</p>
        <button type="button" onClick={() => config && void copy(config.mcpUrl)}>Copiar URL</button>
        <button type="button" onClick={() => config && void copy(config.token)}>Copiar token</button>
      </section>
      <Bench />
      <section>
        <h2>Captura</h2>
        <button
          type="button"
          onClick={() =>
            void run("screenshot", async () => {
              const response = await api("/api/screenshot", { method: "POST", body: "{}" });
              if (!response.ok) {
                const body = (await response.json()) as { error?: string };
                throw new ApiError(body.error ?? response.statusText);
              }
              const blob = await response.blob();
              setShot(URL.createObjectURL(blob));
              return { width: response.headers.get("X-Image-Width"), height: response.headers.get("X-Image-Height") };
            })
          }
        >
          Capturar
        </button>
        {shot ? <img alt="Captura del display principal" src={shot} /> : null}
      </section>
      <section>
        <h2>Log</h2>
        <ul>
          {log.map((line, index) => (
            <li key={`${index}-${line}`}>{line}</li>
          ))}
        </ul>
      </section>
    </main>
  );
}

function Bench() {
  const [x, setX] = useState("100");
  const [y, setY] = useState("100");
  const [text, setText] = useState("hola");
  const [keyName, setKeyName] = useState("enter");
  const [chord, setChord] = useState("ctrl,c");
  const [error, setError] = useState("");

  async function go(label: string, path: string, body: unknown) {
    setError("");
    try {
      await post(path, body);
    } catch (err) {
      if (err instanceof SessionExpired) return;
      setError(err instanceof ApiError ? err.message : "error");
      void label;
    }
  }

  return (
    <section>
      <h2>Ratón y teclado</h2>
      {error ? <p className="error">{error}</p> : null}
      <label>
        X<input value={x} onChange={(event) => setX(event.target.value)} />
      </label>
      <label>
        Y<input value={y} onChange={(event) => setY(event.target.value)} />
      </label>
      <button type="button" onClick={() => void go("move", "/api/mouse/move-to", { x: Number(x), y: Number(y) })}>Mover</button>
      <button type="button" onClick={() => void go("move-rel", "/api/mouse/move", { dx: Number(x), dy: Number(y) })}>Mover relativo</button>
      <button type="button" onClick={() => void go("drag", "/api/mouse/drag-to", { x: Number(x), y: Number(y), button: "left" })}>Arrastrar</button>
      <button type="button" onClick={() => void go("click", "/api/mouse/click", { button: "left", clicks: 1 })}>Click</button>
      <button type="button" onClick={() => void go("down", "/api/mouse/down", { button: "left" })}>Bajar</button>
      <button type="button" onClick={() => void go("up", "/api/mouse/up", { button: "left" })}>Subir</button>
      <button type="button" onClick={() => void go("scroll", "/api/mouse/scroll", { clicks: 3 })}>Rueda</button>
      <button type="button" onClick={() => void go("hscroll", "/api/mouse/hscroll", { clicks: 3 })}>Rueda horizontal</button>
      <label>
        Texto
        <input value={text} onChange={(event) => setText(event.target.value)} />
      </label>
      <button type="button" onClick={() => void go("write", "/api/keyboard/write", { text, interval: 0 })}>Escribir</button>
      <label>
        Tecla
        <input value={keyName} onChange={(event) => setKeyName(event.target.value)} />
      </label>
      <button type="button" onClick={() => void go("press", "/api/keyboard/press", { key: keyName, presses: 1 })}>Pulsar</button>
      <button type="button" onClick={() => void go("down-key", "/api/keyboard/down", { key: keyName })}>Mantener</button>
      <button type="button" onClick={() => void go("up-key", "/api/keyboard/up", { key: keyName })}>Soltar</button>
      <label>
        Atajo
        <input value={chord} onChange={(event) => setChord(event.target.value)} />
      </label>
      <button type="button" onClick={() => void go("hotkey", "/api/keyboard/hotkey", { keys: chord.split(",").map((part) => part.trim()) })}>
        Atajo
      </button>
    </section>
  );
}
