import {
  EngineEvalResponse,
  PuzzleWithReview,
  SolveAttemptRequest,
  SolveResponse,
  StatsSummary,
  SyncRequest,
  SyncStatus,
  User,
  ValidateMoveRequest,
  ValidateMoveResponse,
} from '../types';

const API_BASE = '/api';

export const api = {
  async getOrCreateUser(username: string): Promise<User> {
    const res = await fetch(`${API_BASE}/users/profile?username=${encodeURIComponent(username)}`);
    if (!res.ok) {
      const err = await res.text();
      throw new Error(err || 'Failed to fetch user');
    }
    return res.json();
  },

  async startSync(req: SyncRequest): Promise<SyncStatus> {
    const res = await fetch(`${API_BASE}/sync`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!res.ok) {
      const err = await res.text();
      throw new Error(err || 'Failed to start sync');
    }
    return res.json();
  },

  async getSyncStatus(username: string): Promise<SyncStatus> {
    const res = await fetch(`${API_BASE}/sync/status?username=${encodeURIComponent(username)}`);
    if (!res.ok) {
      throw new Error('Failed to get sync status');
    }
    return res.json();
  },

  async getReviewQueue(userId: number, limit = 20): Promise<PuzzleWithReview[]> {
    const res = await fetch(`${API_BASE}/puzzles/review?user_id=${userId}&limit=${limit}`);
    if (!res.ok) {
      throw new Error('Failed to fetch review queue');
    }
    return res.json();
  },

  async getAllPuzzles(
    userId: number,
    limit = 50,
    offset = 0,
    severity?: string
  ): Promise<PuzzleWithReview[]> {
    let url = `${API_BASE}/puzzles/all?user_id=${userId}&limit=${limit}&offset=${offset}`;
    if (severity) {
      url += `&severity=${encodeURIComponent(severity)}`;
    }
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error('Failed to fetch puzzles');
    }
    return res.json();
  },

  async getPuzzleById(id: number): Promise<PuzzleWithReview> {
    const res = await fetch(`${API_BASE}/puzzles/${id}`);
    if (!res.ok) {
      throw new Error('Puzzle not found');
    }
    return res.json();
  },

  async submitSolve(puzzleId: number, attempt: SolveAttemptRequest): Promise<SolveResponse> {
    const res = await fetch(`${API_BASE}/puzzles/${puzzleId}/solve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(attempt),
    });
    if (!res.ok) {
      throw new Error('Failed to submit solve');
    }
    return res.json();
  },

  async evaluatePosition(fen: string, depth = 14, multiPv = 3): Promise<EngineEvalResponse> {
    const res = await fetch(`${API_BASE}/engine/evaluate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ fen, depth, multi_pv: multiPv }),
    });
    if (!res.ok) {
      throw new Error('Failed to evaluate position');
    }
    return res.json();
  },

  async validateMove(req: ValidateMoveRequest): Promise<ValidateMoveResponse> {
    const res = await fetch(`${API_BASE}/engine/validate_move`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!res.ok) {
      throw new Error('Failed to validate move');
    }
    return res.json();
  },

  async getStats(userId: number): Promise<StatsSummary> {
    const res = await fetch(`${API_BASE}/stats?user_id=${userId}`);
    if (!res.ok) {
      throw new Error('Failed to fetch stats');
    }
    return res.json();
  },
};
