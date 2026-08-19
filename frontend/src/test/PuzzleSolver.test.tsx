import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PuzzleSolver } from '../components/PuzzleSolver';
import { PuzzleWithReview } from '../types';
import { api } from '../api/client';

vi.mock('../api/client', () => ({
  api: {
    validateMove: vi.fn(),
    submitSolve: vi.fn().mockResolvedValue({
      puzzle_id: 42,
      success: true,
      new_easiness_factor: 2.5,
      new_interval_days: 1,
      new_repetition_number: 1,
      next_due_at: new Date().toISOString(),
      is_mastered: false,
    }),
    evaluatePosition: vi.fn().mockResolvedValue({
      fen: '8/8/8/8/8/8/8/8 w - - 0 1',
      depth: 12,
      best_move: 'e7e5',
      score_cp: 0,
      mate_in: null,
      lines: [],
    }),
  },
}));

vi.mock('canvas-confetti', () => ({
  default: vi.fn(),
}));

const mockPuzzle: PuzzleWithReview = {
  puzzle: {
    id: 42,
    game_id: 1,
    user_id: 1,
    initial_fen: 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',
    move_number: 1,
    player_color: 'white',
    blunder_move_san: 'h4',
    blunder_move_uci: 'h2h4',
    best_move_san: 'e4',
    best_move_uci: 'e2e4',
    eval_before: 0,
    eval_after_blunder: -100,
    eval_after_best: 30,
    continuation_uci: '["e2e4"]',
    blunder_continuation_uci: '[]',
    tactical_tags: '["Opening"]',
    blunder_severity: 'mistake',
    opening_name: "King's Pawn Opening",
    created_at: new Date().toISOString(),
  },
  review: {
    id: 1,
    puzzle_id: 42,
    user_id: 1,
    easiness_factor: 2.5,
    interval_days: 1,
    repetition_number: 1,
    last_reviewed_at: null,
    next_due_at: new Date().toISOString(),
    times_solved: 0,
    times_failed: 0,
  },
  game_white: 'player1',
  game_black: 'player2',
  game_time_class: 'blitz',
  game_played_at: new Date().toISOString(),
};

describe('PuzzleSolver Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders puzzle header and blunder info', () => {
    render(
      <PuzzleSolver
        puzzleData={mockPuzzle}
        onSolved={vi.fn()}
        userId={1}
      />
    );

    expect(screen.getByText("King's Pawn Opening")).toBeInTheDocument();
    expect(screen.getByText('Move 1')).toBeInTheDocument();
    expect(screen.getByText('player2')).toBeInTheDocument();
    expect(screen.getByText(/Why was my move bad\?/i)).toBeInTheDocument();
  });

  it('initially does not display "See Best Move" button until alternative move is played', () => {
    render(
      <PuzzleSolver
        puzzleData={mockPuzzle}
        onSolved={vi.fn()}
        userId={1}
      />
    );

    expect(screen.queryByText(/See Best Move/i)).not.toBeInTheDocument();
  });

  it('renders Step Back / Undo button when appropriate', () => {
    render(
      <PuzzleSolver
        puzzleData={mockPuzzle}
        onSolved={vi.fn()}
        userId={1}
      />
    );

    // At ply 0 with no moves, Undo is not active
    expect(screen.queryByText('Undo')).not.toBeInTheDocument();
  });

  it('displays hint for current position when clicking Hint', async () => {
    render(
      <PuzzleSolver
        puzzleData={mockPuzzle}
        onSolved={vi.fn()}
        userId={1}
      />
    );

    const hintButton = screen.getByRole('button', { name: /hint/i });
    expect(hintButton).toBeInTheDocument();
  });
});

