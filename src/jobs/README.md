# Jobs & Resiliencia

Sistema de jobs críticos con resiliencia que sobrevive a reinicios.

## Jobs Críticos

### 1. Re-sync Blockchain
- **Frecuencia**: Cada minuto
- **Crítico**: Sí
- **Resiliente**: Sí
- **Descripción**: Sincroniza eventos de blockchain y reconcilia estados

### 2. Limpieza de Órdenes Expiradas
- **Frecuencia**: Cada hora
- **Crítico**: Sí
- **Resiliente**: Sí
- **Descripción**: Cancela automáticamente órdenes expiradas

### 3. Verificación de Inconsistencias
- **Frecuencia**: Cada 30 minutos
- **Crítico**: Sí
- **Resiliente**: Sí
- **Descripción**: Detecta y reporta inconsistencias entre órdenes, escrows y disputas

## Resiliencia

### Características

1. **Locks Distribuidos**: Previene ejecuciones concurrentes
2. **Estado Persistente**: Guarda estado en Redis para reanudación
3. **Recuperación Automática**: Recupera jobs después de reinicios
4. **Tracking Completo**: Registra todas las ejecuciones
5. **Tolerancia a Fallos**: Continúa funcionando aunque Redis falle

### JobTrackerService

Servicio central para tracking y gestión de jobs.

```typescript
import { JobTrackerService } from '@/jobs';

// Iniciar tracking
await jobTracker.startJob('job-name', executionId);

// Completar
await jobTracker.completeJob('job-name', executionId, result);

// Fallar
await jobTracker.failJob('job-name', executionId, error);

// Adquirir lock
const acquired = await jobTracker.acquireJobLock('job-name', ttl);

// Guardar estado
await jobTracker.saveJobState('job-name', state);

// Obtener estado
const state = await jobTracker.getJobState('job-name');
```

### JobRecoveryService

Se ejecuta automáticamente al iniciar el módulo:
- Libera locks huérfanos
- Recupera estados guardados
- Re-sincroniza blockchain si es necesario

## Jobs Implementados

### BlockchainSyncJob

**Tareas:**
- Sincronización continua (cada minuto)
- Verificación de estado (cada 5 minutos)
- Reconciliación profunda (cada hora)

**Resiliencia:**
- Guarda último bloque sincronizado
- Re-sincroniza desde último bloque conocido después de reinicio
- Lock distribuido previene ejecuciones concurrentes

**Ejemplo de uso manual:**
```typescript
await blockchainSyncJob.emergencyResync(fromBlock);
```

### CleanupJob

**Tareas:**
- Limpieza de órdenes expiradas (cada hora)
- Limpieza de notificaciones antiguas (diario a las 2 AM)
- Limpieza semanal de datos antiguos

**Resiliencia:**
- Guarda estadísticas de limpieza
- Notifica a usuarios afectados
- Continúa desde donde quedó después de reinicio

### ConsistencyCheckJob

**Tareas:**
- Verificación de inconsistencias (cada 30 minutos)
- Verificación profunda semanal

**Verificaciones:**
1. Consistencia entre órdenes y escrows
2. Escrows sin orden
3. Estados inconsistentes
4. Disputas sin orden o con estado incorrecto

**Resiliencia:**
- Guarda lista de issues encontrados
- Reporta problemas sin bloquear sistema

## Uso

### Ejecutar Job Manualmente

```typescript
import { BlockchainSyncJob } from '@/jobs';

// Re-sincronización de emergencia
await blockchainSyncJob.emergencyResync(fromBlock);
```

### Verificar Estado de Job

```typescript
import { JobTrackerService } from '@/jobs';

// Última ejecución
const lastExecution = await jobTracker.getLastExecution('blockchain-sync');

// Estado guardado
const state = await jobTracker.getJobState('blockchain-sync');

// Verificar si está en ejecución
const isRunning = await jobTracker.isJobRunning('blockchain-sync');
```

### Crear Nuevo Job

```typescript
import { Injectable, Logger } from '@nestjs/common';
import { Cron, CronExpression } from '@nestjs/schedule';
import { JobTrackerService } from './job-tracker.service';

@Injectable()
export class MyJob {
  private readonly logger = new Logger(MyJob.name);
  private readonly jobName = 'my-job';

  constructor(private readonly jobTracker: JobTrackerService) {}

  @Cron(CronExpression.EVERY_HOUR)
  async handleMyJob() {
    const executionId = this.jobTracker.generateExecutionId();

    // Verificar si ya está en ejecución
    const isRunning = await this.jobTracker.isJobRunning(this.jobName);
    if (isRunning) {
      return;
    }

    // Adquirir lock
    const lockAcquired = await this.jobTracker.acquireJobLock(this.jobName, 3600);
    if (!lockAcquired) {
      return;
    }

    try {
      await this.jobTracker.startJob(this.jobName, executionId);

      // Tu lógica aquí
      const result = await this.doSomething();

      // Guardar estado
      await this.jobTracker.saveJobState(this.jobName, {
        lastRun: new Date(),
        result,
      });

      await this.jobTracker.completeJob(this.jobName, executionId, result);
    } catch (error) {
      this.logger.error(`Error: ${error.message}`, error.stack);
      await this.jobTracker.failJob(this.jobName, executionId, error.message);
    } finally {
      await this.jobTracker.releaseJobLock(this.jobName);
    }
  }

  private async doSomething() {
    // Tu lógica
  }
}
```

## Recuperación Después de Reinicio

### Proceso Automático

1. **Al iniciar**: `JobRecoveryService.onModuleInit()` se ejecuta
2. **Libera locks**: Elimina locks huérfanos de reinicios anteriores
3. **Recupera estado**: Obtiene estados guardados de Redis
4. **Re-sincroniza**: Si es necesario, re-sincroniza blockchain

### Estado Guardado

Los jobs guardan estado en Redis con TTL de 24 horas:
- Última ejecución
- Resultados
- Puntos de control
- Errores

### Locks

Los locks tienen TTL automático:
- Previene locks infinitos
- Se liberan automáticamente si el proceso muere
- Se limpian al reiniciar

## Monitoreo

### Logs

Todos los jobs registran:
- Inicio de ejecución
- Completación exitosa
- Errores
- Estadísticas

### Métricas

Estado guardado incluye:
- Tiempo de ejecución
- Resultados
- Errores encontrados
- Items procesados

## Configuración

### Frecuencias

```typescript
// Cada minuto
@Cron(CronExpression.EVERY_MINUTE)

// Cada 30 minutos
@Cron('0 */30 * * * *')

// Cada hora
@Cron(CronExpression.EVERY_HOUR)

// Diario a las 2 AM
@Cron(CronExpression.EVERY_DAY_AT_2AM)

// Semanal
@Cron(CronExpression.EVERY_WEEK)
```

### TTL de Locks

```typescript
// Lock de 5 minutos
await jobTracker.acquireJobLock('job-name', 300);

// Lock de 1 hora
await jobTracker.acquireJobLock('job-name', 3600);

// Lock de 2 horas
await jobTracker.acquireJobLock('job-name', 7200);
```

## Notas Importantes

1. **Siempre usar locks**: Previene ejecuciones concurrentes
2. **Guardar estado**: Permite reanudación después de reinicio
3. **Manejar errores**: Siempre liberar locks en finally
4. **Logging**: Registrar todas las operaciones importantes
5. **TTL adecuado**: Configurar TTL según duración esperada del job

## Troubleshooting

### Job no se ejecuta

1. Verificar si hay lock activo: `await jobTracker.isJobRunning('job-name')`
2. Verificar logs para errores
3. Verificar configuración de cron

### Job se ejecuta dos veces

1. Verificar que el lock se adquiera correctamente
2. Verificar TTL del lock
3. Verificar que se libere en finally

### Estado perdido después de reinicio

1. Verificar que Redis esté funcionando
2. Verificar TTL del estado guardado
3. Verificar que se guarde antes de completar

## Ejemplos

### Job Simple con Resiliencia

```typescript
@Cron(CronExpression.EVERY_HOUR)
async handleJob() {
  const executionId = this.jobTracker.generateExecutionId();
  const lockAcquired = await this.jobTracker.acquireJobLock('my-job', 3600);
  
  if (!lockAcquired) return;

  try {
    await this.jobTracker.startJob('my-job', executionId);
    
    // Lógica del job
    const result = await this.process();
    
    await this.jobTracker.saveJobState('my-job', { result });
    await this.jobTracker.completeJob('my-job', executionId, result);
  } catch (error) {
    await this.jobTracker.failJob('my-job', executionId, error.message);
  } finally {
    await this.jobTracker.releaseJobLock('my-job');
  }
}
```

Listo para usar. El sistema de jobs es resiliente y sobrevive a reinicios.
