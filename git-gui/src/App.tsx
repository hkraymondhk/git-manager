import { RepoHeader } from './components/RepoHeader';
import { RepoOpenDialog } from './components/RepoOpenDialog';
import { ChangesPanel } from './components/ChangesPanel';
import { DiffViewer } from './components/DiffViewer';
import { CommitForm } from './components/CommitForm';
import { CommitList } from './components/CommitList';
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
          <div style={styles.workspace}>
            {/* Left panel: Changes */}
            <div style={styles.leftPanel}>
              <ChangesPanel />
              <CommitForm />
            </div>
            
            {/* Middle panel: Diff viewer */}
            <div style={styles.middlePanel}>
              <DiffViewer />
            </div>
            
            {/* Right panel: Commit history */}
            <div style={styles.rightPanel}>
              <CommitList />
            </div>
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
    overflow: 'hidden',
    backgroundColor: '#fff',
  },
  workspace: {
    display: 'flex',
    height: '100%',
  },
  leftPanel: {
    width: '280px',
    display: 'flex',
    flexDirection: 'column',
    borderRight: '1px solid #d0d7de',
  },
  middlePanel: {
    flex: 1,
    display: 'flex',
    flexDirection: 'column',
    borderRight: '1px solid #d0d7de',
  },
  rightPanel: {
    width: '350px',
    display: 'flex',
    flexDirection: 'column',
  },
};

export default App;
