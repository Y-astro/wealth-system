// Settings Commands
import type { Settings } from "@/lib/types";
import type { AppInfo, PlatformInfo } from "../types";

import { invoke, logger, tauriInvoke } from "./core";

export const getSettings = async (): Promise<Settings> => {
  try {
    return await invoke<Settings>("get_settings");
  } catch (err) {
    logger.error("Error fetching settings.");
    throw err;
  }
};

export const updateSettings = async (settingsUpdate: Partial<Settings>): Promise<Settings> => {
  try {
    return await invoke<Settings>("update_settings", { settingsUpdate });
  } catch (error) {
    logger.error("Error updating settings.");
    throw error;
  }
};

export const backupDatabase = async (): Promise<{ filename: string; data: Uint8Array }> => {
  try {
    // Desktop (Tauri) returns a tuple [filename, data[]], transform internally to object
    const result = await tauriInvoke<[string, number[]]>("backup_database");
    const [filename, data] = result;
    return { filename, data: new Uint8Array(data) };
  } catch (error) {
    logger.error("Error backing up database.");
    throw error;
  }
};

export const backupDatabaseToPath = async (backupDir: string): Promise<string> => {
  try {
    return await invoke<string>("backup_database_to_path", { backupDir });
  } catch (error) {
    logger.error("Error backing up database to path.");
    throw error;
  }
};

export const restoreDatabase = async (backupFilePath: string): Promise<void> => {
  try {
    await invoke<void>("restore_database", { backupFilePath });
  } catch (error) {
    logger.error("Error restoring database.");
    throw error;
  }
};

// ============================================================================
// App Commands
// ============================================================================

export const getAppInfo = async (): Promise<AppInfo> => {
  try {
    return await invoke<AppInfo>("get_app_info");
  } catch (err) {
    logger.error("Error fetching app info");
    throw err;
  }
};

// ============================================================================
// Platform Commands
// ============================================================================

export const getPlatform = async (): Promise<PlatformInfo> => {
  return invoke<PlatformInfo>("get_platform");
};
