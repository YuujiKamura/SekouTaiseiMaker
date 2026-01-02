/**
 * Safe base64 decoding utility
 * Handles common issues with base64 strings from external sources (GAS, etc.)
 */

/**
 * Sanitize a base64 string by removing whitespace and line breaks
 * that may have been added during transmission
 */
export function sanitizeBase64(base64: string): string {
  if (!base64 || typeof base64 !== 'string') {
    throw new Error('Invalid base64 input: empty or not a string');
  }

  // Remove all whitespace, line breaks, and carriage returns
  return base64.replace(/[\s\r\n]+/g, '');
}

/**
 * Validate that a string is valid base64
 */
export function isValidBase64(str: string): boolean {
  if (!str || typeof str !== 'string') return false;

  // Base64 should only contain A-Z, a-z, 0-9, +, /, and = for padding
  const base64Regex = /^[A-Za-z0-9+/]*={0,2}$/;
  return base64Regex.test(str) && str.length % 4 === 0;
}

/**
 * Safely decode base64 to ArrayBuffer
 * Handles common issues like whitespace, line breaks, and invalid characters
 */
export function safeBase64ToArrayBuffer(base64: string): ArrayBuffer {
  // Sanitize the input
  const sanitized = sanitizeBase64(base64);

  // Validate the sanitized string
  if (!isValidBase64(sanitized)) {
    // Try to identify the issue
    const invalidChars = sanitized.match(/[^A-Za-z0-9+/=]/g);
    if (invalidChars) {
      console.error('[base64] Invalid characters found:', [...new Set(invalidChars)].join(', '));
    }
    if (sanitized.length % 4 !== 0) {
      console.error('[base64] Invalid length:', sanitized.length, '(not divisible by 4)');
    }
    throw new Error('Base64データが不正です。ファイルを再取得してください。');
  }

  try {
    const binary = atob(sanitized);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
  } catch (e) {
    console.error('[base64] Decode failed:', e);
    throw new Error('Base64デコードに失敗しました。ファイルが破損している可能性があります。');
  }
}

/**
 * Safely decode base64 to binary string
 * Used when you need the raw binary string instead of ArrayBuffer
 */
export function safeBase64ToBinary(base64: string): string {
  const sanitized = sanitizeBase64(base64);

  if (!isValidBase64(sanitized)) {
    throw new Error('Base64データが不正です。');
  }

  try {
    return atob(sanitized);
  } catch (e) {
    console.error('[base64] Decode failed:', e);
    throw new Error('Base64デコードに失敗しました。');
  }
}
