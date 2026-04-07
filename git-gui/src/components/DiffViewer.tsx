import { useState } from 'react';
import { useRepoStore } from '../store/repoStore';
import Editor from '@monaco-editor/react';

export function DiffViewer() {
  const { selectedFile, diffContent, setSelectedFile } = useRepoStore();
  const [viewMode, setViewMode] = useState<'diff' | 'code'>('diff');

  if (!selectedFile) {
    return (
      <div style={styles.empty}>
        <p>Select a file to view changes</p>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <span style={styles.filename}>{selectedFile}</span>
        <div style={styles.viewToggle}>
          <button
            style={{
              ...styles.toggleBtn,
              backgroundColor: viewMode === 'diff' ? '#0969da' : '#f6f8fa',
              color: viewMode === 'diff' ? '#fff' : '#24292f',
            }}
            onClick={() => setViewMode('diff')}
          >
            Diff
          </button>
          <button
            style={{
              ...styles.toggleBtn,
              backgroundColor: viewMode === 'code' ? '#0969da' : '#f6f8fa',
              color: viewMode === 'code' ? '#fff' : '#24292f',
            }}
            onClick={() => setViewMode('code')}
          >
            Code
          </button>
        </div>
      </div>
      
      <div style={styles.editorContainer}>
        {viewMode === 'diff' ? (
          <Editor
            height="100%"
            defaultLanguage="diff"
            value={diffContent || 'No changes detected'}
            theme="vs-light"
            options={{
              readOnly: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              wordWrap: 'on',
              lineNumbers: 'off',
              glyphMargin: false,
              folding: false,
              lineDecorationsWidth: 0,
              lineNumbersMinChars: 0,
            }}
          />
        ) : (
          <Editor
            height="100%"
            defaultLanguage="typescript"
            value={diffContent}
            theme="vs-light"
            options={{
              readOnly: true,
              minimap: { enabled: true },
              scrollBeyondLastLine: false,
              wordWrap: 'on',
            }}
          />
        )}
      </div>
    </div>
  );
}

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: '#fff',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '12px 16px',
    borderBottom: '1px solid #d0d7de',
    backgroundColor: '#f6f8fa',
  },
  filename: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#24292f',
    fontFamily: 'ui-monospace, SFMono-Regular, monospace',
  },
  viewToggle: {
    display: 'flex',
    gap: '4px',
  },
  toggleBtn: {
    padding: '4px 12px',
    fontSize: '12px',
    border: '1px solid #d0d7de',
    borderRadius: '6px',
    cursor: 'pointer',
    transition: 'all 0.15s',
  },
  editorContainer: {
    flex: 1,
    overflow: 'hidden',
  },
  empty: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    color: '#8b949e',
    fontSize: '14px',
  },
};
