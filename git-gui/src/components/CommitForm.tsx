import { useState } from 'react';
import { useRepoStore } from '../store/repoStore';

export function CommitForm() {
  const { createCommit, repoStatus } = useRepoStore();
  const [message, setMessage] = useState('');
  const [isCommitting, setIsCommitting] = useState(false);

  const stagedFiles = repoStatus?.files.filter(f => f.staged) || [];
  const canCommit = message.trim() && stagedFiles.length > 0;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canCommit) return;

    setIsCommitting(true);
    try {
      await createCommit(message.trim());
      setMessage('');
    } finally {
      setIsCommitting(false);
    }
  };

  return (
    <div style={styles.container}>
      <form onSubmit={handleSubmit} style={styles.form}>
        <textarea
          style={styles.textarea}
          placeholder="Commit message (required)"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          rows={3}
        />
        <div style={styles.footer}>
          <div style={styles.info}>
            <span style={styles.count}>{stagedFiles.length} staged files</span>
          </div>
          <button
            type="submit"
            disabled={!canCommit || isCommitting}
            style={{
              ...styles.button,
              opacity: canCommit && !isCommitting ? 1 : 0.6,
              cursor: canCommit && !isCommitting ? 'pointer' : 'not-allowed',
            }}
          >
            {isCommitting ? 'Committing...' : 'Commit'}
          </button>
        </div>
      </form>
    </div>
  );
}

const styles: { [key: string]: React.CSSProperties } = {
  container: {
    padding: '12px 16px',
    borderTop: '1px solid #d0d7de',
    backgroundColor: '#f6f8fa',
  },
  form: {
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
  textarea: {
    width: '100%',
    padding: '8px 12px',
    fontSize: '13px',
    border: '1px solid #d0d7de',
    borderRadius: '6px',
    resize: 'none',
    fontFamily: 'inherit',
    outline: 'none',
  },
  footer: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  info: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  count: {
    fontSize: '12px',
    color: '#57606a',
  },
  button: {
    padding: '6px 16px',
    fontSize: '13px',
    fontWeight: 500,
    color: '#fff',
    backgroundColor: '#2da44e',
    border: '1px solid rgba(27,31,36,0.15)',
    borderRadius: '6px',
    transition: 'all 0.15s',
  },
};
