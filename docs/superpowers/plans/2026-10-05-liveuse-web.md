# Live Use Web Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Vite + React SPA named Live Use logs in, drives the same manual bench as the old desktop app, and edits the device name. The Rust crates and the Tauri app leave the tree.

**Architecture:** `apps/web` talks only to `/api` on the same origin. In dev, Vite proxies `/api` and `/mcp` to `127.0.0.1:7395`. The daemon serves `dist/` as `web/`. The page does not rotate the token and does not change the password.

**Tech Stack:** Vite 8, React 19, TypeScript, Vitest 3. Tests sit next to the module they test and run with `-- --run`.

## Global Constraints

- Package name is `liveuse`. Document title and the visible heading are `Live Use`.
- The SPA is the only JavaScript in the product. Do not add a second UI framework.
- A 401 from `/api` means the session is gone. The login error string from the server is shown as-is. The client does not say whether the user or the password was wrong.
- `PUT /api/config` sends only `{ "name" }`.
- The token is shown and can be copied. There is no button to rotate it.
- Mouse, keyboard, echo, help, and demo controls stay disabled until `GET /api/status` says `connected` is true. Screenshot stays enabled without a serial connection.
- Vitest files live beside the source file. The test command is `pnpm --filter liveuse test -- --run`.
- Do not commit a real token. The README shows the MCP JSON with placeholders.
- Remove `crates/`, `apps/desktop/`, `Cargo.toml`, and `Cargo.lock`. The repo directory name stays. The USB product string stays.

---

### Task 1: API client

**Files:**
- Create: `apps/web/package.json`
- Create: `apps/web/vite.config.ts`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/index.html`
- Create: `apps/web/src/api.ts`
- Test: `apps/web/src/api.test.ts`
- Modify: `package.json` (root scripts)

**Interfaces:**
- Consumes: the REST paths from the host plan
- Produces: `api.login`, `api.logout`, `api.session`, `api.config`, `api.saveName`, `api.ports`, `api.connect`, `api.disconnect`, `api.status`, `api.move`, `api.click`, `api.scroll`, `api.demo`, `api.echo`, `api.help`, `api.typeText`, `api.key`, `api.screenshot`, and `ApiError` with `status` and `message`. `screenshot` returns a blob URL string plus width and height from `X-Image-Width` and `X-Image-Height`.

- [ ] **Step 1: Write the failing test**

`apps/web/src/api.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import { ApiError, messageFromResponse } from "./api";

describe("messageFromResponse", () => {
  it("turns 401 into a signed-out error", () => {
    const error = messageFromResponse(401, { error: "credenciales invalidas" });
    expect(error).toBeInstanceOf(ApiError);
    expect(error.status).toBe(401);
    expect(error.message).toBe("credenciales invalidas");
    expect(error.signedOut).toBe(true);
  });

  it("shows the server error for 400 and 502", () => {
    const error = messageFromResponse(502, { error: "ERR TYPE bad" });
    expect(error.signedOut).toBe(false);
    expect(error.message).toBe("ERR TYPE bad");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter liveuse test -- --run`

Expected: fail because `apps/web` or `./api` does not exist.

- [ ] **Step 3: Add the package and the client**

`apps/web/package.json` name `liveuse`, private, `"type": "module"`, scripts `dev` = `vite`, `build` = `tsc && vite build`, `test` = `vitest`. Dependencies `react` and `react-dom` at `^19.1.0`. Dev dependencies match `apps/desktop/package.json` for Vite, Vitest, TypeScript, and the React plugin, without Tauri packages.

`vite.config.ts` uses `@vitejs/plugin-react` and proxies `/api` and `/mcp` to `http://127.0.0.1:7395`.

`index.html` sets `<title>Live Use</title>` and mounts `/src/main.tsx`.

`messageFromResponse` returns `ApiError`. `signedOut` is true only for status 401. `message` is the JSON `error` string when present, otherwise `HTTP <status>`. There is no `completed` field.

`api.login` `POST /api/login` with `{ username, password }` and `credentials: "include"`. The other methods send the cookie the same way. `api.saveName` `PUT /api/config` with only `{ name }`. `api.screenshot` reads the two headers and returns `{ url: URL.createObjectURL(blob), width, height }`.

Root `package.json` scripts become `"web": "pnpm --filter liveuse dev"` and `"test:web": "pnpm --filter liveuse test -- --run"`. Remove the `desktop` and `test:desktop` scripts.

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm install && pnpm --filter liveuse test -- --run`

Expected: 2 tests passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/package.json apps/web/vite.config.ts apps/web/tsconfig.json apps/web/index.html apps/web/src/api.ts apps/web/src/api.test.ts package.json pnpm-lock.yaml
git commit -m "$(cat <<'EOF'
feat: add the Live Use web client

401 drops the session and other API errors show the server text.

EOF
)"
```

---

### Task 2: Login and bench UI

**Files:**
- Create: `apps/web/src/bench.ts`
- Create: `apps/web/src/main.tsx`
- Create: `apps/web/src/App.tsx`
- Create: `apps/web/src/App.css`
- Test: `apps/web/src/bench.test.ts`

**Interfaces:**
- Consumes: the `api` object from Task 1
- Produces: a default export `App` from `App.tsx`. `bench.test.ts` tests pure helpers in `apps/web/src/bench.ts`: `canDrive(connected: boolean): boolean` and `mcpSnippet(url: string, token: string): string`.

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it } from "vitest";

import { canDrive, mcpSnippet } from "./bench";

describe("bench", () => {
  it("blocks the pointer until the serial port is open", () => {
    expect(canDrive(false)).toBe(false);
    expect(canDrive(true)).toBe(true);
  });

  it("builds the Cursor snippet without adding a scheme", () => {
    expect(mcpSnippet("http://10.0.0.8:7395/mcp", "abc")).toContain('"liveuse"');
    expect(mcpSnippet("http://10.0.0.8:7395/mcp", "abc")).toContain("Bearer abc");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter liveuse test -- --run`

Expected: fail to resolve `./bench`.

- [ ] **Step 3: Implement the helpers and the page**

`canDrive` returns its argument. `mcpSnippet` returns a JSON string of the Cursor config from the spec, with the given URL and `Authorization` value `Bearer <token>`.

`App.tsx` heading text is `Live Use`.

Logged out: a form with username and password. Submit calls `api.login`. On `ApiError`, show `error.message`.

Logged in, one page:

- Text field for the device name and a save button that calls `api.saveName`.
- Read-only MCP URL and token, each with a copy button. No rotate control.
- The snippet from `mcpSnippet` in a `<pre>`.
- Port `<select>` from `api.ports`, buttons connect and disconnect, a line that shows `port` when `connected` is true.
- `dx`, `dy`, move. Demo buttons `circle`, `zigzag`, `both`.
- Click buttons `left`, `right`, `double`. Scroll steps, up sends the positive integer, down sends the negation.
- Text field for `TYPE`, chord field for `KEY`, echo field, help button.
- Capture button, enabled when the session exists. Show the image from `api.screenshot`.
- A log list of outgoing commands and the returned `lines`.
- Mouse, keyboard, echo, help, and demo buttons use `disabled={!canDrive(status.connected)}`.

`main.tsx` renders `App` in `#root`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter liveuse test -- --run`

Expected: the `bench` tests and the `api` tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/main.tsx apps/web/src/App.tsx apps/web/src/App.css apps/web/src/bench.ts apps/web/src/bench.test.ts
git commit -m "$(cat <<'EOF'
feat: add the Live Use bench for manual control

The page copies the MCP token and leaves rotation to the CLI.

EOF
)"
```

---

### Task 3: Retire Rust and document Live Use

**Files:**
- Delete: `crates/`, `apps/desktop/`, `Cargo.toml`, `Cargo.lock`, `scripts/install-sidecar.sh`
- Modify: `README.md`
- Modify: `.gitignore`
- Modify: `package.json` if any desktop script remains

**Interfaces:**
- Consumes: the binary and routes from the host plan

- [ ] **Step 1: Replace the README**

`README.md` states that Live Use is computer use on the real machine. Document, in this order:

1. Flash: `cd firmware && pio run -t upload`. USB is mouse + keyboard + CDC. Lines include `TYPE` and `KEY`. Version `0.3.0`.
2. Host build: `cmake -S host -B host/build && cmake --build host/build`.
3. Tests: `ctest --test-dir host/build --output-on-failure` and `pnpm --filter liveuse test -- --run`.
4. First run on the Mini: `liveuse init --user <name>`, then `liveuse`.
5. Web: `pnpm --filter liveuse dev`, then the browser at the Vite URL. The daemon must already be listening.
6. The Cursor snippet with placeholders `http://<ip-del-mini>:7395/mcp` and `Bearer <token>`, server key `liveuse`.
7. CLI: `liveuse token rotate`, `liveuse user set`, `liveuse password set`.
8. The ESP32-C3 note stays: do not use C3 as the HID target.

- [ ] **Step 2: Remove the Rust tree**

Delete the directories and files listed above. `.gitignore` drops the Rust `target/` and `apps/desktop/src-tauri/binaries/` lines and ignores `host/build/`.

- [ ] **Step 3: Confirm the web tests still run**

Run: `pnpm --filter liveuse test -- --run`

Expected: pass.

Run: `cmake --build host/build && ctest --test-dir host/build --output-on-failure`

Expected: pass. This requires the host plan to be applied first.

- [ ] **Step 4: Commit**

```bash
git add -A README.md .gitignore package.json
git commit -m "$(cat <<'EOF'
feat: replace the Rust sidecar with the Live Use host and web app

The Mini is configured from the browser and from the liveuse CLI.

EOF
)"
```

Do not `git add` a file that contains a real token. `.cursor/mcp.json` stays untracked. Put the placeholder snippet in the README only.
