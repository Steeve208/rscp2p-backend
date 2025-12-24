# Regla Final de Seguridad - NO NEGOCIABLE

## 🚨 REGLA CRÍTICA: NINGÚN ARCHIVO DEL BACKEND DEBE MOVER FONDOS

### Principio Fundamental

**Si un módulo puede mover fondos → está mal diseñado.**

El backend de RSC Finance (P2P) es un sistema **read-only** para blockchain. Su única función es:

1. ✅ **Escuchar eventos** de blockchain
2. ✅ **Validar estados** entre off-chain y on-chain
3. ✅ **Gestionar estados off-chain** (órdenes, disputas, reputation)
4. ✅ **Proporcionar APIs** para el frontend

### ❌ LO QUE EL BACKEND NUNCA DEBE HACER

El backend **NUNCA** debe:

- ❌ Ejecutar transacciones que muevan fondos
- ❌ Usar `wallet.send()` o `wallet.sendTransaction()`
- ❌ Llamar métodos de contrato que muevan fondos (`release()`, `refund()`, `transfer()`, etc.)
- ❌ Firmar transacciones con claves privadas del servidor
- ❌ Tener acceso a claves privadas con fondos

### ✅ LO QUE EL BACKEND SÍ DEBE HACER

El backend **SÍ** debe:

- ✅ Escuchar eventos de blockchain (`contract.on()`)
- ✅ Leer estados de contratos (`contract.functions.*()` sin `send()`)
- ✅ Validar consistencia entre órdenes y escrows
- ✅ Actualizar estados off-chain basados en eventos
- ✅ Proporcionar información al frontend

## Arquitectura Correcta

### Flujo de Transacciones

```
Usuario (Frontend) → Wallet (MetaMask, etc.) → Blockchain
                              ↓
                    Eventos emitidos
                              ↓
                    Backend (escucha eventos)
                              ↓
                    Actualiza estados off-chain
```

**El backend NO está en el flujo de transacciones.**

### Módulos y Sus Responsabilidades

#### Blockchain Module
- ✅ Escucha eventos (`EventListenerService`)
- ✅ Lee bloques y transacciones
- ✅ Valida bloques
- ✅ Reconcilia estados
- ❌ NUNCA ejecuta transacciones

#### Escrow Module
- ✅ Mapea `order_id ↔ escrow_id`
- ✅ Valida consistencia
- ✅ Actualiza estados basados en eventos
- ❌ NUNCA ejecuta transacciones de escrow

#### Orders Module
- ✅ Crea órdenes off-chain
- ✅ Gestiona estados de órdenes
- ✅ Valida transiciones de estado
- ❌ NUNCA mueve fondos

#### Disputes Module
- ✅ Gestiona disputas off-chain
- ✅ Procesa evidencia
- ✅ Calcula resoluciones
- ❌ NUNCA ejecuta resoluciones de escrow

## Verificación de Código

### Patrones Prohibidos

Si encuentras alguno de estos patrones, **está mal diseñado**:

```typescript
// ❌ PROHIBIDO
wallet.sendTransaction(tx)
wallet.send(tx)
contract.release(escrowId).send()
contract.refund(escrowId).send()
contract.transfer(to, amount).send()
signer.sendTransaction(tx)
```

### Patrones Permitidos

Estos patrones son correctos:

```typescript
// ✅ PERMITIDO
provider.getBalance(address) // Solo lectura
contract.on('Event', handler) // Solo escucha
contract.functions.getState().call() // Solo lectura
ethers.verifyMessage(message, signature) // Solo validación
```

## Configuración Segura

### Variables de Entorno

```env
# ✅ CORRECTO: Solo RPC y dirección de contrato
BLOCKCHAIN_RPC_URL=https://eth.llamarpc.com
BLOCKCHAIN_ESCROW_CONTRACT_ADDRESS=0x...

# ❌ INCORRECTO: Clave privada con fondos
# BLOCKCHAIN_PRIVATE_KEY=0x... (NO debe tener fondos)
```

### Wallet en Configuración

El wallet en `config/blockchain.ts` es **SOLO para lectura** (si es necesario):

```typescript
// ✅ CORRECTO: Wallet sin fondos, solo para lectura
const wallet = new ethers.Wallet(privateKey, provider);
// NUNCA usar: wallet.send() o wallet.sendTransaction()

// ✅ CORRECTO: Solo provider para lectura
const provider = new ethers.JsonRpcProvider(rpcUrl);
```

## Testing de Seguridad

### Checklist de Revisión

Antes de hacer commit, verifica:

- [ ] No hay llamadas a `send()` o `sendTransaction()`
- [ ] No hay métodos de contrato que muevan fondos
- [ ] El wallet (si existe) no tiene fondos
- [ ] Solo se usan métodos de lectura
- [ ] Los eventos se escuchan, no se emiten desde el backend

### Comandos de Verificación

```bash
# Buscar patrones prohibidos
grep -r "\.send(" src/
grep -r "sendTransaction" src/
grep -r "\.release\|\.refund\|\.transfer" src/

# Si encuentras resultados, REVISAR y ELIMINAR
```

## Ejemplos de Código Correcto

### ✅ Escuchar Eventos

```typescript
// CORRECTO: Solo escucha eventos
contract.on('FundsReleased', async (escrowId, recipient, amount, event) => {
  // Actualizar estado off-chain
  await escrowService.update(escrowId, {
    status: EscrowStatus.RELEASED,
    releaseTransactionHash: event.transactionHash,
  });
});
```

### ✅ Leer Estados

```typescript
// CORRECTO: Solo lectura
const balance = await provider.getBalance(address);
const state = await contract.functions.getEscrowState(escrowId).call();
```

### ❌ Ejemplos Incorrectos

```typescript
// ❌ INCORRECTO: Mover fondos
await contract.release(escrowId).send({ from: wallet.address });

// ❌ INCORRECTO: Enviar transacción
await wallet.sendTransaction({
  to: contractAddress,
  data: releaseData,
});

// ❌ INCORRECTO: Firmar y enviar
const tx = await contract.populateTransaction.release(escrowId);
const signedTx = await wallet.signTransaction(tx);
await provider.sendTransaction(signedTx);
```

## Responsabilidades del Frontend

El frontend es responsable de:

- ✅ Conectar wallets de usuarios (MetaMask, WalletConnect, etc.)
- ✅ Solicitar firmas de transacciones a los usuarios
- ✅ Enviar transacciones firmadas a blockchain
- ✅ Mostrar estados actualizados del backend

## Consecuencias de Violar Esta Regla

Si un módulo puede mover fondos:

1. **Riesgo de seguridad crítico**: El backend podría ser comprometido
2. **Pérdida de fondos**: Si el servidor es hackeado
3. **Violación de principios**: El backend no debe tener control sobre fondos
4. **Diseño incorrecto**: Va contra la arquitectura del sistema

## Resumen

- ✅ Backend = Read-only para blockchain
- ✅ Backend = Gestión de estados off-chain
- ❌ Backend ≠ Ejecución de transacciones
- ❌ Backend ≠ Movimiento de fondos

**Si puedes mover fondos desde el backend, está mal diseñado.**
