/**
 * Archivo exclusivo para el CLI de TypeORM (migration:run, migration:generate, migration:revert).
 * Debe exportar ÚNICAMENTE una instancia de DataSource; el CLI falla si hay más de un export.
 *
 * La app NestJS sigue usando getDatabaseConfig/AppDataSource desde database.ts.
 */
import { AppDataSource } from './database';
export default AppDataSource;
