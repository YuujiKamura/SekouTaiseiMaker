/**
 * APIキー設定コンポーネント（パスキー対応）
 */
import { useState, useEffect } from 'react';
import { hashPassword } from '../utils/crypto';
import {
  hasEncryptedApiKey,
  setApiKeyEncrypted,
  loadApiKeyEncrypted,
  clearApiKey,
  getMasterHashKey,
  setApiKey as saveApiKey,
} from '../services/apiKey';
import {
  isBiometricAvailable,
  hasRegisteredPasskey,
  registerPasskey,
  authenticateWithPasskey,
  removePasskey,
} from '../services/webAuthn';
import './ApiKeySetup.css';

interface Props {
  onComplete: (apiKey: string) => void;
  onCancel?: () => void;
}

type Mode = 'check' | 'passkey' | 'unlock' | 'new';

export function ApiKeySetup({ onComplete, onCancel }: Props) {
  const [mode, setMode] = useState<Mode>('check');
  const [apiKey, setApiKey] = useState('');
  const [masterPassword, setMasterPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [biometricAvailable, setBiometricAvailable] = useState(false);
  const [registerBiometric, setRegisterBiometric] = useState(false);

  useEffect(() => {
    const init = async () => {
      const bioAvailable = await isBiometricAvailable();
      setBiometricAvailable(bioAvailable);

      if (hasRegisteredPasskey()) {
        setMode('passkey');
      } else if (hasEncryptedApiKey()) {
        setMode('unlock');
      } else {
        setMode('new');
      }
    };
    init();
  }, []);

  const isValidKey = apiKey.trim().startsWith('AIza') && apiKey.trim().length >= 39;
  const isValidPassword = masterPassword.length >= 4;
  const passwordsMatch = masterPassword === confirmPassword;

  // パスキー認証
  const handlePasskeyAuth = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await authenticateWithPasskey();
      if (result.success && result.apiKey) {
        saveApiKey(result.apiKey);
        onComplete(result.apiKey);
      } else {
        setError(result.error || '認証失敗');
      }
    } catch {
      setError('パスキー認証に失敗');
    } finally {
      setLoading(false);
    }
  };

  // パスワードでアンロック
  const handleUnlock = async () => {
    if (!masterPassword) {
      setError('パスワードを入力');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const success = await loadApiKeyEncrypted(masterPassword);
      if (success) {
        const { getApiKey } = await import('../services/apiKey');
        const key = getApiKey();
        if (key) onComplete(key);
        else setError('キー読み込み失敗');
      } else {
        setError('パスワードが違います');
      }
    } catch {
      setError('復号失敗');
    } finally {
      setLoading(false);
    }
  };

  // 新規設定
  const handleSubmit = async () => {
    if (!isValidKey || !isValidPassword || !passwordsMatch) {
      setError('入力を確認してください');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const trimmedKey = apiKey.trim();

      // パスキー登録が選択されている場合
      if (registerBiometric && biometricAvailable) {
        const result = await registerPasskey(trimmedKey);
        if (result.success) {
          saveApiKey(trimmedKey);
          onComplete(trimmedKey);
          return;
        } else {
          // パスキー登録失敗時はパスワード方式にフォールバック
          console.warn('Passkey registration failed, falling back to password:', result.error);
        }
      }

      // パスワード方式で保存
      const hash = await hashPassword(masterPassword);
      localStorage.setItem(getMasterHashKey(), hash);
      await setApiKeyEncrypted(trimmedKey, masterPassword);
      onComplete(trimmedKey);
    } catch (e: unknown) {
      setError('保存失敗: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setLoading(false);
    }
  };

  // リセット
  const handleReset = () => {
    if (window.confirm('保存されたAPIキーを削除しますか？')) {
      clearApiKey();
      removePasskey();
      setMode('new');
      setMasterPassword('');
      setConfirmPassword('');
      setApiKey('');
      setError('');
    }
  };

  if (mode === 'check') {
    return <div className="api-key-setup">読み込み中...</div>;
  }

  return (
    <div className="api-key-setup">
      <h2>APIキー設定</h2>

      {/* パスキーモード */}
      {mode === 'passkey' && (
        <div className="setup-form">
          <p>指紋/顔認証でログイン</p>
          <div className="button-row">
            <button onClick={handlePasskeyAuth} disabled={loading} className="passkey-btn">
              {loading ? '認証中...' : '認証する'}
            </button>
          </div>
          <div className="alt-login">
            <button onClick={() => setMode('unlock')} className="link-btn">
              パスワードでログイン
            </button>
            <button onClick={handleReset} className="link-btn reset">
              リセット
            </button>
          </div>
        </div>
      )}

      {/* パスワードアンロックモード */}
      {mode === 'unlock' && (
        <div className="setup-form">
          <p>マスターパスワードを入力</p>
          <input
            type="password"
            value={masterPassword}
            onChange={(e) => setMasterPassword(e.target.value)}
            placeholder="マスターパスワード"
            onKeyDown={(e) => e.key === 'Enter' && handleUnlock()}
          />
          <div className="button-row">
            <button onClick={handleUnlock} disabled={loading}>
              {loading ? '処理中...' : 'アンロック'}
            </button>
          </div>
          <div className="alt-login">
            {hasRegisteredPasskey() && (
              <button onClick={() => setMode('passkey')} className="link-btn">
                パスキーでログイン
              </button>
            )}
            <button onClick={handleReset} className="link-btn reset">
              リセット
            </button>
          </div>
        </div>
      )}

      {/* 新規設定モード */}
      {mode === 'new' && (
        <div className="setup-form">
          <div className="step">
            <span className="step-num">1</span>
            <a href="https://aistudio.google.com/apikey" target="_blank" rel="noopener noreferrer">
              Google AI StudioでAPIキーを取得
            </a>
          </div>

          <div className="step">
            <span className="step-num">2</span>
            <span>APIキーを貼り付け</span>
          </div>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="AIza..."
          />
          {apiKey && !isValidKey && <span className="hint error">AIza...で始まる39文字以上</span>}
          {isValidKey && <span className="hint ok">OK</span>}

          <div className="step">
            <span className="step-num">3</span>
            <span>保護方法を選択</span>
          </div>

          {biometricAvailable && (
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={registerBiometric}
                onChange={(e) => setRegisterBiometric(e.target.checked)}
              />
              <span>指紋/顔認証を使用（推奨）</span>
            </label>
          )}

          {!registerBiometric && (
            <>
              <input
                type="password"
                value={masterPassword}
                onChange={(e) => setMasterPassword(e.target.value)}
                placeholder="マスターパスワード（4文字以上）"
              />
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="パスワード確認"
              />
              {masterPassword && confirmPassword && !passwordsMatch && (
                <span className="hint error">パスワードが一致しません</span>
              )}
            </>
          )}

          <div className="button-row">
            <button
              onClick={handleSubmit}
              disabled={loading || !isValidKey || (!registerBiometric && (!isValidPassword || !passwordsMatch))}
            >
              {loading ? '保存中...' : '保存'}
            </button>
            {onCancel && (
              <button onClick={onCancel} className="cancel-btn">
                キャンセル
              </button>
            )}
          </div>
        </div>
      )}

      {error && <div className="error-message">{error}</div>}
    </div>
  );
}
