// Web adapter - Settings and App Info

import { invoke, logger } from "./core";
import type { Settings } from "@/lib/types";
import type { AppInfo, PlatformInfo } from "../types";

// ============================================================================
// Settings Commands
// ============================================================================

export const getSettings = async (): Promise<Settings> => {
  try {
    return await invoke<Settings>("get_settings");
  } catch (_error) {
    logger.error("Error fetching settings.");
    return {} as Settings;
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
    // Web backend returns { filename, data } object directly (transformed in invoke)
    return await invoke<{ filename: string; data: Uint8Array }>("backup_database");
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
    console.error(err);
    return {
      version: "",
      dbPath: "",
      logsDir: "",
    };
  }
};

// ============================================================================
// Platform Commands
// ============================================================================

export const getPlatform = (): Promise<PlatformInfo> => {
  // Web environment - detect from user agent
  const userAgent = typeof window !== "undefined" ? window.navigator.userAgent.toLowerCase() : "";
  const platform =
    typeof window !== "undefined" ? window.navigator.platform?.toLowerCase() || "" : "";

  // Check for mobile devices
  const isMobileUA = /android|webos|iphone|ipad|ipod|blackberry|iemobile|opera mini/i.test(
    userAgent,
  );
  const isTablet = /ipad|tablet|playbook|silk/i.test(userAgent);

  // Detect OS
  let os = "unknown";
  if (/iphone|ipad|ipod/.test(userAgent)) {
    os = "ios";
  } else if (userAgent.includes("android")) {
    os = "android";
  } else if (/mac|darwin/.test(platform) || userAgent.includes("macintosh")) {
    os = "macos";
  } else if (platform.includes("win") || userAgent.includes("windows")) {
    os = "windows";
  } else if (platform.includes("linux") || userAgent.includes("linux")) {
    os = "linux";
  }

  const is_mobile = isMobileUA || isTablet;

  return Promise.resolve({
    os,
    is_mobile,
    is_desktop: !is_mobile,
    is_tauri: false,
    capabilities: {
      connect_sync: true,
      device_sync: true,
      cloud_sync: true,
    },
  });
};
