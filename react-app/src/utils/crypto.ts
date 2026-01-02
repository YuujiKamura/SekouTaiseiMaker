/**
 * 暗号化ユーティリティ
 * Web Crypto APIを使用してAES-256-GCMで暗号化
 */

const ALGORITHM = 'AES-GCM';
const KEY_LENGTH = 256;
const ITERATIONS = 100000;
const SALT_LENGTH = 16;
const IV_LENGTH = 12;

const deriveKey = async (password: string, salt: Uint8Array): Promise<CryptoKey> => {
  const encoder = new TextEncoder();
  const passwordBuffer = encoder.encode(password);
  const passwordKey = await crypto.subtle.importKey('raw', passwordBuffer, 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: salt.buffer as ArrayBuffer, iterations: ITERATIONS, hash: 'SHA-256' },
    passwordKey,
    { name: ALGORITHM, length: KEY_LENGTH },
    false,
    ['encrypt', 'decrypt']
  );
};

export const encrypt = async (plaintext: string, password: string): Promise<{ encrypted: string; iv: string; salt: string }> => {
  const encoder = new TextEncoder();
  const data = encoder.encode(plaintext);
  const salt = crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
  const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));
  const key = await deriveKey(password, salt);
  const encryptedBuffer = await crypto.subtle.encrypt({ name: ALGORITHM, iv: iv.buffer as ArrayBuffer }, key, data);
  return {
    encrypted: btoa(String.fromCharCode(...new Uint8Array(encryptedBuffer))),
    iv: btoa(String.fromCharCode(...iv)),
    salt: btoa(String.fromCharCode(...salt)),
  };
};

export const decrypt = async (encrypted: string, password: string, ivBase64: string, saltBase64: string): Promise<string> => {
  const encryptedBuffer = Uint8Array.from(atob(encrypted), c => c.charCodeAt(0));
  const iv = Uint8Array.from(atob(ivBase64), c => c.charCodeAt(0));
  const salt = Uint8Array.from(atob(saltBase64), c => c.charCodeAt(0));
  const key = await deriveKey(password, salt);
  const decryptedBuffer = await crypto.subtle.decrypt({ name: ALGORITHM, iv: iv.buffer as ArrayBuffer }, key, encryptedBuffer);
  return new TextDecoder().decode(decryptedBuffer);
};

export const hashPassword = async (password: string): Promise<string> => {
  const encoder = new TextEncoder();
  const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(password));
  return btoa(String.fromCharCode(...new Uint8Array(hashBuffer)));
};

// 固定キー暗号化（スプレッドシート埋め込み用）
const FIXED_KEY = 'SekouTaisei2024!AppKey#Encrypt';

export const encryptWithFixedKey = async (plaintext: string): Promise<string> => {
  const { encrypted, iv, salt } = await encrypt(plaintext, FIXED_KEY);
  return JSON.stringify({ encrypted, iv, salt });
};

export const decryptWithFixedKey = async (encryptedJson: string): Promise<string | null> => {
  try {
    const { encrypted, iv, salt } = JSON.parse(encryptedJson);
    return await decrypt(encrypted, FIXED_KEY, iv, salt);
  } catch {
    return null;
  }
};
