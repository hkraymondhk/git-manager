import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface CommitInfo {
  id: string;
  message: string;
  author: string;
  email: string;
  timestamp: number;
}

export interface FileStatus {
  path: string;
  status: string;
  staged: boolean;
}

export interface RepoStatus {
  branch: string | null;
  ahead: number;
  behind: number;
  files: FileStatus[];
}

interface RepoState {
  currentRepoPath: string | null;
  isLoading: boolean;
  error: string | null;
  
  // Repository state
  commits: CommitInfo[];
  repoStatus: RepoStatus | null;
  selectedFile: string | null;
  diffContent: string;
  
  // Actions
  openRepository: (path: string) => Promise<void>;
  closeRepository: () => Promise<void>;
  loadCommitHistory: () => Promise<void>;
  loadRepoStatus: () => Promise<void>;
  stageFile: (path: string) => Promise<void>;
  unstageFile: (path: string) => Promise<void>;
  createCommit: (message: string) => Promise<void>;
  loadDiff: (path: string) => Promise<void>;
  discardChanges: (path: string) => Promise<void>;
  setSelectedFile: (path: string | null) => void;
}

export const useRepoStore = create<RepoState>((set, get) => ({
  currentRepoPath: null,
  isLoading: false,
  error: null,
  
  // Repository state
  commits: [],
  repoStatus: null,
  selectedFile: null,
  diffContent: '',
  
  openRepository: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('open_repository', { path });
      set({ currentRepoPath: path, isLoading: false });
      // Load initial data
      await get().loadCommitHistory();
      await get().loadRepoStatus();
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  closeRepository: async () => {
    try {
      await invoke('close_repository');
      set({ 
        currentRepoPath: null, 
        error: null,
        commits: [],
        repoStatus: null,
        selectedFile: null,
        diffContent: '',
      });
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  loadCommitHistory: async () => {
    try {
      const commits = await invoke<CommitInfo[]>('get_commit_log', { 
        options: { limit: 200, offset: 0 }
      });
      set({ commits });
    } catch (err) {
      console.error('Failed to load commit history:', err);
      set({ error: String(err) });
    }
  },
  
  loadRepoStatus: async () => {
    try {
      const status = await invoke<RepoStatus>('get_repo_status');
      set({ repoStatus: status });
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  stageFile: async (path: string) => {
    try {
      await invoke('stage_file', { path });
      await get().loadRepoStatus();
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  unstageFile: async (path: string) => {
    try {
      await invoke('unstage_file', { path });
      await get().loadRepoStatus();
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  createCommit: async (message: string) => {
    try {
      await invoke('create_commit', { message });
      await get().loadCommitHistory();
      await get().loadRepoStatus();
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  loadDiff: async (path: string) => {
    try {
      const diff = await invoke<string>('get_diff', { path });
      set({ diffContent: diff });
    } catch (err) {
      set({ error: String(err), diffContent: '' });
    }
  },
  
  discardChanges: async (path: string) => {
    try {
      await invoke('discard_changes', { path });
      await get().loadRepoStatus();
      if (get().selectedFile === path) {
        await get().loadDiff(path);
      }
    } catch (err) {
      set({ error: String(err) });
    }
  },
  
  setSelectedFile: (path: string | null) => {
    set({ selectedFile: path });
    if (path) {
      get().loadDiff(path);
    } else {
      set({ diffContent: '' });
    }
  },
}));
