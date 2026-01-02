/**
 * APIキー管理（暗号化保存対応）
 */
import { encrypt, decrypt, encryptWithFixedKey, decryptWithFixedKey } from '../utils/crypto';

const STORAGE_KEY = 'sekou_taisei_api_key';
const ENCRYPTED_KEY = 'sekou_taisei_encrypted_key';
const MASTER_HASH_KEY = 'sekou_taisei_master_hash';

let cachedApiKey: string | null = null;

export const getApiKey = (): string | null => {
  if (cachedApiKey) return cachedApiKey;
  const sessionKey = sessionStorage.getItem(STORAGE_KEY);
  if (sessionKey) {
    cachedApiKey = sessionKey;
    return sessionKey;
  }
  const localKey = localStorage.getItem(STORAGE_KEY);
  if (localKey) {
    cachedApiKey = localKey;
    sessionStorage.setItem(STORAGE_KEY, localKey);
    return localKey;
  }
  return null;
};

export const setApiKey = (key: string): void => {
  cachedApiKey = key;
  sessionStorage.setItem(STORAGE_KEY, key);
  localStorage.setItem(STORAGE_KEY, key);
};

export const setApiKeyEncrypted = async (key: string, masterPassword: string): Promise<void> => {
  cachedApiKey = key;
  const { encrypted, iv, salt } = await encrypt(key, masterPassword);
  localStorage.setItem(ENCRYPTED_KEY, JSON.stringify({ encrypted, iv, salt }));
  localStorage.removeItem(STORAGE_KEY);
};

export const loadApiKeyEncrypted = async (masterPassword: string): Promise<boolean> => {
  const stored = localStorage.getItem(ENCRYPTED_KEY);
  if (!stored) return false;
  try {
    const { encrypted, iv, salt } = JSON.parse(stored);
    const decrypted = await decrypt(encrypted, masterPassword, iv, salt);
    if (decrypted && decrypted.startsWith('AIza')) {
      cachedApiKey = decrypted;
      sessionStorage.setItem(STORAGE_KEY, decrypted);
      localStorage.setItem(STORAGE_KEY, decrypted);
      return true;
    }
    return false;
  } catch {
    return false;
  }
};

export const hasEncryptedApiKey = (): boolean => !!localStorage.getItem(ENCRYPTED_KEY);

export const hasApiKey = (): boolean => {
  const key = getApiKey();
  return !!key && key.startsWith('AIza');
};

export const clearApiKey = (): void => {
  cachedApiKey = null;
  sessionStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem(ENCRYPTED_KEY);
  localStorage.removeItem(MASTER_HASH_KEY);
};

export const getMasterHashKey = (): string => MASTER_HASH_KEY;

// スプレッドシート埋め込み用（固定キー暗号化）

/**
 * APIキーを暗号化してスプレッドシートに保存可能な形式で返す
 */
export const encryptApiKeyForSpreadsheet = async (apiKey: string): Promise<string> => {
  return await encryptWithFixedKey(apiKey);
};

/**
 * スプレッドシートから読み込んだ暗号化APIキーを復号してセット
 */
export const loadApiKeyFromSpreadsheet = async (encryptedData: string): Promise<boolean> => {
  try {
    const decrypted = await decryptWithFixedKey(encryptedData);
    if (decrypted && decrypted.startsWith('AIza')) {
      setApiKey(decrypted);
      console.log('[APIKey] Loaded from spreadsheet successfully');
      return true;
    }
    console.log('[APIKey] Decrypted but invalid key format');
    return false;
  } catch (e) {
    console.error('[APIKey] Failed to decrypt from spreadsheet:', e);
    return false;
  }
};

/**
 * 現在のAPIキーを暗号化してスプレッドシートに保存
 */
export const saveApiKeyToSpreadsheet = async (gasUrl: string): Promise<boolean> => {
  const apiKey = getApiKey();
  if (!apiKey) {
    console.log('[APIKey] No API key to save');
    return false;
  }

  try {
    const encryptedData = await encryptApiKeyForSpreadsheet(apiKey);

    // GASに送信して_settingsシートに保存
    const response = await fetch(gasUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        action: 'saveSettings',
        settings: {
          encryptedApiKey: encryptedData
        }
      })
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const result = await response.json();
    console.log('[APIKey] Saved to spreadsheet:', result);
    return result.success === true;
  } catch (e) {
    console.error('[APIKey] Failed to save to spreadsheet:', e);
    return false;
  }
};
