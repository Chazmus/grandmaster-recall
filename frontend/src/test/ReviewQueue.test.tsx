import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ReviewQueue } from '../components/ReviewQueue';
import { PuzzleWithReview, StatsSummary } from '../types';

const mockStats: StatsSummary = {
  total_puzzles: 15,
  due_today: 4,
  mastered_puzzles: 3,
  total_reviews: 20,
  retention_rate: 85.0,
  blunders_count: 5,
  mistakes_count: 6,
  inaccuracies_count: 4,
  tactical_tag_breakdown: [{ tag: 'Fork', count: 5, success_rate: 80.0 }],
  top_blundered_openings: [{ opening_name: 'Sicilian Defense', blunder_count: 3 }],
};

const mockPuzzle: PuzzleWithReview = {
  puzzle: {
    id: 42,
    game_id: 1,
    user_id: 1,
    initial_fen: 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',
    move_number: 14,
    player_color: 'white',
    blunder_move_san: 'Nxe5',
    blunder_move_uci: 'f3e5',
    best_move_san: 'd4',
    best_move_uci: 'd2d4',
    eval_before: 150,
    eval_after_blunder: -180,
    eval_after_best: 150,
    continuation_uci: '["d2d4"]',
    blunder_continuation_uci: '[]',
    tactical_tags: '["Fork", "Missed Tactic"]',
    blunder_severity: 'blunder',
    opening_name: 'Sicilian Defense',
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
    times_solved: 1,
    times_failed: 0,
  },
  game_white: 'testuser',
  game_black: 'Magnus',
  game_time_class: 'rapid',
  game_played_at: new Date().toISOString(),
};

describe('ReviewQueue Component', () => {
  it('renders daily review queue greeting and metrics', () => {
    const onStartReview = vi.fn();
    const onSelectPuzzle = vi.fn();
    const onOpenSync = vi.fn();

    render(
      <ReviewQueue
        duePuzzles={[mockPuzzle]}
        stats={mockStats}
        onStartReview={onStartReview}
        onSelectPuzzle={onSelectPuzzle}
        onOpenSync={onOpenSync}
        username="testuser"
      />
    );

    expect(screen.getByText(/Welcome back,/i)).toBeInTheDocument();
    expect(screen.getByText('testuser')).toBeInTheDocument();
    expect(screen.getByText('85%')).toBeInTheDocument();
    expect(screen.getByText('Train 4 Puzzles')).toBeInTheDocument();
  });

  it('triggers start review callback when clicking button', () => {
    const onStartReview = vi.fn();
    const onSelectPuzzle = vi.fn();
    const onOpenSync = vi.fn();

    render(
      <ReviewQueue
        duePuzzles={[mockPuzzle]}
        stats={mockStats}
        onStartReview={onStartReview}
        onSelectPuzzle={onSelectPuzzle}
        onOpenSync={onOpenSync}
        username="testuser"
      />
    );

    const button = screen.getByText('Train 4 Puzzles');
    fireEvent.click(button);
    expect(onStartReview).toHaveBeenCalledTimes(1);
  });
});
