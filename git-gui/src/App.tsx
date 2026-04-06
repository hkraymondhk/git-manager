import { RepoHeader } from './components/RepoHeader';
import { RepoOpenDialog } from './components/RepoOpenDialog';
import { useRepoStore } from './store/repoStore';

function App() {
  const { currentRepoPath } = useRepoStore();

  return (
    <div style={styles.app}>
      <RepoHeader />
      <main style={styles.main}>
        {!currentRepoPath ? (
          <RepoOpenDialog />
        ) : (
          <div style={styles.placeholder}>
            <p>Repository loaded: {currentRepoPath}</p>
            <p style={styles.hint}>More Git features coming soon...</p>
          </div>
        )}
      </main>
    </div>
  );
}

const styles: { [key: string]: React.CSSProperties } = {
  app: {
    display: 'flex',
    flexDirection: 'column',
    height: '100vh',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif',
  },
  main: {
    flex: 1,
    overflow: 'auto',
    backgroundColor: '#fff',
  },
  placeholder: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    color: '#57606a',
  },
  hint: {
    fontSize: '14px',
    color: '#8b949e',
  },
};

export default App;
