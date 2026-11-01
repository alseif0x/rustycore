# OpenCode local — RustyCore

Configuración de OpenCode del proyecto, tomada de `forever` b8330b1b5 (2026-10-17)
y adaptada a `3.4.3` (servidor `/home/server`, usuario `ubuntu`). No contiene credenciales: la autenticación
de los proveedores sigue en la configuración de OpenCode del usuario.

## Uso

Desde este checkout, ejecuta `opencode` o `opencode -c`. No hace falta un launcher
especial: el plugin prepara también los comandos del servidor compartido.
El agente inicial de sesiones nuevas es `orchestrator`; las sesiones existentes
conservan su agente y modelo hasta que el usuario los seleccione.

- Equipo (2026-10-17): `orchestrator` y `scout` DeepSeek V4.1 Flash; `general`
  DeepSeek V4 Pro `#high`; `explore` DeepSeek V4.1 Flash `#high`; `builder`
  GPT-6 Luna `#max` con la cuenta OpenAI/Codex. GLM-5.3 se retiró (límite Go de
  15 USD/mes); el revisor queda en una familia distinta del constructor.
- `builder` implementa; `general`, `explore` y `scout` son de solo lectura.
- No hay perfiles Sol en OpenCode. El análisis y la revisión quedan en OpenCode. Esta
  configuración no cambia una conversación Codex ni Claude Code: Codex sigue
  `.codex/` y `routes/codex.md`; Claude Code sigue solo Opus (`routes/claude.md`).
- Los perfiles integrados `build` y `plan` siguen disponibles para sesiones previas.
- Pasos máximos: `scout` 40, `general` 60, `explore` 80, `builder` 200; el
  coordinador no tiene límite. Goal conserva su ventana de 25 continuaciones;
  carga y registro no acreditan una ejecución autónoma completa.

## Coste, contexto y fallback (2026-10-17)

- Contexto efectivo: DeepSeek 262.144 tokens y Luna 272.000 (su tarifa sube por
  encima), para compactar hacia ~235k en lugar de ~900k. `tool_output` limita
  cada resultado de herramienta a 400 líneas / 32 KB.
- Los prompts ya no piden releer `AGENTS.md` (OpenCode lo inyecta siempre) y
  piden informes breves con rutas en lugar de logs.
- `policies` bloquea de forma absoluta push forzado, `reset --hard`, `clean -f`,
  borrado de stashes y la lectura/edición de `.env` y de las claves API de
  OpenCode. Las reglas por agente, incluidas las de `ask`, se conservan.
- OpenCode V2 no tiene fallback automático de proveedor. Con la cuota de Go
  agotada, `.opencode/model-route api` pasa los agentes DeepSeek a la API directa
  de DeepSeek (saldo propio) y `.opencode/model-route go` los devuelve; `status`
  muestra la ruta. Recarga el servicio compartido; una instancia `--standalone`
  debe reiniciarse. Luna no cambia.
- `CARGO_BUILD_JOBS` sigue en 1 por la política de validación de `AGENTS.md`.
- Consumo: `opencode stats --project . --models --cost`.

## Entorno

`project-environment.js` añade `~/.cargo/bin` al PATH y define defaults:

- Cargo: `<checkout>/target`, un job; el runner usa ese mismo target y un job.
- Temporales y caché Python: `~/.local/share/rustycore-opencode-work`.
- PROTOC: `3.4.3` no tiene `.protoc-version`; se usa `~/.local/protoc/bin/protoc`
  (si existiera un pin, `~/.local/share/rustycore-opencode-work/protoc-<pin>/bin/protoc`).
- `RUSTYCORE_CPP_REFERENCE_ROOT`: `/home/server/woltk-trinity-legacy`, la referencia
  C++ 3.4.3 completa de AGENTS.md (solo si existe).
- OpenSSL opcional del bot: headers locales 3.5.5 y enlaces a las bibliotecas
  3.5.5 instaladas; no implica compilación ni aceptación del bot.

Los valores explícitos del caller se conservan. Los permisos cubren las referencias
C++ de AGENTS.md (`/home/server/woltk-trinity-legacy` y `/home/server/azerothcore-wotlk-reference`,
esta última en lugar de la copia AzerothCore de `forever`), el directorio de trabajo de OpenCode, el registro de Cargo y los toolchains.
Retirados frente a `forever` (no existen en este servidor): el laboratorio
`rustycore-extractor-work`, `rustycore-extractors`, `forever-product-runs`, REA/Ghidra
(servidor MCP `rea` y su skill). No se
concede acceso general a todo el directorio personal. Las reglas de permisos
no son un sandbox contra comandos shell; AGENTS.md sigue gobernando el trabajo.

## Servicios y autoridad

El servidor MCP REA de `forever` no se incluye: no está instalado aquí. La autenticación de gh y del proveedor de modelos sigue
siendo la del usuario; no se copian claves a este proyecto.

Lee `AGENTS.md` y el checkpoint activo antes de continuar. Respeta la rama,
el trabajo sin commitear, las pausas de gameplay y el presupuesto de validación.
Esta preparación no inicia desarrollo, objetivos, servidores ni bases de datos,
ni acredita el build completo del código actual. Un push, merge, despliegue o
prueba live conserva su autoridad y sus gates propios.

Verificación: la del 2026-10-17 (`opencode debug agents`, prueba de humo) se hizo en
la máquina de `forever`; en este servidor OpenCode aún no está instalado y la
configuración adaptada no se ha ejecutado.
