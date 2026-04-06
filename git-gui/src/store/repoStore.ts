import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface RepoState {
  currentRepoPath: string | null;
  isLoading: boolean;
  error: string | null;
  
  openRepository: (path: string) => Promise<void>;
  closeRepository: () => Promise<void>;
  refreshRepository: () => Promise<void>;
}

export const useRepoStore = create<RepoState>((set, get) => ({
  currentRepoPath: null,
  isLoading: false,
  error: null,

  openRepository: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('open_repository', { path });
      set({ currentRepoPath: path, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  closeRepository: async () => {
    try {
      await invoke('close_repository');
      set({ currentRepoPath: null, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  refreshRepository: async () => {
    // Placeholder for future refresh logic
    console.log('Refreshing repository...');
  },
}));
