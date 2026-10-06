export class SessionExpired extends Error {
  constructor() {
    super("sesion caducada");
  }
}

export class ApiError extends Error {
  constructor(message: string) {
    super(message);
  }
}

export async function api(path: string, init?: RequestInit): Promise<Response> {
  const headers = new Headers(init?.headers);
  if (init?.body && !headers.has("Content-Type")) headers.set("Content-Type", "application/json");
  const response = await fetch(path, { ...init, headers, credentials: "include" });
  if (response.status === 401 && path !== "/api/login") throw new SessionExpired();
  return response;
}

export async function apiJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await api(path, init);
  const body = (await response.json().catch(() => ({}))) as { error?: string };
  if (!response.ok) throw new ApiError(body.error ?? response.statusText);
  return body as T;
}
