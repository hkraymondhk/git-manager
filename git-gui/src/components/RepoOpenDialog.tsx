import { useState } from 'react';
import { FolderOpen } from 'lucide-react';
import { useRepoStore } from '../store/repoStore';

export function RepoOpenDialog() {
  const [path, setPath] = useState('');
  const { openRepository, isLoading } = useRepoStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (path.trim()) {
      await openRepository(path.trim());
    }
  };

  return (
    <div style={styles.container}>
      <form onSubmit={handleSubmit} style={styles.form}>
        <div style={styles.iconWrapper}>
          <FolderOpen size={48} color="#57606a" />
        </div>
        <h2 style={styles.title}>Open Git Repository</h2>
        <p style={styles.subtitle}>Enter the path to your Git repository</p>
        
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/path/to/your/repository"
          style={styles.input}
        />
        
        <button 
          type="submit" 
          disabled={isLoading || !path.trim()}
          style={{
            ...styles.button,
            ...(isLoading || !path.trim() ? styles.buttonDisabled : {}),
          }}
        >
          {isLoading ? 'Opening...' : 'Open Repository'}
        </button>
      </form>
    </div>
  );
}

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    flex: 1,
    padding: '40px',
  },
  form: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: '16px',
    maxWidth: '400px',
    width: '100%',
  },
  iconWrapper: {
    marginBottom: '8px',
  },
  title: {
    fontSize: '20px',
    fontWeight: 600,
    margin: 0,
  },
  subtitle: {
    fontSize: '14px',
    color: '#57606a',
    margin: 0,
  },
  input: {
    width: '100%',
    padding: '10px 12px',
    fontSize: '14px',
    border: '1px solid #d0d7de',
    borderRadius: '6px',
    outline: 'none',
  },
  button: {
    width: '100%',
    padding: '10px 16px',
    fontSize: '14px',
    fontWeight: 500,
    color: '#fff',
    backgroundColor: '#2da44e',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  buttonDisabled: {
    backgroundColor: '#94d3a2',
    cursor: 'not-allowed',
  },
};
