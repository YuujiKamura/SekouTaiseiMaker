/**
 * WebAuthn Service - パスキー（指紋/顔認証）でAPIキーを保護
 * APIキーは暗号化してlocalStorageに保存（XSS対策）
 */

import { encryptWithFixedKey, decryptWithFixedKey } from '../utils/crypto';

// localStorage key names (not actual credentials)
const STORAGE_KEY_CREDENTIAL_ID = 'sekou_taisei_credential_id';
const STORAGE_KEY_API_KEY_ENCRYPTED = 'sekou_taisei_passkey_api_key_enc';

export const isWebAuthnSupported = (): boolean => {
  return !!(
    typeof window !== 'undefined' &&
    window.PublicKeyCredential &&
    typeof navigator.credentials?.create === 'function' &&
    typeof navigator.credentials?.get === 'function'
  );
};

export const isBiometricAvailable = async (): Promise<boolean> => {
  if (!isWebAuthnSupported()) return false;
  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
};

export const hasRegisteredPasskey = (): boolean => {
  return !!(localStorage.getItem(STORAGE_KEY_CREDENTIAL_ID) && localStorage.getItem(STORAGE_KEY_API_KEY_ENCRYPTED));
};

const generateChallenge = (): Uint8Array => {
  const challenge = new Uint8Array(32);
  crypto.getRandomValues(challenge);
  return challenge;
};

const bufferToBase64url = (buffer: ArrayBuffer): string => {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
};

const base64urlToBuffer = (base64url: string): Uint8Array => {
  const base64 = base64url.replace(/-/g, '+').replace(/_/g, '/');
  const padding = '='.repeat((4 - base64.length % 4) % 4);
  const binary = atob(base64 + padding);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
};

export const registerPasskey = async (apiKey: string): Promise<{ success: boolean; error?: string }> => {
  if (!isWebAuthnSupported()) {
    return { success: false, error: 'WebAuthn非対応ブラウザ' };
  }

  try {
    const challenge = generateChallenge();
    const userId = new Uint8Array(16);
    crypto.getRandomValues(userId);

    const credential = await navigator.credentials.create({
      publicKey: {
        challenge: challenge.buffer as ArrayBuffer,
        rp: { name: '施工体制メーカー', id: window.location.hostname },
        user: { id: userId.buffer as ArrayBuffer, name: 'user@sekou-taisei', displayName: 'ユーザー' },
        pubKeyCredParams: [
          { alg: -7, type: 'public-key' },
          { alg: -257, type: 'public-key' },
        ],
        authenticatorSelection: {
          authenticatorAttachment: 'platform',
          userVerification: 'required',
          residentKey: 'preferred',
        },
        timeout: 60000,
        attestation: 'none',
      },
    }) as PublicKeyCredential;

    if (!credential) {
      return { success: false, error: 'キャンセルされました' };
    }

    localStorage.setItem(STORAGE_KEY_CREDENTIAL_ID, bufferToBase64url(credential.rawId));
    // Encrypt API key before storing (XSS protection)
    const encryptedApiKey = await encryptWithFixedKey(apiKey);
    localStorage.setItem(STORAGE_KEY_API_KEY_ENCRYPTED, encryptedApiKey);
    return { success: true };
  } catch (e: unknown) {
    const err = e as Error & { name?: string };
    if (err.name === 'NotAllowedError') return { success: false, error: 'キャンセルされました' };
    if (err.name === 'SecurityError') return { success: false, error: 'HTTPSが必要です' };
    return { success: false, error: err.message || '登録失敗' };
  }
};

export const authenticateWithPasskey = async (): Promise<{ success: boolean; apiKey?: string; error?: string }> => {
  if (!isWebAuthnSupported()) {
    return { success: false, error: 'WebAuthn非対応' };
  }

  const storedCredentialId = localStorage.getItem(STORAGE_KEY_CREDENTIAL_ID);
  if (!storedCredentialId) {
    return { success: false, error: 'パスキー未登録' };
  }

  try {
    const credId = base64urlToBuffer(storedCredentialId);
    const assertion = await navigator.credentials.get({
      publicKey: {
        challenge: generateChallenge().buffer as ArrayBuffer,
        allowCredentials: [{
          id: credId.buffer as ArrayBuffer,
          type: 'public-key',
          transports: ['internal'],
        }],
        userVerification: 'required',
        timeout: 60000,
      },
    }) as PublicKeyCredential;

    if (!assertion) {
      return { success: false, error: 'キャンセルされました' };
    }

    // Decrypt API key from storage
    const encryptedApiKey = localStorage.getItem(STORAGE_KEY_API_KEY_ENCRYPTED);
    if (!encryptedApiKey) {
      return { success: false, error: 'APIキーが見つかりません' };
    }

    const apiKey = await decryptWithFixedKey(encryptedApiKey);
    if (!apiKey) {
      return { success: false, error: 'APIキーの復号に失敗しました' };
    }

    return { success: true, apiKey };
  } catch (e: unknown) {
    const err = e as Error & { name?: string };
    if (err.name === 'NotAllowedError') return { success: false, error: 'キャンセルされました' };
    return { success: false, error: err.message || '認証失敗' };
  }
};

export const removePasskey = (): void => {
  localStorage.removeItem(STORAGE_KEY_CREDENTIAL_ID);
  localStorage.removeItem(STORAGE_KEY_API_KEY_ENCRYPTED);
  // Clean up legacy plaintext storage if exists
  localStorage.removeItem('sekou_taisei_passkey_api_key');
};

/**
 * Migrate legacy plaintext API key to encrypted storage
 * Call this on app startup to ensure existing users are migrated
 */
export const migratePasskeyStorage = async (): Promise<void> => {
  const legacyKey = localStorage.getItem('sekou_taisei_passkey_api_key');
  const credentialId = localStorage.getItem(STORAGE_KEY_CREDENTIAL_ID);

  if (legacyKey && credentialId && !localStorage.getItem(STORAGE_KEY_API_KEY_ENCRYPTED)) {
    // Migrate to encrypted storage
    const encryptedApiKey = await encryptWithFixedKey(legacyKey);
    localStorage.setItem(STORAGE_KEY_API_KEY_ENCRYPTED, encryptedApiKey);
    localStorage.removeItem('sekou_taisei_passkey_api_key');
    console.log('[WebAuthn] Migrated API key to encrypted storage');
  }
};
