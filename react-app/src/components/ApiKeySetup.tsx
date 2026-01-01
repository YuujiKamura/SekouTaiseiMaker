/**
 * APIキー設定コンポーネント（パスキー対応）
 */
import { useState, useEffect } from 'react';
import { getApiKey, setApiKey, clearApiKey } from '../services/apiKey';
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

type Mode = 'loading' | 'auth' | 'new' | 'done';

export function ApiKeySetup({ onComplete, onCancel }: Props) {
  const [mode, setMode] = useState<Mode>('loading');
  const [apiKey, setApiKeyValue] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [biometricAvailable, setBiometricAvailable] = useState(false);

  useEffect(() => {
    const init = async () => {
      const bioAvail = await isBiometricAvailable();
      setBiometricAvailable(bioAvail);

      if (hasRegisteredPasskey()) {
        setMode('auth');
      } else if (getApiKey()) {
        // 既にAPIキーがある
        setMode('auth');
      } else {
        setMode('new');
      }
    };
    init();
  }, []);

  const isValidKey = apiKey.trim().startsWith('AIza') && apiKey.trim().length >= 39;

  // パスキー認証
  const handlePasskeyAuth = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await authenticateWithPasskey();
      if (result.success && result.apiKey) {
        setApiKey(result.apiKey);
        setMode('done');
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

  // 既存キーを使用
  const handleUseExisting = () => {
    const key = getApiKey();
    if (key) {
      setMode('done');
      onComplete(key);
    }
  };

  // パスキーで保存
  const handleSaveWithPasskey = async () => {
    if (!isValidKey) return;
    setLoading(true);
    setError('');
    try {
      const trimmedKey = apiKey.trim();
      const result = await registerPasskey(trimmedKey);
      if (result.success) {
        setApiKey(trimmedKey);
        setMode('done');
        onComplete(trimmedKey);
      } else {
        setError(result.error || 'パスキー登録失敗');
      }
    } catch (e) {
      setError('エラー: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setLoading(false);
    }
  };

  // 通常保存
  const handleSaveNormal = () => {
    if (!isValidKey) return;
    const trimmedKey = apiKey.trim();
    setApiKey(trimmedKey);
    setMode('done');
    onComplete(trimmedKey);
  };

  // リセット
  const handleReset = () => {
    if (window.confirm('保存されたAPIキーを削除しますか？')) {
      clearApiKey();
      removePasskey();
      setMode('new');
      setApiKeyValue('');
      setError('');
    }
  };

  if (mode === 'loading') {
    return <div className="api-key-setup"><p>読み込み中...</p></div>;
  }

  if (mode === 'done') {
    return (
      <div className="api-key-setup">
        <h2>APIキー設定</h2>
        <div className="setup-form">
          <p className="success-message">設定完了</p>
          <div className="button-row">
            <button onClick={() => window.parent.postMessage({ type: 'apikey-setup-complete' }, '*')}>
              閉じる
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="api-key-setup">
      <h2>APIキー設定</h2>

      {mode === 'auth' && (
        <div className="setup-form">
          {hasRegisteredPasskey() ? (
            <>
              <p>パスキーで認証</p>
              <div className="button-row">
                <button onClick={handlePasskeyAuth} disabled={loading} className="passkey-btn">
                  {loading ? '認証中...' : '指紋/顔認証'}
                </button>
              </div>
            </>
          ) : (
            <>
              <p>APIキー設定済み</p>
              <p className="key-preview">AIza...{getApiKey()?.slice(-4)}</p>
              <div className="button-row">
                <button onClick={handleUseExisting}>このまま使用</button>
              </div>
            </>
          )}
          <div className="alt-login">
            <button onClick={handleReset} className="link-btn reset">リセット</button>
          </div>
        </div>
      )}

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
            onChange={(e) => setApiKeyValue(e.target.value)}
            placeholder="AIza..."
            autoFocus
          />
          {apiKey && !isValidKey && <span className="hint error">AIza...で始まる39文字以上</span>}
          {isValidKey && <span className="hint ok">OK</span>}

          <div className="button-row">
            {biometricAvailable ? (
              <button onClick={handleSaveWithPasskey} disabled={!isValidKey || loading} className="passkey-btn">
                {loading ? '登録中...' : 'パスキーで保存'}
              </button>
            ) : (
              <button onClick={handleSaveNormal} disabled={!isValidKey}>
                保存
              </button>
            )}
            {onCancel && <button onClick={onCancel} className="cancel-btn">キャンセル</button>}
          </div>
          {biometricAvailable && (
            <div className="alt-login">
              <button onClick={handleSaveNormal} disabled={!isValidKey} className="link-btn">
                パスキーなしで保存
              </button>
            </div>
          )}
        </div>
      )}

      {error && <div className="error-message">{error}</div>}
    </div>
  );
}
