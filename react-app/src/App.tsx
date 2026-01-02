import { PdfEditor } from './components/PdfEditor'
import { PdfViewer } from './components/PdfViewer'
import { ApiKeySetup } from './components/ApiKeySetup'
import { AiChecker } from './components/AiChecker'
import { SpreadsheetChecker } from './components/SpreadsheetChecker'

function App() {
  const params = new URLSearchParams(window.location.search)
  const mode = params.get('mode')

  if (mode === 'apikey') {
    return (
      <ApiKeySetup
        onComplete={() => {
          window.parent.postMessage({ type: 'apikey-setup-complete' }, '*')
        }}
        onCancel={() => {
          window.parent.postMessage({ type: 'apikey-setup-cancel' }, '*')
        }}
      />
    )
  }

  if (mode === 'check') {
    return <AiChecker />
  }

  if (mode === 'spreadsheet-check') {
    return <SpreadsheetChecker />
  }

  if (mode === 'view') {
    return <PdfViewer />
  }

  return <PdfEditor />
}

export default App
