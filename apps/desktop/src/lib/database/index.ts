import Database from "@tauri-apps/plugin-sql";

let dbInstance: Database | null = null;

/**
 * Lazily initializes and returns the SQLite database connection.
 * The database connection is NOT opened during application startup,
 * but rather on the first call to this function.
 */
export async function getDb(): Promise<Database> {
  if (!dbInstance) {
    // Open the SQLite database file: aquatick.sqlite
    dbInstance = await Database.load("sqlite:aquatick.sqlite");
  }
  return dbInstance;
}
