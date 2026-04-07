import { useState } from 'react';
import { useRepoStore, FileStatus } from '../store/repoStore';
import { ChevronRight, ChevronDown, File, Plus, Minus, Disc } from 'lucide-react';

interface FileTreeNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children?: FileTreeNode[];
  status?: FileStatus;
}

export function ChangesPanel() {
  const { repoStatus, stageFile, unstageFile, setSelectedFile, selectedFile, discardChanges } = useRepoStore();
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set(['']));

  if (!repoStatus) {
    return <div style={styles.empty}>No repository loaded</div>;
  }

  const { files, branch, ahead, behind } = repoStatus;

  // Build tree structure
  const buildTree = (fileList: FileStatus[]): FileTreeNode[] => {
    const root: FileTreeNode[] = [];
    
    for (const file of fileList) {
      const parts = file.path.split('/');
      let currentLevel = root;
      let currentPath = '';
      
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        currentPath = currentPath ? `${currentPath}/${part}` : part;
        const isLast = i === parts.length - 1;
        
        const existing = currentLevel.find(node => node.name === part);
        if (existing) {
          if (!isLast) {
            currentLevel = existing.children || [];
          } else {
            existing.status = file;
          }
        } else {
          const newNode: FileTreeNode = {
            name: part,
            path: currentPath,
            isDirectory: !isLast,
            children: !isLast ? [] : undefined,
            status: isLast ? file : undefined,
          };
          currentLevel.push(newNode);
          if (!isLast) {
            currentLevel = newNode.children!;
          }
        }
      }
    }
    
    return root;
  };

  const tree = buildTree(files);

  const toggleDir = (path: string) => {
    const newExpanded = new Set(expandedDirs);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    setExpandedDirs(newExpanded);
  };

  const renderNode = (node: FileTreeNode, depth: number) => {
    const isSelected = selectedFile === node.path;
    const isExpanded = expandedDirs.has(node.path);
    
    if (node.isDirectory) {
      return (
        <div key={node.path}>
          <div
            style={{
              ...styles.fileRow,
              paddingLeft: `${depth * 16 + 8}px`,
              backgroundColor: isSelected ? '#0969da' : 'transparent',
            }}
            onClick={(e) => {
              e.stopPropagation();
              toggleDir(node.path);
            }}
          >
            {isExpanded ? (
              <ChevronDown size={16} style={styles.icon} />
            ) : (
              <ChevronRight size={16} style={styles.icon} />
            )}
            <File size={16} style={styles.icon} />
            <span style={{
              ...styles.fileName,
              color: isSelected ? '#fff' : '#24292f',
            }}>{node.name}</span>
          </div>
          {isExpanded && node.children && node.children.map(child => renderNode(child, depth + 1))}
        </div>
      );
    } else {
      const statusColor = node.status?.staged ? '#3fb950' : '#d29922';
      const statusIcon = node.status?.staged ? Plus : Disc;
      
      return (
        <div
          key={node.path}
          style={{
            ...styles.fileRow,
            paddingLeft: `${depth * 16 + 8}px`,
            backgroundColor: isSelected ? '#0969da' : 'transparent',
          }}
          onClick={() => setSelectedFile(node.path)}
        >
          <span style={{ width: 16 }} />
          {statusIcon({ size: 16, color: statusColor, style: styles.icon })}
          <span style={{
            ...styles.fileName,
            color: isSelected ? '#fff' : '#24292f',
          }}>{node.name}</span>
          <span style={{
            ...styles.statusText,
            color: isSelected ? '#fff' : statusColor,
          }}>
            {node.status?.staged ? 'Staged' : node.status?.status}
          </span>
          <div style={styles.actions}>
            {node.status?.staged ? (
              <button
                style={styles.actionBtn}
                onClick={(e) => {
                  e.stopPropagation();
                  unstageFile(node.path);
                }}
                title="Unstage"
              >
                <Minus size={14} />
              </button>
            ) : (
              <>
                <button
                  style={styles.actionBtn}
                  onClick={(e) => {
                    e.stopPropagation();
                    stageFile(node.path);
                  }}
                  title="Stage"
                >
                  <Plus size={14} />
                </button>
                <button
                  style={styles.actionBtn}
                  onClick={(e) => {
                    e.stopPropagation();
                    discardChanges(node.path);
                  }}
                  title="Discard"
                >
                  <Disc size={14} />
                </button>
              </>
            )}
          </div>
        </div>
      );
    }
  };

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <div style={styles.branchInfo}>
          <span style={styles.branch}>{branch || 'HEAD'}</span>
          {ahead > 0 && <span style={styles.aheadBehind}>↑{ahead}</span>}
          {behind > 0 && <span style={styles.aheadBehind}>↓{behind}</span>}
        </div>
        <span style={styles.fileCount}>{files.length} changed</span>
      </div>
      
      {files.length === 0 ? (
        <div style={styles.empty}>No changes</div>
      ) : (
        <div style={styles.list}>
          {tree.map(node => renderNode(node, 0))}
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
    borderRight: '1px solid #d0d7de',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '12px 16px',
    borderBottom: '1px solid #d0d7de',
    backgroundColor: '#f6f8fa',
  },
  branchInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  branch: {
    fontWeight: 600,
    fontSize: '13px',
    color: '#24292f',
  },
  aheadBehind: {
    fontSize: '11px',
    padding: '2px 6px',
    borderRadius: '10px',
    backgroundColor: '#ddf4ff',
    color: '#0969da',
  },
  fileCount: {
    fontSize: '12px',
    color: '#57606a',
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
  fileRow: {
    display: 'flex',
    alignItems: 'center',
    padding: '6px 8px',
    cursor: 'pointer',
    borderBottom: '1px solid #ebeced',
  },
  icon: {
    marginRight: '6px',
  },
  fileName: {
    flex: 1,
    fontSize: '13px',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  statusText: {
    fontSize: '11px',
    marginRight: '8px',
    textTransform: 'capitalize',
  },
  actions: {
    display: 'flex',
    gap: '4px',
    opacity: 0,
    transition: 'opacity 0.15s',
  },
  actionBtn: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '24px',
    height: '24px',
    padding: 0,
    border: 'none',
    borderRadius: '4px',
    backgroundColor: 'transparent',
    cursor: 'pointer',
    color: '#57606a',
  },
};

// Add hover effect for actions
const hoverStyles = `
  .file-row:hover .actions {
    opacity: 1;
  }
`;
