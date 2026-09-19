# Prompt de reanudación autónoma de RustyCore

Pega el siguiente bloque como primer mensaje de una sesión nueva de Codex abierta en
`/home/server/rustycore`:

```text
Continúa autónomamente el desarrollo de RustyCore desde el estado real del checkout.

Repositorio: /home/server/rustycore
Rama de integración: 3.4.3
Objetivo: paridad funcional completa con el servidor TrinityCore 3.4.3 de
/home/server/woltk-trinity-legacy, manteniendo el producto nativo/Wasm aprobado.

Antes de decidir nada, ejecuta:
  cd /home/server/rustycore
  git status --short --branch
  git log --oneline --decorate -8
  sed -n '1,100p' docs/migration/STATE.md
  sed -n '1,180p' docs/migration/PORT_PLAN.md
  ps -eo pid,etime,stat,cmd | rg 'cargo|validation-v2|rustc|world-server|bnet-server|claude|codex' | rg -v 'rg ' || true

Lee AGENTS.md y las skills aplicables antes de editar. Las fuentes de comportamiento son
el C++ exacto y las capturas de la versión objetivo; el Rust existente no demuestra paridad.
Conserva trabajo ajeno y no hagas una reauditoría general.

Orquestación vigente:
- Sol medium es el agente padre y dueño de arquitectura, decisiones, Git, integración,
  documentación de estado y aceptación final.
- Luna max es el único subagente permitido, solo para una responsabilidad independiente
  y acotada. No uses Astra, Spark, Claude ni MiMo.
- Como máximo un subagente a la vez. El padre no delega Git, publicación, merge ni QA viva.
- No abras una issue, rama o PR por cada helper: una macro-issue, una entrega coherente,
  una rama y una PR dirigida a 3.4.3.

Estado conocido al preparar este prompt:
- HEAD local: bf5248a0, commit de configuración del orquestador Sol/Luna; está un commit
  por delante de origin/3.4.3 y el árbol estaba limpio.
- #29 sigue abierta. Las entregas de melee ya integradas cubren roll de daño, armadura,
  hit table, block, victim-side damage taken, auras de avoidance, absorb de escuela,
  mana shield, logs visibles, inmunidad de daño, resistencia física, bonuses del atacante
  criatura y la fidelidad del crushing band negativo.
- El siguiente límite registrado es SPELL_AURA_SPLIT_DAMAGE_PCT / SHARE_DAMAGE_PCT.
  No lo implementes como un multiplicador local: C++ Unit::CalcAbsorbResist y
  Unit::DealDamage crean daño secundario con selección de caster vivo, máscara de escuela,
  inmunidad, DealDamageMods, orden de logs, proc y publicación propios. Primero traza
  propietarios, targets Player/Creature y orden exacto en Rust y C++.
- #31 está cerrada administrativamente en GitHub, pero sus límites no deben desaparecer
  del port: spell-side absorb/crit, DoT/HoT, efectos de criatura y fan-out de logs siguen
  siendo alcance pendiente si PORT_PLAN.md/STATE.md los mantienen abiertos.
- La última auditoría delegada de split damage se interrumpió; no la trates como evidencia.

Selecciona la siguiente entrega por dependencias, valor funcional y riesgo demostrado.
Si split damage no tiene todavía una unidad completa y segura, documenta la frontera con
anchors C++ y elige la siguiente responsabilidad preparada del PORT_PLAN.md sin inventar
una arquitectura. No cierres una issue por documentación o tests aislados.

Para cada entrega completa:
1. Identifica la operación completa, sus consumidores y el único propietario de cada estado.
2. Compara las funciones C++ exactas, datos, orden, cancelación, persistencia y publicación.
3. Implementa el cambio mínimo coherente, incluyendo consumidores productivos y regresiones
   positivas, negativas y de lifetime/fallo que el contrato requiera.
4. Actualiza STATE.md y PORT_PLAN.md con SHA, anchors, comandos, resultados y límites reales.
5. Valida una sola vez al final con un ejecutor exclusivo, Cargo con un job y caché existente.
   La campaña ordinaria completa debe quedar en 600 segundos o menos; mide compilación
   especial y QA viva por separado. Usa validation-v2 según el alcance y no ocultes un
   fallo de hotspot ratchet preexistente.
6. Haz commit coherente, crea/actualiza la única PR de la macro-issue y continúa con la
   siguiente entrega autorizada. No te detengas después de un helper, commit o PR.

No pidas confirmación para decisiones que se puedan resolver inspeccionando código, C++,
datos, tests o el plan. Pregunta solo si falta una autorización o decisión material que
la evidencia no pueda resolver. Si un colaborador se atasca, recupéralo y continúa tú.
Nunca afirmes que algo está integrado, jugable o aceptado sin evidencia ejecutada en el
SHA correspondiente.
```

La sesión nueva debe tratar este documento como un punto de entrada corto, no como una
fuente de estado alternativa. Para el estado actualizado prevalecen [`STATE.md`](../migration/STATE.md),
[`PORT_PLAN.md`](../migration/PORT_PLAN.md), GitHub #49 y el `AGENTS.md` del repositorio.
La configuración efectiva del orquestador vive en [`.codex/config.toml`](../../.codex/config.toml)
y [`.codex/agents/luna-worker.toml`](../../.codex/agents/luna-worker.toml).
