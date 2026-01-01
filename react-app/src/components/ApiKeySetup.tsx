/**
 * APIキー設定コンポーネント
 */
import { useState, useEffect } from 'react';
import { hashPassword } from '../utils/crypto';
import {
  hasEncryptedApiKey,
  setApiKeyEncrypted,
  loadApiKeyEncrypted,
  clearApiKey,
  getMasterHashKey,
} from '../services/apiKey';
import './ApiKeySetup.css';

interface Props {
  onComplete: (apiKey: string) => void;
  onCancel?: () => void;
}

type Mode = 'check' | 'new' | 'unlock';

export function ApiKeySetup({ onComplete, onCancel }: Props) {
  const [mode, setMode] = useState<Mode>('check');
  const [apiKey, setApiKey] = useState('');
  const [masterPassword, setMasterPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setMode(hasEncryptedApiKey() ? 'unlock' : 'new');
  }, []);

  const isValidKey = apiKey.trim().startsWith('AIza') && apiKey.trim().length >= 39;
  const isValidPassword = masterPassword.length >= 4;
  const passwordsMatch = masterPassword === confirmPassword;

  const handleUnlock = async () => {
    if (!masterPassword) {
      setError('パスワードを入力してください');
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
        else setError('キーの読み込みに失敗');
      } else {
        setError('パスワードが違います');
      }
    } catch {
      setError('復号に失敗しました');
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async () => {
    if (!isValidKey) {
      setError('APIキーの形式が不正です（AIza...で始まる39文字以上）');
      return;
    }
    if (!isValidPassword) {
      setError('パスワードは4文字以上');
      return;
    }
    if (!passwordsMatch) {
      setError('パスワードが一致しません');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const hash = await hashPassword(masterPassword);
      localStorage.setItem(getMasterHashKey(), hash);
      await setApiKeyEncrypted(apiKey.trim(), masterPassword);
      onComplete(apiKey.trim());
    } catch (e: unknown) {
      setError('保存に失敗: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setLoading(false);
    }
  };

  const handleReset = () => {
    if (window.confirm('保存されたAPIキーを削除しますか？')) {
      clearApiKey();
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

      {mode === 'unlock' && (
        <div className="setup-form">
          <p>暗号化されたAPIキーが保存されています。</p>
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
            <button onClick={handleReset} className="reset-btn">
              リセット
            </button>
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
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="AIza..."
          />
          {apiKey && !isValidKey && (
            <span className="hint error">AIza...で始まる39文字以上</span>
          )}
          {isValidKey && <span className="hint ok">OK</span>}

          <div className="step">
            <span className="step-num">3</span>
            <span>マスターパスワードを設定</span>
          </div>
          <input
            type="password"
            value={masterPassword}
            onChange={(e) => setMasterPassword(e.target.value)}
            placeholder="パスワード（4文字以上）"
          />
          <input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            placeholder="パスワード（確認）"
          />
          {masterPassword && confirmPassword && !passwordsMatch && (
            <span className="hint error">パスワードが一致しません</span>
          )}

          <div className="button-row">
            <button
              onClick={handleSubmit}
              disabled={loading || !isValidKey || !isValidPassword || !passwordsMatch}
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
