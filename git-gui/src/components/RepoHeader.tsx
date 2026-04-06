import { FolderOpen, GitBranch, X } from 'lucide-react';
import { useRepoStore } from '../store/repoStore';

export function RepoHeader() {
  const { currentRepoPath, closeRepository } = useRepoStore();

  return (
    <div style={styles.header}>
      <div style={styles.left}>
        <GitBranch size={20} />
        <span style={styles.title}>Git GUI</span>
      </div>
      
      {currentRepoPath && (
        <div style={styles.repoInfo}>
          <FolderOpen size={16} />
          <span style={styles.path}>{currentRepoPath}</span>
          <button onClick={closeRepository} style={styles.closeBtn}>
            <X size={16} />
          </button>
        </div>
      )}
    </div>
  );
}

const styles: { [key: string]: React.CSSProperties } = {
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px 16px',
    borderBottom: '1px solid #e1e4e8',
    backgroundColor: '#f6f8fa',
  },
  left: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  title: {
    fontSize: '16px',
    fontWeight: 600,
  },
  repoInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    padding: '6px 12px',
    backgroundColor: '#fff',
    borderRadius: '6px',
    border: '1px solid #e1e4e8',
  },
  path: {
    fontSize: '13px',
    color: '#57606a',
    maxWidth: '400px',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  closeBtn: {
    background: 'none',
    border: 'none',
    cursor: 'pointer',
    padding: '4px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: '4px',
  },
};
