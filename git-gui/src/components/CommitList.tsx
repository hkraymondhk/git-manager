import { useRepoStore } from '../store/repoStore';
import { formatDistanceToNow } from 'date-fns';

export function CommitList() {
  const { commits, loadCommitHistory } = useRepoStore();

  const formatDate = (timestamp: number) => {
    try {
      return formatDistanceToNow(new Date(timestamp * 1000), { addSuffix: true });
    } catch {
      return '';
    }
  };

  const getShortHash = (hash: string) => hash.substring(0, 7);

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h3 style={styles.title}>Commit History</h3>
        <button onClick={() => loadCommitHistory()} style={styles.refreshBtn}>
          Refresh
        </button>
      </div>
      
      {commits.length === 0 ? (
        <div style={styles.empty}>No commits found</div>
      ) : (
        <div style={styles.list}>
          {commits.map((commit) => (
            <div key={commit.id} style={styles.commit}>
              <div style={styles.commitHeader}>
                <span style={styles.hash}>{getShortHash(commit.id)}</span>
                <span style={styles.time}>{formatDate(commit.timestamp)}</span>
              </div>
              <div style={styles.message}>{commit.message}</div>
              <div style={styles.author}>
                {commit.author} {commit.email ? `<${commit.email}>` : ''}
              </div>
            </div>
          ))}
        </div>
      )}
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
  title: {
    margin: 0,
    fontSize: '14px',
    fontWeight: 600,
    color: '#24292f',
  },
  refreshBtn: {
    padding: '4px 12px',
    fontSize: '12px',
    backgroundColor: '#f6f8fa',
    border: '1px solid #d0d7de',
    borderRadius: '6px',
    cursor: 'pointer',
    color: '#24292f',
  },
  empty: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flex: 1,
    color: '#8b949e',
    fontSize: '14px',
  },
  list: {
    flex: 1,
    overflow: 'auto',
  },
  commit: {
    padding: '12px 16px',
    borderBottom: '1px solid #ebeced',
    cursor: 'pointer',
  },
  commitHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: '4px',
  },
  hash: {
    fontFamily: 'ui-monospace, SFMono-Regular, monospace',
    fontSize: '12px',
    color: '#57606a',
    backgroundColor: '#f6f8fa',
    padding: '2px 6px',
    borderRadius: '4px',
  },
  time: {
    fontSize: '12px',
    color: '#8b949e',
  },
  message: {
    fontSize: '14px',
    color: '#24292f',
    marginBottom: '4px',
    wordBreak: 'break-word',
  },
  author: {
    fontSize: '12px',
    color: '#57606a',
  },
};
