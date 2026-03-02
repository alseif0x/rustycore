# Phase 4: Analisis de Errores y Plan de Correccion

## Estado Actual

Phase 4 implementada (254 tests). Compila y los tests pasan, pero hay **diferencias criticas** entre nuestra implementacion y el protocolo real del cliente 3.4.3.54261 que impediran que el cliente funcione correctamente.

---

## 1. CRITICO: Flujo ConnectTo en PlayerLogin

### El problema

Nuestra implementacion de `handle_player_login` envia la secuencia de login directamente en la misma conexion TCP. El cliente 3.4.3 espera un flujo completamente diferente:

**Lo que hace el C# (correcto):**
```
Cliente envia CMSG_PLAYER_LOGIN
  → Servidor envia SMSG_CONNECT_TO (con IP, puerto, key RSA-firmado)
  → Cliente se desconecta de la conexion actual
  → Cliente abre NUEVA conexion TCP al "instance server"
  → Cliente envia AuthContinuedSession (con key)
  → Servidor valida, envia EnterEncryptedMode
  → Servidor envia ResumeComms (paquete vacio)
  → Servidor carga datos del personaje de DB (async)
  → Servidor envia secuencia de login (30+ paquetes)
```

**Lo que hace nuestra implementacion (incorrecto):**
```
Cliente envia CMSG_PLAYER_LOGIN
  → Servidor envia secuencia de login directamente (9 paquetes)
```

### Solucion propuesta: "Self-ConnectTo"

Para un servidor single-process, el ConnectTo apunta al **mismo servidor y puerto**. El cliente igualmente se reconecta. Necesitamos:

1. **Implementar `SMSG_CONNECT_TO`** con firma RSA (256 bytes signature)
2. **Implementar `AuthContinuedSession`** handler (similar a AuthSession pero con key)
3. **Almacenar ConnectToKey** en la sesion para validar la reconexion
4. **Implementar `ResumeComms`** (paquete vacio, solo opcode)
5. **Mover la secuencia de login** a `HandleContinuePlayerLogin`

### Archivos afectados

| Archivo | Cambio |
|---------|--------|
| `wow-packet/src/packets/auth.rs` | Agregar ConnectTo, AuthContinuedSession, ResumeComms |
| `wow-network/src/world_socket.rs` | Handler para AuthContinuedSession en el accept loop |
| `wow-world/src/handlers/character.rs` | PlayerLogin solo envia ConnectTo, nueva funcion continue_login |
| `wow-world/src/session.rs` | Campo connect_to_key para validar reconexion |

### Alternativa simplificada (para pruebas iniciales)

Podriamos intentar **enviar la secuencia de login en la misma conexion** sin ConnectTo, pero es muy probable que el cliente lo rechace porque espera la reconexion. Si el cliente no responde a nuestros paquetes de login directos, sabemos que ConnectTo es obligatorio.

---

## 2. CRITICO: Response Codes Incorrectos

### El problema

Nuestros codigos de respuesta usan 0 para exito. El C# usa valores diferentes:

| Codigo | Nuestro valor | Valor correcto |
|--------|:------------:|:--------------:|
| CHAR_CREATE_SUCCESS | 0 | **24** |
| CHAR_CREATE_ERROR | 1 | **25** |
| CHAR_CREATE_FAILED | 2 | **26** |
| CHAR_CREATE_NAME_IN_USE | 3 | **27** |
| CHAR_CREATE_DISABLED | 4 | **28** |
| CHAR_CREATE_SERVER_LIMIT | 7 | **30** |
| CHAR_CREATE_ACCOUNT_LIMIT | 8 | **31** |
| CHAR_DELETE_SUCCESS | 0 | **63** |
| CHAR_DELETE_FAILED | 1 | **64** |

### Solucion

Actualizar `wow-packet/src/packets/character.rs` modulo `response_codes`.

### Impacto

Sin esto, el cliente mostrara mensajes de error incorrectos o se comportara de forma inesperada al crear/borrar personajes.

---

## 3. IMPORTANTE: EnumCharactersResult - Campos Faltantes

### El problema

Nuestro CharacterInfo le faltan campos que el C# serializa. El formato correcto es:

```
PackedGuid guid
u64 GuildClubMemberID          ← FALTA
u8 ListPosition
u8 RaceId
u8 ClassId
u8 SexId
i32 Customizations.Count       ← Tenemos u8, debe ser i32
u8 ExperienceLevel
i32 ZoneId
i32 MapId
Vector3 Position
PackedGuid GuildGuid
u32 Flags
u32 Flags2
u32 Flags3
u32 PetDisplayId
u32 PetLevel
u32 PetFamily
u32 ProfessionIds[0]           ← FALTA
u32 ProfessionIds[1]           ← FALTA
VisualItemInfo[34]
i64 LastPlayedTime             ← Tenemos pero valor fijo 0
i16 SpecID                     ← Tenemos pero valor fijo 0
i32 Unknown703                 ← FALTA
i32 LastLoginVersion           ← FALTA (deberia ser 54261)
u32 Flags4                     ← FALTA
i32 MailSenders.Count          ← FALTA
i32 MailSenderTypes.Count      ← FALTA
u32 OverrideSelectScreenFileDataID ← FALTA (tenemos pero escribe ch.flags en lugar de 0)
ChrCustomizationChoice[]       ← FALTA (array vacio)
MailSenderTypes[]              ← FALTA (array vacio)
bits(6) Name.Length
bit FirstLogin
bit BoostInProgress
bits(5) unkWod61x
bits(2) unknown                ← FALTA
bit RpeResetAvailable          ← FALTA
bit RpeResetQuestClearAvailable ← FALTA
MailSenders[] strings          ← FALTA
string Name
```

### Solucion

Actualizar `CharacterInfo` struct con campos faltantes y corregir el orden de serializacion en `EnumCharactersResult::write()`.

---

## 4. IMPORTANTE: AccountDataTimes - Array Size

### El problema

Tenemos `account_times: [i64; 8]`. El C# usa `AccountDataTypes.Max = 15`.

```rust
// Nuestro codigo (incorrecto)
pub account_times: [i64; 8],

// Correcto
pub account_times: [i64; 15],
```

### Impacto

El cliente lee 15 entradas. Si solo enviamos 8, leera datos basura de los siguientes bytes, corrompiendo el parseo de paquetes posteriores.

---

## 5. IMPORTANTE: FeatureSystemStatus - Formato Incorrecto

### El problema

Nuestro `FeatureSystemStatus` tiene el formato de campos completamente diferente al C#. El C# tiene:

```
u8 ComplaintStatus
u32 CfgRealmID
i32 CfgRealmRecID
u32 RAFSystem (5 fields)
u32 TokenPollTimeSeconds
u32 KioskSessionMinutes
i64 TokenBalanceAmount
u32 BpayStoreProductDeliveryDelay
u32 ClubsPresenceUpdateTimer
u32 HiddenUIClubsPresenceUpdateTimer
i32 ActiveSeason
i32 GameRuleValues.Count
i16 MaxPlayerNameQueriesPerPacket
i16 PlayerNameQueryTelemetryInterval
u32 PlayerNameQueryInterval
GameRuleValuePair[] (dynamic)
42 bits (boolean flags)
FlushBits
QuickJoinConfig (bit + 20 floats)
SessionAlert (optional: 3x i32)
Squelch (bit + 2 packed guids)
EuropaTicketSystemStatus (optional)
```

Nuestro formato es completamente diferente: tiene campos que no existen y le faltan campos criticos. No hay correspondencia campo a campo.

### Solucion

Reescribir `FeatureSystemStatus::write()` completamente siguiendo el formato exacto del C#.

---

## 6. MENOR: UpdateObject - Simplificaciones

### El problema

Nuestro UpdateObject packet usa un formato muy simplificado. El formato real es mucho mas complejo con:

- ChangeMask sections para cada grupo de campos (ObjectData, UnitData, PlayerData)
- Cada section tiene: field_count + mask_count + mask_words + field_values
- Movement block tiene bits para cada componente (living, position, stationary, etc.)
- CreateObjectBits tiene 18+ bits (no 15)

### Impacto

El UpdateObject es probablemente el paquete mas critico y complejo. Si el formato es incorrecto, el cliente crasheara o no renderizara al personaje.

### Solucion

Este es el fix mas complejo. Necesita un analisis profundo del formato de UpdateObject del C# (UpdateData.cs, UpdateFieldsData.cs, ObjectUpdate.cs).

---

## 7. MENOR: CreateChar Response - Falta GUID

### El problema

El C# envia GUID en la respuesta de CreateChar:
```csharp
_worldPacket.WriteUInt8((byte)Code);
_worldPacket.WritePackedGuid(Guid);  // GUID del nuevo personaje
```

Nuestro codigo ya tiene esto correcto (`CreateChar` tiene `guid: ObjectGuid`).

---

## Plan de Correccion (Orden de Prioridad)

### Paso 1: Fixes rapidos (se pueden probar inmediatamente)

1. **Response codes** → Cambiar valores en `character.rs::response_codes`
2. **AccountDataTimes** → Cambiar array de 8 a 15
3. **EnumCharactersResult** → Agregar campos faltantes en orden correcto

### Paso 2: FeatureSystemStatus

4. **Reescribir FeatureSystemStatus** siguiendo formato exacto del C#

### Paso 3: ConnectTo Flow

5. **Implementar SMSG_CONNECT_TO** packet
6. **Implementar AuthContinuedSession** handler
7. **Implementar ResumeComms** packet
8. **Reestructurar PlayerLogin** → ConnectTo → reconexion → login sequence

### Paso 4: UpdateObject

9. **Estudiar formato completo** de UpdateObject/UpdateData del C#
10. **Reescribir UpdateObject** con ChangeMask correcto
11. **Movement block** con bits correctos

### Paso 5: Login Sequence Completa

12. Agregar paquetes faltantes de la secuencia de login (los 30+ paquetes)
13. Muchos pueden ser stubs vacios inicialmente

---

## Estrategia de Testing

### Test 1: Character List (sin login)

Probar que el cliente puede:
1. Conectar al world server
2. Completar handshake + encryption
3. Enviar EnumCharacters
4. Recibir EnumCharactersResult (lista vacia)
5. Enviar CreateCharacter
6. Recibir CreateChar (success code 24)
7. Enviar EnumCharacters de nuevo
8. Ver el personaje en la lista

**Requiere fixes:** Response codes (#2), EnumCharactersResult format (#3), AccountDataTimes (#4), FeatureSystemStatus (#5)

### Test 2: Player Login (con ConnectTo)

Despues del Test 1:
1. Cliente selecciona personaje
2. Envia PlayerLogin
3. Servidor envia ConnectTo
4. Cliente reconecta
5. AuthContinuedSession
6. Login sequence
7. Personaje aparece en el mundo

**Requiere:** Todo lo anterior + ConnectTo flow (#5-8) + UpdateObject fix (#9-11)

---

## Referencia: Secuencia Completa de Login (30+ paquetes)

Orden exacto del C#:

```
 1. ResumeComms (vacio)
 2. AccountDataTimes (global)
 3. AccountDataTimes (per-character)
 4. TutorialFlags
 5. LoginVerifyWorld
 6. FeatureSystemStatus
 7. MOTD (server message, si existe)
 8. SetTimeZoneInformation
 9. SeasonInfo (arena season)
--- SendInitialPacketsBeforeAddToMap ---
10. TimeSyncRequest
11. SocialList (friends/ignore)
12. BindPointUpdate (hearthstone)
13. TalentsInfo
14. InitialSpells
15. SendUnlearnSpells
16. SendSpellHistory
17. SendSpellCharges
18. ActiveGlyphs
19. ActionButtons
20. InitializeFactions (reputations)
21. SetupCurrency
22. EquipmentSetList
23. Achievements
24. QuestObjectiveCriteria
25. LoginSetTimeSpeed
26. WorldServerInfo
27. SpellModifiers
28. AccountMountUpdate
29. AccountToysUpdate
--- Player Added to Map ---
30. UpdateObject (CREATE_OBJECT2 para el player)
--- SendInitialPacketsAfterAddToMap ---
31. LoadCUFProfiles
32. Login spell effect
33. Aura updates
34. Movement state compound
35. Enchantment/item durations
36. Phase updates
```

Para un MVP minimo, los paquetes 1-9 + 25 + 30 son suficientes. Los demas pueden ser stubs vacios o omitidos inicialmente.
