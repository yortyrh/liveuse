# Live Use

Fecha: 2026-10-05. Firmware `0.4.0`. Host `0.3.0`. El firmware ya compilado en la rama es `0.3.0`; `0.4.0` añade `DOWN`, `UP`, `HSCROLL` y `HOTKEY`.

Live Use es computer use sobre la máquina real. El XIAO, enchufado por USB al Mac Mini, es ratón y teclado de esa máquina. Un daemon en C++23 en el Mini posee el CDC, captura la pantalla y escucha en la red local. Otro ordenador de la LAN usa el MCP. Quien está delante del Mini prueba y pone el nombre del dispositivo en la web.

## Nombre

| Qué | Valor |
| --- | --- |
| Proyecto y binario | `liveuse` |
| Título de la web y `<title>` | Live Use |
| Paquete npm de `apps/web` | `liveuse` |
| Target CMake | `liveuse` |
| `serverInfo.name` del MCP | `liveuse` |
| Directorio de runtime (macOS) | `~/Library/Application Support/liveuse/` |
| Socket local | `liveuse.sock`, dentro de ese directorio |
| Cookie | `liveuse_session` |
| Variable de los estáticos | `LIVEUSE_WEB_ROOT` |
| Clave de ejemplo en el cliente MCP | `liveuse` |

`liveuse` sin argumentos arranca el servidor. `init`, `token`, `user` y `password` no lo arrancan. La carpeta del repo no se renombra en este corte. El nombre USB del firmware tampoco.

## Alcance

Entra:

- Firmware 0.4.0: HID ratón, HID teclado y CDC. El contrato de control copia ratón, teclado y captura de PyAutoGUI.
- Daemon C++23 único: serie, MCP HTTP, API REST, estáticos de la SPA, CLI.
- SPA Vite + React para login, prueba manual y nombre del dispositivo.
- Captura del display principal detrás de una interfaz, con backend macOS en este corte.
- Sustituir los crates Rust y la app Tauri.

No entra:

- Unicode, varios monitores, TLS, puerto distinto de 7395.
- Alertas, cuadros de mensaje y gestión de ventanas. PyAutoGUI los tiene; este contrato no.
- Pausa global de 0,1 s ni fail-safe de esquina. Cada llamada lleva su `duration`. El origen sigue siendo la esquina superior izquierda y la Y crece hacia abajo, igual que en [Mouse Control](https://pyautogui.readthedocs.io/en/latest/mouse.html).
- Backends de captura de Windows y Linux. Solo el hueco para añadirlos.
- Serie, rutas de config y empaquetado fuera de macOS.
- Borrar el historial del firmware ESP32-C3. Sigue documentado como callejón sin salida.

## Arquitectura

```text
Cursor en otro PC de la LAN
    POST /mcp
    Authorization: Bearer <token>
            │
            ▼
daemon C++23 en el Mini, 0.0.0.0:7395
    ├─ /mcp      JSON-RPC stateless
    ├─ /api/*    REST, cookie de sesión
    ├─ /         estáticos de apps/web/dist
    ├─ CDC 115200 8N1
    └─ ScreenCapture::capture_primary()
            │
            ▼
XIAO SAMD21
    ├─ HID mouse
    ├─ HID keyboard
    └─ CDC
```

Tres puertas, un puerto. El token no abre la web. La cookie no abre `/mcp`. Sin credencial válida, HTTP 401.

| Ruta | Cliente | Auth |
| --- | --- | --- |
| `POST /mcp` | Cursor u otro cliente MCP | `Authorization: Bearer <token>` |
| `/api/*` | SPA del mismo origen | Cookie `liveuse_session` |
| `/` | Navegador | La SPA pide login; los estáticos son públicos |

Cursor remoto:

```json
{
  "mcpServers": {
    "liveuse": {
      "url": "http://<ip-del-mini>:7395/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

El token no se commitea. El README muestra ese JSON con placeholders.

HTTP va en claro en la LAN. Token y contraseña viajan sin cifrar. TLS queda fuera de este corte.

No hay transporte stdio. Los binarios `mcp-mouse-mcp` y `mcp-mouse-daemon` desaparecen. Queda `liveuse`.

## Piezas

| Pieza | Lenguaje | Ruta |
| --- | --- | --- |
| Firmware | C++ PlatformIO | `firmware/` |
| Daemon, CLI, serie, captura, MCP, REST | C++23, CMake | `host/` |
| SPA | Vite + React | `apps/web/` |

Se eliminan `crates/`, `apps/desktop/`, `Cargo.toml` y `Cargo.lock`. El workspace pnpm queda solo para `apps/web`. El README pasa a describir este diseño.

El host fija `CMAKE_CXX_STANDARD` en 23 y `CMAKE_CXX_STANDARD_REQUIRED` en ON. Un compilador sin C++23 no sirve. El firmware sigue el dialecto del core Seeed de PlatformIO: el toolchain del SAMD21 no es C++23.

Dependencias del host, por FetchContent: cpp-httplib y nlohmann/json. El PNG común usa stb_image_write. El hash de contraseña usa CommonCrypto (PBKDF2). No hay SDK MCP de terceros: esos servidores se adueñan del puerto, y este proceso tiene que servir también `/api` y la SPA.

## Firmware 0.4.0

USB compuesto en el mismo Type-C, dos interfaces HID más CDC:

- Ratón: el descriptor relativo actual.
- Teclado: descriptor boot de 8 bytes (modificador, reservado, hasta 6 keycodes). En la práctica cada informe lleva un solo keycode.
- CDC 115200 8N1, líneas terminadas en LF. CR se ignora.

El demo automático a los 2 s del `loop` se quita. `DEMO` sigue existiendo si alguien lo pide.

`firmware/VERSION` es `0.4.0`.

### Líneas CDC que se quedan

`HELP`, `STATUS`, `ECHO <texto>`, `MOVE <dx> <dy>`, `CLICK left|right|double`, `SCROLL <pasos>`, `DEMO circle|zigzag|both`. Límites de `MOVE` y `SCROLL`: ±2000. Una línea que no reconoce el parser se hace eco, como ahora.

`STATUS` responde exactamente:

```text
OK uptime_ms=<ms> fw=0.4.0
```

`HELP` lista también `TYPE`, `KEY`, `KDOWN`, `KUP`, `HOTKEY`, `MDOWN`, `MUP` y `HSCROLL`.

### TYPE

```text
TYPE <ascii>
```

ASCII imprimible `0x20`–`0x7E`, sin salto de línea. El mapa es US ANSI. Letras y dígitos coinciden en un layout ES o US. Los símbolos (`@`, `#`, `{`) siguen el teclado US. Cada carácter es pulsar y soltar, con Shift si el mapa lo pide.

`TYPE` y `KEY` son comandos distintos. `TYPE` escribe un texto. `KEY` pulsa una tecla o una combinación. El firmware valida el `TYPE` entero, teclea todos los caracteres y no imprime línea de éxito. No existe `TYPE done`: una respuesta por carácter, o por trozo, haría un viaje de ida y vuelta por tecla.

Error, antes de pulsar nada: `ERR TYPE bad`.

`TYPE` vacío o con un byte fuera de rango es `ERR TYPE bad`.

### KEY

```text
KEY <acorde>
```

Un toque: modificadores opcionales y exactamente una tecla, unidos por `+`. No hay mantener pulsado.

Modificadores, solo lado izquierdo. Alias aceptados y forma canónica:

| Alias | Canónico | Bit HID |
| --- | --- | --- |
| `cmd`, `command`, `meta` | `cmd` | Left GUI `0x08` |
| `ctrl`, `control` | `ctrl` | Left Control `0x01` |
| `alt`, `opt`, `option` | `alt` | Left Alt `0x04` |
| `shift` | `shift` | Left Shift `0x02` |

Teclas: `a`–`z`, `0`–`9`, `enter` (`return`), `tab`, `esc` (`escape`), `space`, `backspace`, `delete`, `up`, `down`, `left`, `right`, `f1`–`f12`, y la puntuación del mapa US (`minus`, `equal`, `leftbracket`, `rightbracket`, `backslash`, `semicolon`, `quote`, `grave`, `comma`, `dot`, `slash`).

Orden canónico del acorde: `cmd`, `ctrl`, `alt`, `shift`, tecla. Ejemplo: la entrada `shift+cmd+4` se responde como `KEY cmd+shift+4 done`.

Las letras del acorde se aceptan en cualquier caja y salen en minúscula. `KEY A` es la tecla `a`, sin Shift. La mayúscula se pide con el modificador (`KEY shift+a`) o con `TYPE A`.

Se rechaza, sin pulsar nada: acorde vacío, solo modificadores, dos teclas, modificador repetido, nombre desconocido.

Ejemplos válidos: `KEY enter`, `KEY cmd+space`, `KEY cmd+shift+4`.

### Buffer

El buffer de línea del firmware sigue en 96 bytes con el NUL. El host no manda una línea de más de 80 bytes. `TYPE ` ocupa 5, así que cada trozo lleva como máximo 75 caracteres.

## Contrato de control

El agente ve la misma superficie que [PyAutoGUI](https://pyautogui.readthedocs.io/en/latest/) en ratón, teclado y captura. El XIAO solo emite informes HID relativos. Posición, tamaño, movimiento absoluto, arrastre, búsqueda de imagen y color los calcula el daemon en el Mini.

Coordenadas: origen arriba a la izquierda, X hacia la derecha, Y hacia abajo. En una pantalla `W×H` el píxel inferior derecho es `(W-1, H-1)`. `on_screen(W, H)` es falso.

### Ratón

Equivalente a [Mouse Control Functions](https://pyautogui.readthedocs.io/en/latest/mouse.html).

| Operación | PyAutoGUI | Quién la resuelve |
| --- | --- | --- |
| Tamaño | `size()` | Daemon. Display principal. No usa el serie |
| Posición | `position()` | Daemon. Cursor del sistema, convertido a origen arriba-izquierda |
| Dentro de pantalla | `onScreen(x, y)` | Daemon. `0 ≤ x < W` y `0 ≤ y < H` |
| Mover absoluto | `moveTo(x, y, duration, tween)` | Daemon lee la posición, parte el camino y manda `MOVE` relativos |
| Mover relativo | `move(dx, dy, duration, tween)` | `MOVE` |
| Arrastrar | `dragTo` / `drag` | `DOWN`, movimientos, `UP`. `button`: `left`, `middle`, `right` |
| Click | `click`, `doubleClick`, `tripleClick`, `rightClick` | `x` e `y` opcionales mueven antes. `button` `left` por defecto. `clicks` y `interval` en segundos |
| Bajar / subir botón | `mouseDown` / `mouseUp` | `DOWN` / `UP`. El botón se queda pulsado hasta `UP` |
| Rueda | `scroll(clicks)` | Positivo sube. `HSCROLL` es la rueda horizontal: positivo a la derecha |

`duration` va en segundos. `0` es instantáneo. No hay pausa extra entre llamadas.

`tween`, por defecto lineal. Nombres admitidos, con el mismo sentido que PyAutoGUI: `linear`, `easeInQuad`, `easeOutQuad`, `easeInOutQuad`, `easeInBounce`, `easeOutElastic`, `easeInElastic`. El daemon muestrea el camino y el firmware solo ve deltas.

Un solo `MOVE` del firmware sigue en ±2000. Un desplazamiento mayor se parte en varios `MOVE`. El descriptor del ratón incluye botón medio y rueda horizontal.

CDC nuevo, además de `MOVE`, `CLICK`, `SCROLL` y `DEMO`. `left` es a la vez un botón y una flecha, así que el verbo distingue el dispositivo:

- `MDOWN left|right|middle` y `MUP left|right|middle`
- `KDOWN <tecla>` y `KUP <tecla>`
- `HSCROLL <pasos>` con el mismo límite ±2000
- `HOTKEY <k1+k2+…>`

`CLICK` se queda como atajo de un golpe. `double` y `triple` los compone el daemon con `MDOWN`/`MUP` e `interval`.

### Teclado

Equivalente a [Keyboard Control Functions](https://pyautogui.readthedocs.io/en/latest/keyboard.html).

| Operación | PyAutoGUI | CDC |
| --- | --- | --- |
| Escribir texto | `write(text, interval)` | `TYPE`. `interval` 0: un `TYPE` por trozo, sin línea de éxito. `interval` > 0: el daemon separa los caracteres con esa pausa, en segundos |
| Pulsar y soltar | `press(key, presses, interval)` | `KEY` |
| Mantener | `keyDown` / `keyUp` / `hold` | `KDOWN` y `KUP`. `hold` no es un comando: es `KDOWN` y luego `KUP` |
| Atajo | `hotkey(k1, k2, …)` | `HOTKEY k1+k2+…`. El firmware pulsa en ese orden y suelta al revés. Una sola línea, sin `done` por tecla |

`TYPE` sigue sin línea de éxito. `KEY` y `HOTKEY` contestan `KEY <canónico> done` y `HOTKEY <canónico> done`, o `ERR KEY bad` / `ERR HOTKEY bad`, sin haber pulsado nada.

Nombres de tecla: los de `KEYBOARD_KEYS` en esa página que tengan uso HID. Modificadores izquierdos y derechos (`ctrl`, `ctrlleft`, `ctrlright`, `alt`, `option`, `shift`, `command`, `win` y sus lados). Letras, dígitos, puntuación US, `enter`, `return`, `tab`, `esc`, `escape`, `space`, `backspace`, `delete`, `del`, flechas, `home`, `end`, `pageup`, `pagedown`, `pgup`, `pgdn`, `insert`, `f1`–`f12`. Un nombre de esa lista sin uso HID (`browserback`, `volumeup`, `num0`, …) es 400 y no sale al serie. `write` solo acepta un carácter imprimible por posición; una tecla con nombre (`esc`, `f1`) va por `press` o `hotkey`.

### Captura

Equivalente a [Screenshot Functions](https://pyautogui.readthedocs.io/en/latest/screenshot.html). No usa el serie. Display principal. El backend entrega BGRA; el resto es común.

| Operación | PyAutoGUI | Resultado |
| --- | --- | --- |
| Captura | `screenshot()` y `screenshot(region=(left, top, width, height))` | PNG. Sin `region`, pantalla entera. `region` fuera de rango es 400 |
| Píxel | `pixel(x, y)` | `{ "red", "green", "blue" }`. Fuera de rango es 400 |
| Color | `pixelMatchesColor(x, y, (r, g, b), tolerance)` | `{ "match": true/false }`. `tolerance` es entero ≥ 0 por canal. Por defecto 0 |
| Buscar | `locateOnScreen`, `locateCenterOnScreen`, `locateAllOnScreen` | Caja `{ "left", "top", "width", "height" }` o centro `{ "x", "y" }`. `locateAll` devuelve la lista. Si no está, error `image not found` |
| Buscar en otra imagen | `locate`, `locateAll` | La aguja y el pajar son PNG. El mismo contrato de caja |

`grayscale` true compara en gris. `confidence` va de 0 a 1. `1` exige igualdad. Un valor menor permite, en cada canal, una diferencia de hasta `round(255 * (1 - confidence))`. No hay dependencia de OpenCV. `region` limita la búsqueda.

La aguja llega en base64 dentro del JSON. No se lee un path del disco del cliente.

## Configuración

Directorio de runtime en macOS: `~/Library/Application Support/liveuse/`.

`config.json`, modo `0600`:

```json
{
  "name": "mini",
  "username": "ada",
  "password_hash": "pbkdf2-sha256$100000$<salt_b64>$<hash_b64>",
  "token": "<64 hex>"
}
```

- `name`: texto que muestra la SPA y el `serverInfo` del MCP. La web solo puede cambiar este campo.
- `username`: login de la web.
- `password_hash`: PBKDF2-HMAC-SHA256, 100000 iteraciones, sal de 16 bytes, salida de 32 bytes. Salt y hash en base64 estándar.
- `token`: 32 bytes de `SecRandomCopyBytes`, escritos en hex. 64 caracteres.

Sin `config.json` el daemon no abre el 7395 e indica que hay que correr `init`. No hay usuario por defecto.

El primer arranque de `init` crea el directorio y el fichero. `init` no pisa un config existente salvo `--force`.

El socket local `liveuse.sock` en ese directorio no sale a la LAN. Solo lo usa la CLI para `reload`.

## CLI

Mismo binario, `liveuse`. La contraseña se lee por stdin, sin eco, y no se acepta como argumento.

| Comando | Efecto |
| --- | --- |
| `liveuse` | Arranca el servidor. |
| `liveuse init --user <nombre>` | Pide la contraseña, genera token y `name` igual al username, escribe `config.json`. |
| `liveuse token rotate` | Nuevo token de 32 bytes. |
| `liveuse user set <nombre>` | Cambia `username`. |
| `liveuse password set` | Pide la contraseña y reescribe `password_hash`. |

Después de escribir, si `liveuse.sock` acepta conexión, la CLI llama `reload`. Si el daemon no está, el fichero lo lee el siguiente arranque. `reload` vuelve a leer `config.json` y sustituye nombre, usuario, hash y token en memoria. No cierra el puerto serie. Las sesiones de cookie anteriores se invalidan.

## HTTP

Escucha `0.0.0.0:7395`. Si el puerto está ocupado, el proceso sale. No hay segundo puerto.

La URL que ve la SPA usa la primera IPv4 que no sea loopback. Si no hay, `127.0.0.1`. Forma: `http://<ip>:7395/mcp`.

Cookie de sesión: nombre `liveuse_session`, valor de 32 bytes en hex, `Path=/`, `HttpOnly`, `SameSite=Lax`, sin `Secure` (el navegador no la enviaría por HTTP). Vive en memoria 12 horas. Reiniciar el daemon cierra las sesiones. El login fallido responde siempre `credenciales invalidas`, tanto si falla el usuario como la contraseña. No se envían cabeceras CORS. La SPA y el daemon comparten origen.

Comparación del token en tiempo constante. Verificar la contraseña calcula el PBKDF2 y compara el hash en tiempo constante. Token, contraseña y hash no se escriben en el log.

Estáticos: directorio `web/` junto al binario, o `LIVEUSE_WEB_ROOT` si está definida. Si falta, `/` responde un texto corto con esa ruta, y `/api` y `/mcp` siguen funcionando. En desarrollo, Vite hace proxy de `/api` y `/mcp` a `127.0.0.1:7395`.

### REST

Todas las rutas bajo `/api` exigen cookie, menos `POST /api/login`.

| Método y ruta | Cuerpo | Respuesta |
| --- | --- | --- |
| `POST /api/login` | `{ "username", "password" }` | 200 y `Set-Cookie`, o 401 |
| `POST /api/logout` | | 200, borra la cookie |
| `GET /api/session` | | `{ "username", "name" }` |
| `GET /api/config` | | `{ "name", "mcpUrl", "token" }` |
| `PUT /api/config` | `{ "name" }` | El mismo objeto. Ignora cualquier otro campo |
| `GET /api/ports` | | `{ "ports": [{ "name", "kind" }] }` |
| `POST /api/connect` | `{ "port" }` | `{ "port", "banner" }` |
| `POST /api/disconnect` | | `{ "disconnected", "port" }` |
| `GET /api/status` | | `{ "connected", "port", "firmware" }`. `firmware` es la línea `STATUS` o `null` |
| `GET /api/mouse/size` | | `{ "width", "height" }` |
| `GET /api/mouse/position` | | `{ "x", "y" }` |
| `GET /api/mouse/on-screen` | query `x`, `y` | `{ "onScreen" }` |
| `POST /api/mouse/move-to` | `{ "x", "y", "duration", "tween" }` | `{ "lines" }` |
| `POST /api/mouse/move` | `{ "dx", "dy", "duration", "tween" }` | `{ "lines" }` |
| `POST /api/mouse/drag-to` | `{ "x", "y", "duration", "tween", "button" }` | `{ "lines" }` |
| `POST /api/mouse/drag` | `{ "dx", "dy", "duration", "tween", "button" }` | `{ "lines" }` |
| `POST /api/mouse/click` | `{ "x", "y", "button", "clicks", "interval" }` | `{ "lines" }`. `button`: `left`, `middle`, `right` |
| `POST /api/mouse/down` | `{ "x", "y", "button" }` | `{ "lines" }` |
| `POST /api/mouse/up` | `{ "x", "y", "button" }` | `{ "lines" }` |
| `POST /api/mouse/scroll` | `{ "clicks", "x", "y" }` | `{ "lines" }` |
| `POST /api/mouse/hscroll` | `{ "clicks", "x", "y" }` | `{ "lines" }` |
| `POST /api/mouse/demo` | `{ "pattern" }` | `{ "lines" }`. `pattern`: `circle`, `zigzag` o `both` |
| `POST /api/echo` | `{ "text" }` | `{ "lines" }` |
| `POST /api/help` | | `{ "lines" }` |
| `POST /api/keyboard/write` | `{ "text", "interval" }` | `{ "lines", "chunks" }` |
| `POST /api/keyboard/press` | `{ "key", "presses", "interval" }` | `{ "lines" }` |
| `POST /api/keyboard/down` | `{ "key" }` | `{ "lines" }` |
| `POST /api/keyboard/up` | `{ "key" }` | `{ "lines" }` |
| `POST /api/keyboard/hotkey` | `{ "keys": ["ctrl", "c"] }` | `{ "lines" }` |
| `POST /api/screenshot` | `{ "region": [left, top, width, height] }` opcional | `image/png`, cabeceras `X-Image-Width` y `X-Image-Height` |
| `GET /api/pixel` | query `x`, `y` | `{ "red", "green", "blue" }` |
| `GET /api/pixel-matches` | query `x`, `y`, `red`, `green`, `blue`, `tolerance` | `{ "match" }` |
| `POST /api/locate` | `{ "needle", "haystack", "grayscale", "confidence", "region" }` | Caja, centro o lista. `haystack` vacío busca en la pantalla |
| `POST /api/command` | `{ "line" }` | `{ "lines" }`. Una línea CDC cruda, ≤ 80 bytes, sin validar el mapa de teclas. El mismo escape que `send_line` |

`lines` es un array de strings con lo que contestó el CDC. Errores REST, cuerpo `{ "error": "<texto>" }`:

| Código | Cuándo |
| --- | --- |
| 400 | Argumento inválido, tecla sin uso HID, o `region` fuera de rango. No se escribe en el serie |
| 404 | `image not found` |
| 401 | Sin cookie o login mal |
| 409 | Ratón o teclado con el serie cerrado |
| 502 | El firmware contestó `ERR` |
| 503 | Captura sin permiso, sin display, o sin backend |

Un `TYPE` inválido es 400 y no sale al serie. Si el firmware contesta `ERR TYPE bad`, el REST es 502. No hay campo `completed`.

El serie atiende una transacción CDC a la vez. Las demás esperan. No se mezclan líneas.

### MCP

`POST /mcp` con cuerpo JSON-RPC. El cliente puede mandar `Accept: application/json, text/event-stream`. El servidor responde siempre un solo JSON, `Content-Type: application/json`. No abre SSE y no exige `Mcp-Session-Id`.

Métodos: `initialize`, `tools/list`, `tools/call`, `ping`. Cualquier otro método es error JSON-RPC. Un fallo de dispositivo, con el Bearer ya válido, es HTTP 200 y resultado de tool con `isError: true`. El 401 no llega a JSON-RPC.

`initialize` anuncia `serverInfo.name` `liveuse`, la versión del host y, en las instrucciones, el nombre del dispositivo leído del config. Las instrucciones dicen que hay que conectar antes de mover, pulsar o teclear, y que tamaño, posición, captura, píxel y búsqueda no usan el serie.

Tools:

| Tool | Argumentos | Notas |
| --- | --- | --- |
| `list_serial_ports` | | |
| `connect_device` | `port` | |
| `disconnect_device` | | |
| `device_status` | | |
| `mouse_size` | | No usa el serie |
| `mouse_position` | | No usa el serie |
| `mouse_on_screen` | `x`, `y` | No usa el serie |
| `mouse_move_to` | `x`, `y`, `duration`, `tween` | |
| `mouse_move` | `dx`, `dy`, `duration`, `tween` | |
| `mouse_drag_to` | `x`, `y`, `duration`, `tween`, `button` | |
| `mouse_drag` | `dx`, `dy`, `duration`, `tween`, `button` | |
| `mouse_click` | `x`, `y`, `button`, `clicks`, `interval` | |
| `mouse_down` | `x`, `y`, `button` | |
| `mouse_up` | `x`, `y`, `button` | |
| `mouse_scroll` | `clicks`, `x`, `y` | Positivo sube |
| `mouse_hscroll` | `clicks`, `x`, `y` | Positivo a la derecha |
| `mouse_demo` | `pattern`: `circle`, `zigzag`, `both` | |
| `keyboard_write` | `text`, `interval` | ASCII imprimible, 1–2000. Sin `TYPE done` |
| `keyboard_press` | `key`, `presses`, `interval` | |
| `keyboard_key_down` | `key` | |
| `keyboard_key_up` | `key` | |
| `keyboard_hotkey` | `keys` array | |
| `serial_echo` | `text` una línea | |
| `firmware_help` | | |
| `send_line` | `line` una línea, ≤ 80 bytes | |
| `screenshot` | `region` opcional `[left, top, width, height]` | PNG en el resultado, `image/png` |
| `pixel` | `x`, `y` | |
| `pixel_matches_color` | `x`, `y`, `red`, `green`, `blue`, `tolerance` | |
| `locate_on_screen` | `needle`, `grayscale`, `confidence`, `region` | `needle` es PNG en base64 |
| `locate_center_on_screen` | los mismos | |
| `locate_all_on_screen` | los mismos | |
| `locate` | `needle`, `haystack`, `grayscale`, `confidence` | Los dos son PNG en base64 |
| `locate_all` | los mismos | |

`duration` y `interval` son segundos. `tween` es opcional. Las tools de ratón y teclado que mandan HID exigen `connect_device`. Tamaño, posición, `on_screen`, captura, píxel y búsqueda no.

## Captura de pantalla

Quien pide la captura recibe un PNG. El sistema operativo solo entrega píxeles.

```text
screenshot / pixel / locate
        │
        ▼
ScreenCapture::capture(region)
        │
        ├─ host/src/capture_macos.mm    Apple, este corte
        └─ host/src/capture_stub.cpp    cualquier otro sistema, este corte
        │
        ▼
PixelBuffer   BGRA, ancho, alto, stride
        │
        ▼
encode_png()
```

CMake compila un solo backend. `capture_stub.cpp` devuelve exactamente `screen capture is not implemented on this OS`. Un Windows o un Linux futuros sustituyen el stub por su archivo en esa rama de CMake. Ni el MCP ni la SPA nombran una API de captura.

Este corte implementa macOS: display principal, permiso de grabación de pantalla. Sin permiso o sin display, error y ningún PNG vacío. Varios monitores no entran.

## SPA

`apps/web`, paquete npm `liveuse`. El título visible y el `<title>` son Live Use. Vite en desarrollo hace proxy. El build `dist/` es lo que el daemon sirve como `web/`.

Pantallas:

- Login: usuario y contraseña.
- Panel, con la sesión abierta:
  - Nombre editable. Guardar llama a `PUT /api/config`.
  - URL del MCP y token, solo lectura, con copiar. Rotar el token no está en la web.
  - Puertos, conectar, soltar, estado.
  - Mover absoluto y relativo, arrastrar, click, bajar y subir botón, rueda vertical y horizontal.
  - Escribir, pulsar, mantener y atajo.
  - Capturar, con región opcional. Muestra el PNG. No exige serie abierto.
  - Log de líneas enviadas y recibidas.

Un 401 manda al login. El cliente HTTP de la SPA es la única lógica con test de Vitest.

## Errores de firmware y de host

El firmware no pulsa teclas si el comando ya es `ERR TYPE` o `ERR KEY`. Una línea más larga que su buffer responde `ERR line too long` y descarta la línea.

El host rechaza, antes de escribir en el serie, un acorde mal formado, un `TYPE` vacío, Unicode, un salto de línea, o un `MOVE` / `SCROLL` fuera de ±2000.

## Tests

Sin placa y sin pantalla real. CTest en `host/tests`:

- Formato CDC: `TYPE`, `KEY` canónico, alias, rechazo de dos teclas y de solo modificadores, troceo a 75 caracteres, tope 2000, rechazo de Unicode y de saltos de línea, mapa US ANSI de letras, dígitos y de `@` en layout US.
- Config: el hash verifica la contraseña correcta y rechaza otra, el token compara igual y distinto, falta de `config.json` impide escuchar.
- HTTP en un puerto efímero, con config de prueba: 401 en `/mcp` sin Bearer y con Bearer mal, 401 en `/api` sin cookie, login que deja cookie, `tools/list` contiene `mouse_move_to`, `mouse_drag`, `keyboard_hotkey`, `keyboard_key_down`, `screenshot` y `locate_on_screen`.
- Búsqueda: una aguja conocida dentro de un `PixelBuffer` devuelve la caja; si no está, `image not found`. `pixel_matches_color` respeta `tolerance`.
- `encode_png` de un `PixelBuffer` conocido produce un PNG decodificable del mismo tamaño.
- El backend ausente devuelve `screen capture is not implemented on this OS`.

Vitest, el archivo de test junto al módulo probado: el cliente HTTP convierte un 401 en sesión caducada y enseña el `error` de un 400 o 502.

El firmware se prueba con la placa: `TYPE`, `KEY cmd+space`, `HELP`, `STATUS` con `fw=0.4.0`, y que al abrir el CDC no corre el demo solo. Eso no entra en CI.

## Orden de implementación

1. Firmware: segundo HID, `TYPE`, `KEY`, `STATUS` con versión, sin demo de arranque.
2. Host: config, CLI, HTTP, login y cookie.
3. Serie y las operaciones de ratón que ya existen.
4. Teclado en el host, con troceo.
5. Interfaz de captura, `encode_png` y backend macOS.
6. MCP stateless encima de las mismas operaciones.
7. REST y estáticos.
8. SPA.
9. Quitar Rust, Tauri y reescribir el README.
